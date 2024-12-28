use std::{collections::HashMap, env::current_dir, ops::DerefMut, path::PathBuf, sync::Arc};

use anyhow::{anyhow, Context, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_sdk_dynamodb::{
    operation::create_table::CreateTableOutput,
    types::{
        AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput,
        ReturnValue, ScalarAttributeType,
    },
    Client,
};

use axum::{
    http::{Request, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Router,
};
use google_oauth::AsyncClient;
use r2d2_sqlite::SqliteConnectionManager;
use regex::bytes::Regex;
use rusqlite::{params, Connection, OpenFlags};
use tokio::fs;
use tower_http::{
    services::ServeDir,
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{debug, debug_span, error};
use uuid::Uuid;

use crate::{
    middlewares::{Schedule, ScheduleWithId, SelectedCourses, Selection, Session},
    scraper::{Course, Days, MeetingTime, Section, Term, ThinCourse, ThinSection},
};

mod calendar;
mod import;
mod login;
mod preview;
mod root;
mod schedule;
mod search;
mod share;

pub enum SectionType {
    Lecture,
    Lab,
    Tutorial,
}

impl From<String> for SectionType {
    fn from(sequence_code: String) -> Self {
        match sequence_code.get(0..1).unwrap() {
            "A" => SectionType::Lecture,
            "B" => SectionType::Lab,
            "T" => SectionType::Tutorial,
            unparsable_sequence_code => {
                debug!(unparsable_sequence_code);
                unreachable!()
            }
        }
    }
}

pub fn selected_sections(courses: &[Course], selected: &SelectedCourses) -> Vec<Section> {
    let crns = selected.crns();
    courses
        .iter()
        .flat_map(|c| c.sections.clone())
        .filter(|s| crns.contains(&s.crn))
        .collect()
}

pub struct User {
    pub schedules: Vec<ScheduleWithId>,
}

trait UserStore {
    async fn get_user(&self, user_id: &str) -> Result<User>;
    async fn get_user_schedule(&self, user_id: &str, schedule_id: &str) -> Result<Schedule>;
    async fn set_user_schedule(
        &self,
        user_id: &str,
        schedule_id: &str,
        schedule: &Schedule,
    ) -> Result<Schedule>;
    async fn delete_user_schedule(&self, user_id: &str, schedule_id: &str);
    async fn make_session(&self, user_id: &str, session_id: &str);
    async fn get_session(&self, user_id: &str, session_id: &str) -> Option<Session>;
}

#[derive(Clone)]
struct DynamoUserStore {
    ddb_client: aws_sdk_dynamodb::Client,
}

impl DynamoUserStore {
    pub fn new(ddb_client: Client) -> DynamoUserStore {
        Self { ddb_client }
    }
}

impl UserStore for DynamoUserStore {
    async fn get_user(&self, user_id: &str) -> Result<User> {
        debug!("get_user called");
        let results = self
            .ddb_client
            .query()
            .table_name("schedules")
            .key_condition_expression("#uid = :user_id")
            .expression_attribute_names("#uid", "userId")
            .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
            .send()
            .await?;

        Ok(User {
            schedules: results
                .items()
                .iter()
                .filter_map(|v| {
                    let schedule: Schedule = v.try_into().ok()?;
                    let schedule_id = v.get("scheduleId")?.as_s().ok()?;
                    Some(ScheduleWithId {
                        id: schedule_id.into(),
                        schedule,
                    })
                })
                .collect(),
        })
    }

    async fn get_user_schedule(&self, user_id: &str, schedule_id: &str) -> Result<Schedule> {
        debug!("get_user_schedule called");
        let results = self
            .ddb_client
            .query()
            .table_name("schedules")
            .key_condition_expression("#uid = :user_id AND scheduleId = :schedule_id")
            .expression_attribute_names("#uid", "userId")
            .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
            .expression_attribute_values(":schedule_id", AttributeValue::S(schedule_id.to_string()))
            .send()
            .await?;
        let first = results
            .items()
            .first()
            .ok_or(anyhow!("get_user_schedule produced no results"))?;
        first.try_into()
    }

    async fn set_user_schedule(
        &self,
        user_id: &str,
        schedule_id: &str,
        schedule: &Schedule,
    ) -> Result<Schedule> {
        let user_id_av = AttributeValue::S(user_id.to_string());
        let schedule_id_av = AttributeValue::S(schedule_id.to_string());
        let schedule_av = AttributeValue::S(serde_json::to_string(&schedule).unwrap());

        let request = self
            .ddb_client
            .update_item()
            .table_name("schedules")
            .key("userId", user_id_av)
            .key("scheduleId", schedule_id_av)
            .update_expression("SET schedule = :schedule")
            .expression_attribute_values(":schedule", schedule_av)
            .return_values(ReturnValue::UpdatedNew);

        let resp = request.send().await?;
        let attrs = resp.attributes().ok_or(anyhow!(
            "SetItem response had no attribute map for user_id={}, schedule_id={}",
            user_id,
            schedule_id
        ))?;
        let schedule_av = attrs.get("schedule").ok_or(anyhow!(
            "SetItem response had no schedule value for user_id={}, schedule_id={}",
            user_id,
            schedule_id
        ))?;
        let s = schedule_av
            .as_s()
            .map_err(|_e| anyhow!("could not get attribute value as s"))?;
        serde_json::from_str(s).map_err(|e| anyhow!("failed to deserialize schedule, {e}"))
    }

    async fn delete_user_schedule(&self, user_id: &str, schedule_id: &str) {
        match self
            .ddb_client
            .delete_item()
            .table_name("schedules")
            .key("user_id", AttributeValue::S(user_id.to_string()))
            .key("scheduleId", AttributeValue::S(schedule_id.to_string()))
            .send()
            .await
        {
            Ok(_out) => debug!("deleted schedule {}:{}", user_id, schedule_id),
            Err(err) => error!(
                "failed to delete schedule {}:{}, {}",
                user_id, schedule_id, err
            ),
        }
    }

    async fn make_session(&self, user_id: &str, session_id: &str) {
        match self
            .ddb_client
            .put_item()
            .table_name("sessions")
            .item("userId", AttributeValue::S(user_id.to_string()))
            .item("sessionId", AttributeValue::S(session_id.to_string()))
            // .item("ttl", ) // TODO: TTL
            .send()
            .await
        {
            Ok(_r) => debug!("created session {}:{}", user_id, session_id),
            Err(err) => error!(
                "failed to make session {}:{}, {}",
                user_id,
                session_id,
                err.to_string()
            ),
        }
    }

    async fn get_session(&self, user_id: &str, session_id: &str) -> Option<Session> {
        debug!("getting session {}:{}", user_id, session_id);
        let results = self
            .ddb_client
            .query()
            .table_name("sessions")
            .key_condition_expression("#uid = :user_id AND sessionId = :sessionId")
            .expression_attribute_names("#uid", "userId")
            .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
            .expression_attribute_values(":sessionId", AttributeValue::S(session_id.to_string()))
            .send()
            .await
            .unwrap();

        let items = results.items?;
        let result = items.first()?;

        match result.get("userId")?.as_s() {
            Ok(uid) => {
                debug!("retrieved session for {}", uid);
                Some(Session {
                    user_id: uid.to_string(),
                    session_id: session_id.to_owned(),
                })
            }
            Err(_) => None,
        }
    }
}

#[derive(Clone)]
pub struct DatabaseAppState {
    terms: HashMap<Term, r2d2::Pool<SqliteConnectionManager>>,
    user_store: DynamoUserStore,
    google_client: AsyncClient,
}

impl DatabaseAppState {
    pub async fn new(dir: PathBuf) -> Result<Self> {
        let region = RegionProviderChain::default_provider().or_else("us-east-1");
        let ddb_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url("http://localhost:8000")
            .load()
            .await;
        let ddb_client = aws_sdk_dynamodb::Client::new(&ddb_config);

        let table_list = ddb_client.list_tables().send().await.unwrap();
        if !table_list.table_names().contains(&"schedules".to_string()) {
            let _ =
                DatabaseAppState::create_table(&ddb_client, "schedules", "userId", "scheduleId")
                    .await
                    .map_err(|_e| panic!());
        }
        if !table_list.table_names().contains(&"sessions".to_string()) {
            let _ = DatabaseAppState::create_table(&ddb_client, "sessions", "userId", "sessionId")
                .await
                .map_err(|_e| panic!());
        }

        let user_store = DynamoUserStore::new(ddb_client);

        let mut terms = HashMap::new();

        let mut entries = fs::read_dir(dir).await?;

        let pattern = Regex::new(r"^sections_([0-9]{6})\.sqlite3$")?;

        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let Some(captures) = pattern.captures(file_name.as_encoded_bytes()) else {
                continue;
            };
            let Some(term) = captures.get(1) else {
                continue;
            };

            let term: Term = std::str::from_utf8(term.as_bytes())
                .expect("term numbers should be ascii")
                .parse()?;

            let manager = SqliteConnectionManager::file(file_name)
                .with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY);
            let pool = r2d2::Pool::new(manager)?;

            terms.insert(term, pool);
        }

        // not a secret
        let google_client = AsyncClient::new(
            "839626045148-u695skik1hvq9o41dactp72usr0i9bsh.apps.googleusercontent.com",
        );

        Ok(Self {
            terms,
            user_store,
            google_client,
        })
    }

    pub fn get_terms(&self) -> Vec<Term> {
        let mut terms: Vec<_> = self.terms.keys().cloned().collect();
        terms.sort();
        terms.reverse();

        terms
    }

    fn courses(&self, term: Term, keys: &[&ThinCourse]) -> Result<Vec<Course>> {
        let Some(conn) = self.get_conn(&term) else {
            return Ok(Vec::new());
        };

        let courses = keys.iter().map(|&ThinCourse {subject_code, course_code}| {
            conn.query_row_and_then("
                    SELECT title, campus
                    FROM course
                    WHERE subject_code = ?1 AND course_code = ?2
                ",
                (subject_code, course_code), |row| {
                    // no N+1 problem when it's in memory
                    let sections = conn.prepare("
                            SELECT crn, sequence_code, enrollment, enrollment_capacity, waitlist, waitlist_capacity
                            FROM section
                            WHERE subject_code = ?1 AND course_code = ?2
                        ")?
                        .query_and_then((subject_code, course_code), |row| {
                            let crn = row.get("crn")?;

                            let meeting_times = conn.prepare("
                                SELECT
                                    start_time, end_time, start_date, end_date,
                                    monday, tuesday, wednesday, thursday, friday, saturday, sunday,
                                    building, room
                                FROM meeting_time
                                WHERE crn = ?1
                                ")?
                                .query_and_then((crn,), |row| {
                                    let start_time: Option<String> = row.get("start_time")?;
                                    let end_time: Option<String> = row.get("end_time")?;

                                    let start_date: String = row.get("start_date")?;
                                    let end_date: String = row.get("end_date")?;

                                    Ok(MeetingTime {
                                        start_time: start_time.map(|s| s.parse()).transpose()?,
                                        end_time: end_time.map(|s| s.parse()).transpose()?,
                                        start_date: start_date.parse()?,
                                        end_date: end_date.parse()?,

                                        days: Days {
                                            monday: row.get("monday")?,
                                            tuesday: row.get("tuesday")?,
                                            wednesday: row.get("wednesday")?,
                                            thursday: row.get("thursday")?,
                                            friday: row.get("friday")?,
                                            saturday: row.get("saturday")?,
                                            sunday: row.get("sunday")?,
                                        },

                                        building: row.get("building")?,
                                        room: row.get("room")?,
                                    })
                                })?.collect::<Result<Vec<_>>>()?;

                            Ok(Section {
                                crn,
                                subject_code: subject_code.clone(),
                                course_code: course_code.clone(),
                                sequence_code: row.get("sequence_code")?,
                                enrollment: row.get("enrollment")?,
                                enrollment_capacity: row.get("enrollment_capacity")?,
                                waitlist: row.get("waitlist")?,
                                waitlist_capacity: row.get("waitlist_capacity")?,
                                meeting_times,
                            })
                    })?.collect::<Result<Vec<_>>>()?;

                    Ok(Course {
                       subject_code: subject_code.clone(),
                       course_code: course_code.clone(),
                       title: row.get("title")?,
                       campus: row.get("campus")?,
                       sections,
                    })
                })
            }).collect::<Result<Vec<_>>>()?;

        Ok(courses)
    }

    fn thin_courses(&self, term: Term) -> Result<Vec<ThinCourse>> {
        let Some(conn) = self.get_conn(&term) else {
            return Ok(Vec::new());
        };

        let courses = conn
            .prepare("SELECT subject_code, course_code FROM course")
            .context("failed to prepare courses SQL statement")?
            .query_and_then((), |row| {
                let subject_code = row.get("subject_code")?;
                let course_code = row.get("course_code")?;
                Ok(ThinCourse {
                    subject_code,
                    course_code,
                })
            })
            .context("failed to execute query")?
            .collect::<Result<Vec<_>>>() // quit on the first error
            .context("failed to get query rows")?;

        Ok(courses)
    }

    fn search(&self, term: Term, query: &str) -> Result<Vec<ThinCourse>> {
        let db = self
            .get_conn(&term)
            .context("failed to get conn from pool")?;
        let courses = db
            .prepare(
                "SELECT subject_code, course_code
                FROM course
                WHERE subject_code || course_code LIKE '%' || ?1 || '%'
                   OR subject_code || ' ' || course_code LIKE '%' || ?2 || '%'",
            )?
            .query_and_then((query, query), |row| {
                let subject_code = row.get("subject_code")?;
                let course_code = row.get("course_code")?;
                Ok(ThinCourse {
                    subject_code,
                    course_code,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(courses)
    }

    fn default_thin_sections(&self, term: &Term, course: ThinCourse) -> Result<Selection> {
        let conn = self
            .get_conn(term)
            .context("failed to get conn from pool")?;

        let sections = conn.prepare(
            "SELECT sequence_code, crn FROM section WHERE subject_code = ?1 AND course_code = ?2 ORDER BY sequence_code",
        )
        .context("failed to prepare courses SQL statement")?
        .query_and_then((&course.subject_code, &course.course_code), |row| {
            let sequence_code: String = row.get(0)?;
            let crn: u64 = row.get(1)?;
            Ok((sequence_code, ThinSection { crn }))
        })
        .context("query failed")?
        .collect::<anyhow::Result<Vec<_>>>()?;

        let lecture = sections
            .iter()
            .filter(|s| s.0.starts_with("A"))
            .map(|(_, ts)| ts.clone())
            .collect::<Vec<_>>()
            .first()
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "Expected to find lecture section for course {} {}",
                    &course.subject_code, &course.course_code
                )
            });

        let lab = sections
            .iter()
            .filter(|s| s.0.starts_with("B"))
            .map(|(_, ts)| ts.clone())
            .collect::<Vec<_>>()
            .first()
            .cloned();

        let tutorial = sections
            .iter()
            .filter(|s| s.0.starts_with("T"))
            .map(|(_, ts)| ts.clone())
            .collect::<Vec<_>>()
            .first()
            .cloned();

        Ok(Selection {
            lecture,
            lab,
            tutorial,
        })
    }

    fn get_section(&self, term: &Term, section: &ThinSection) -> Result<Section> {
        let db = self
            .get_conn(term)
            .context("failed to get conn from pool")?;

        let meeting_times = db
            .prepare(
                "
            SELECT
                start_time, end_time, start_date, end_date,
                monday, tuesday, wednesday, thursday, friday, saturday, sunday,
                building, room
            FROM meeting_time
            WHERE crn = ?1
            ",
            )?
            .query_and_then((section.crn,), |row| {
                let start_time: Option<String> = row.get("start_time")?;
                let end_time: Option<String> = row.get("end_time")?;

                let start_date: String = row.get("start_date")?;
                let end_date: String = row.get("end_date")?;

                Ok(MeetingTime {
                    start_time: start_time.map(|s| s.parse()).transpose()?,
                    end_time: end_time.map(|s| s.parse()).transpose()?,
                    start_date: start_date.parse()?,
                    end_date: end_date.parse()?,

                    days: Days {
                        monday: row.get("monday")?,
                        tuesday: row.get("tuesday")?,
                        wednesday: row.get("wednesday")?,
                        thursday: row.get("thursday")?,
                        friday: row.get("friday")?,
                        saturday: row.get("saturday")?,
                        sunday: row.get("sunday")?,
                    },

                    building: row.get("building")?,
                    room: row.get("room")?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        let result = db
            .prepare(
                "SELECT *
                FROM section
                WHERE crn = :crn
                LIMIT 1",
            )?
            .query_row(params![section.crn], |row| {
                let crn: u64 = row.get("crn")?;
                let subject_code = row.get("subject_code")?;
                let course_code = row.get("course_code")?;
                let sequence_code = row.get("sequence_code")?;
                let enrollment = row.get("enrollment")?;
                let enrollment_capacity = row.get("enrollment_capacity")?;
                let waitlist = row.get("waitlist")?;
                let waitlist_capacity = row.get("waitlist_capacity")?;
                Result::Ok(Section {
                    crn,
                    subject_code,
                    course_code,
                    sequence_code,
                    enrollment,
                    enrollment_capacity,
                    waitlist,
                    waitlist_capacity,
                    meeting_times,
                })
            })?;
        Ok(result)
    }

    pub fn get_conn(&self, term: &Term) -> Option<impl DerefMut<Target = Connection>> {
        self.terms.get(term).and_then(|p| p.get().ok())
    }

    pub async fn get_user(&self, user_id: &str) -> Result<User> {
        self.user_store.get_user(user_id).await
    }

    pub async fn get_user_schedule(&self, user_id: &str, schedule_id: &str) -> Result<Schedule> {
        self.user_store
            .get_user_schedule(user_id, schedule_id)
            .await
    }

    pub async fn set_user_schedule(
        &self,
        user_id: &str,
        schedule_id: &str,
        schedule: &Schedule,
    ) -> Result<Schedule> {
        self.user_store
            .set_user_schedule(user_id, schedule_id, schedule)
            .await
    }

    pub async fn delete_user_schedule(&self, user_id: &str, schedule_id: &str) {
        self.user_store
            .delete_user_schedule(user_id, schedule_id)
            .await;
    }

    pub async fn make_session(&self, user_id: &str) -> Session {
        let session_id = Uuid::new_v4();
        self.user_store
            .make_session(user_id, &session_id.to_string())
            .await;
        Session {
            user_id: user_id.to_string(),
            session_id: session_id.to_string(),
        }
    }

    pub async fn is_valid_session(&self, user_id: &str, session_id: &str) -> bool {
        match self.user_store.get_session(user_id, session_id).await {
            Some(session) => session.user_id == user_id,
            None => false,
        }
    }

    pub async fn create_table(
        client: &Client,
        table: &str,
        primary_key: &str,
        sort_key: &str,
    ) -> Result<CreateTableOutput> {
        let pk_name: String = primary_key.into();
        let sk_name: String = sort_key.into();
        let table_name: String = table.into();

        let pk_ad = AttributeDefinition::builder()
            .attribute_name(&pk_name)
            .attribute_type(ScalarAttributeType::S)
            .build()
            .context(format!(
                "failed to build primary key attribute definition for pk={}",
                primary_key
            ))?;

        let sk_ad = AttributeDefinition::builder()
            .attribute_name(&sk_name)
            .attribute_type(ScalarAttributeType::S)
            .build()
            .context(format!(
                "failed to build sort key attribute definition for sk={}",
                sort_key
            ))?;

        let pk_ks = KeySchemaElement::builder()
            .attribute_name(&pk_name)
            .key_type(KeyType::Hash)
            .build()
            .context(format!(
                "failed to build primary key key schema element for pk={}",
                primary_key
            ))?;

        let sk_ks = KeySchemaElement::builder()
            .attribute_name(&sk_name)
            .key_type(KeyType::Range)
            .build()
            .context(format!(
                "failed to build sort key key schema element for sk={}",
                sort_key
            ))?;

        let pt = ProvisionedThroughput::builder()
            .read_capacity_units(10)
            .write_capacity_units(5)
            .build()?;

        let create_table_response = client
            .create_table()
            .table_name(table_name)
            .key_schema(pk_ks)
            .key_schema(sk_ks)
            .attribute_definitions(pk_ad)
            .attribute_definitions(sk_ad)
            .provisioned_throughput(pt)
            .send()
            .await?;

        Ok(create_table_response)
    }
}

pub enum AppError {
    Anyhow(anyhow::Error),
    Code(StatusCode),
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Anyhow(error) => {
                let backtrace = error.backtrace();
                error!(%error, %backtrace);
                AppError::Code(StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
            AppError::Code(c) => (c, c.to_string()).into_response(),
        }
    }
}

impl From<StatusCode> for AppError {
    fn from(err: StatusCode) -> Self {
        Self::Code(err)
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Anyhow(err)
    }
}

pub async fn make_app() -> Router {
    type State = Arc<DatabaseAppState>;

    let state: State = Arc::new(
        DatabaseAppState::new(current_dir().expect("couldn't access current directory"))
            .await
            .expect("failed to initialize database state"),
    );

    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        // `GET /` goes to `root`
        .route("/", get(root::root))
        .nest(
            "/login",
            Router::new()
                .route("/", get(login::get))
                .route("/unsafe", post(login::post_unsafe)),
        )
        .route("/share/:schedule_id", get(share::get))
        .route("/import", get(import::get))
        .route("/schedule", post(schedule::post))
        .nest(
            "/schedule/:schedule_id",
            Router::new()
                .route("/", get(schedule::get))
                .route("/", delete(schedule::delete))
                .route("/search", post(search::search))
                .nest(
                    "/calendar",
                    Router::new()
                        .route("/", get(calendar::get_calendar))
                        .route("/", put(calendar::add_to_calendar))
                        .route("/", patch(calendar::update_calendar))
                        .route("/", delete(calendar::rm_from_calendar))
                        .route("/preview", get(preview::preview)),
                )
                .layer(middleware::from_fn(schedule::not_found)),
        )
        // TODO: add .fallback() handler
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    debug_span!(
                        "request",
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_response(DefaultOnResponse::new().latency_unit(LatencyUnit::Micros)),
        )
}
