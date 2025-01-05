use std::{collections::HashMap, env::{self, current_dir}, ops::DerefMut, path::PathBuf, sync::Arc};

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
    extract::Request, http::StatusCode, middleware::{self, Next}, response::{IntoResponse, Response}, routing::{delete, get, patch, post, put}, Router
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use google_oauth::AsyncClient;
use r2d2_sqlite::SqliteConnectionManager;
use regex::bytes::Regex;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tower_http::{
    services::ServeDir,
    trace::{DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{debug, debug_span, error};
use uuid::Uuid;

use crate::{
    middlewares::{Schedule, ScheduleWithId, SelectedCourses, Selection},
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

pub trait UserStore: Clone {
    fn get_user(&self, user_id: &str) -> impl std::future::Future<Output = Result<User>> + Send;
    fn get_user_schedule(&self, user_id: &str, schedule_id: &str) -> impl std::future::Future<Output = Result<Schedule>> + Send;
    fn set_user_schedule(
        &self,
        user_id: &str,
        schedule_id: &str,
        schedule: &Schedule,
    ) -> impl std::future::Future<Output = Result<Schedule>> + Send;
    fn delete_user_schedule(&self, user_id: &str, schedule_id: &str) -> impl std::future::Future<Output = ()> + Send;
    fn make_session(&self, user_id: &str, session_id: &str) -> impl std::future::Future<Output = ()> + Send;
    fn has_session(&self, user_id: &str, session_id: &str) -> impl std::future::Future<Output = Result<bool>> + Send;
}

#[derive(Clone)]
pub struct DynamoUserStore {
    ddb_client: aws_sdk_dynamodb::Client,
    sessions_table_name: String,
    schedules_table_name: String,
}

impl DynamoUserStore {
    pub fn new(ddb_client: Client, sessions_table_name: &str, schedules_table_name: &str) -> DynamoUserStore {
        Self { 
            ddb_client,
            sessions_table_name: sessions_table_name.to_string(),
            schedules_table_name: schedules_table_name.to_string(),
         }
    }
}

impl UserStore for DynamoUserStore {
    async fn get_user(&self, user_id: &str) -> Result<User> {
        debug!("get_user called");
        let results = self
            .ddb_client
            .query()
            .table_name(&self.schedules_table_name)
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
        // TODO convert to GetItem request?
        let results = self
            .ddb_client
            .query()
            .table_name(&self.schedules_table_name)
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
            .table_name(&self.schedules_table_name)
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
            .table_name(&self.schedules_table_name)
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
            .table_name(&self.sessions_table_name)
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

    async fn has_session(&self, user_id: &str, session_id: &str) -> Result<bool> {
        debug!("getting session {}:{}", user_id, session_id);
        let results = self
            .ddb_client
            .query()
            .table_name(&self.sessions_table_name)
            .key_condition_expression("#uid = :user_id AND sessionId = :sessionId")
            .expression_attribute_names("#uid", "userId")
            .expression_attribute_values(":user_id", AttributeValue::S(user_id.to_string()))
            .expression_attribute_values(":sessionId", AttributeValue::S(session_id.to_string()))
            .send()
            .await
            .unwrap();

        let items = results.items.ok_or(anyhow!("could not get items from ddb response"))?;
        Ok(!items.is_empty())
    }
}

#[derive(Clone, Serialize)]
struct DiscordTokenRequestBody {
    grant_type: String,
    code: String,
    redirect_uri: String,
}

#[derive(Clone, Deserialize)]
struct DiscordTokenResponseBody {
    access_token: String,
}

#[derive(Clone, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
}

#[derive(Clone)]
struct DiscordClient {
    http_client: reqwest::Client,
    redirect_uri: String,
    client_id: String,
    client_secret: String,
}

impl DiscordClient {
    pub async fn get_user(&self, code: &str) -> Result<DiscordUser> {
        let body = DiscordTokenRequestBody {
            grant_type: "authorization_code".to_owned(),
            code: code.to_string(),
            redirect_uri: self.redirect_uri.clone(),
        };
        debug!("{}", serde_json::to_string_pretty(&body)?);
        debug!("requesting data for code={}", body.code);
        let response = self.http_client.post("https://discord.com/api/oauth2/token")
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&body)
            .send()
            .await?
            .text()
            .await?;
        debug!("response text={}", response);

        let response = serde_json::from_str::<DiscordTokenResponseBody>(&response)
            .map_err(|e| anyhow!("failed to deserialize token response body, error={}", e))?;

        debug!("got auth token={}", response.access_token);

        let user = self.http_client.get("https://discord.com/api/users/@me")
            .bearer_auth(response.access_token)
            .send()
            .await?
            .json::<DiscordUser>()
            .await?;

        Ok(user)
    }
}

#[derive(Clone)]
pub struct DatabaseAppState
{
    terms: HashMap<Term, r2d2::Pool<SqliteConnectionManager>>,
    user_store: DynamoUserStore,
    google_client: AsyncClient,
    discord_client: DiscordClient,
    stage: Stage,
}

impl DatabaseAppState {
    pub async fn new(dir: PathBuf, stage: Stage, user_store: DynamoUserStore, discord_secret: &str) -> Result<Self> {
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

        let discord_redirect_uri = match stage {
            Stage::LOCAL => "http://localhost:8443/login/discord".to_string(),
            Stage::PROD => "https://scheduler.brennanmcmicking.net/login/discord".to_string(),
        };
        let discord_client = DiscordClient {
            http_client: reqwest::Client::new(),
            redirect_uri: discord_redirect_uri,
            client_id: "1324110828810797108".to_string(),
            client_secret: discord_secret.to_string(),
        };

        Ok(Self {
            terms,
            user_store,
            google_client,
            discord_client,
            stage,
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

    pub async fn make_session(&self, user_id: &str) -> String {
        let session_id = Uuid::new_v4().to_string();
        self.user_store
            .make_session(user_id, &session_id)
            .await;
        session_id
    }

    pub async fn is_valid_session(&self, user_id: &str, session_id: &str) -> bool {
        self.user_store.has_session(user_id, session_id).await.unwrap_or(false)
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

#[derive(Clone)]
pub enum Stage {
    LOCAL,
    PROD
}

pub async fn make_app(stage: Stage, use_local_dynamo: bool) -> Router {
    let region = RegionProviderChain::default_provider().or_else("us-east-1");
    let ddb_config = match use_local_dynamo {
        false => aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .load()
            .await,
        true => aws_config::defaults(BehaviorVersion::latest())
            .region(region)
            .endpoint_url("http://localhost:8000")
            .test_credentials()
            .load()
            .await
    };
    let schedules_table_name = match stage {
        Stage::PROD => "schedules".to_string(),
        Stage::LOCAL => "schedules-dev".to_string(),
    };
    let sessions_table_name = match stage {
        Stage::PROD => "sessions".to_string(),
        Stage::LOCAL => "sessions-dev".to_string(),
    };

    let ddb_client = aws_sdk_dynamodb::Client::new(&ddb_config);

    let table_list = ddb_client.list_tables().send().await.unwrap();
    if !table_list.table_names().contains(&schedules_table_name) {
        let _ =
            DatabaseAppState::create_table(&ddb_client, &schedules_table_name, "userId", "scheduleId")
                .await
                .map_err(|_e| panic!());
    }
    if !table_list.table_names().contains(&sessions_table_name) {
        let _ = DatabaseAppState::create_table(&ddb_client, &sessions_table_name, "userId", "sessionId")
            .await
            .map_err(|_e| panic!());
    }

    let user_store = DynamoUserStore::new(ddb_client, &sessions_table_name, &schedules_table_name);

    type State = Arc<DatabaseAppState>;

    let discord_secret = env::var("DISCORD_SECRET").unwrap_or("".to_string());

    let state: State = Arc::new(
        DatabaseAppState::new(current_dir().expect("couldn't access current directory"), stage, user_store, &discord_secret)
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
                .route("/google", post(login::post_google))
                .route("/discord", get(login::get_discord)),
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
                .layer(middleware::from_fn_with_state(state.clone(), schedule::not_found)),
        )
        .with_state(state)
        .layer(middleware::from_fn(unauth_redirect))
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

async fn unauth_redirect(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, AppError> {
    let res = next.run(req).await;
    if res.status() == StatusCode::UNAUTHORIZED {
        // if there was an unauthorized response then delete the session cookie and redirect to the login page
        let cookie = Cookie::build(("session", "")).removal().build();
        return Ok((
            CookieJar::new().add(cookie),
            [("location", "/")],
            StatusCode::MOVED_PERMANENTLY,
        ).into_response());
    }

    Ok(res)
}
