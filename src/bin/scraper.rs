use anyhow::{Ok, Result};
use clap::Parser;
use jiff::Zoned;
use rusqlite::Connection;
use std::path::Path;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use scheduler::scraper::*;

#[derive(Parser)]
/// Downloads section info to SQLite databases in the current folder.
///
/// By default, it will only download past terms once, and will always redownload current or future
/// terms
struct Args {
    /// Term to scrape from. If missing, scrape all terms.
    ///
    /// Format: YYYYMM
    term: Option<Term>,

    /// Force download term, even if we already have an up-to-date copy
    #[arg(long, short, default_value_t = false)]
    force: bool,

    /// Oldest term to possibly fetch, refusing any older terms. This is overridden by the
    /// positional TERM argument if present
    #[arg(long, short, value_name = "TERM")]
    oldest: Option<Term>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "scraper=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    let terms = if let Some(term) = args.term {
        vec![term]
    } else {
        info!("fetching list of all terms");
        let mut terms = scrape::fetch_terms().await?;
        if let Some(oldest) = args.oldest {
            terms.retain(|t| t >= &oldest)
        }
        terms
    };

    let now = Zoned::now();

    // no point in parallelizing, UVic's server is the bottleneck
    for &term in terms.iter() {
        let filename = format!("sections_{}.sqlite3", term);

        if !args.force && term < now && Path::new(&filename).exists() {
            info!("db already downloaded for {}", term);
            continue;
        }
        info!("fetching sections for term {}", term);

        let courses = scrape::fetch_sections(term).await?;
        persist(filename, &courses)?;
    }

    Ok(())
}

fn persist<P: AsRef<Path>>(filename: P, courses: &Vec<Course>) -> Result<()> {
    let conn = Connection::open_in_memory()?;

    store_sections(&conn, courses)?;

    conn.backup(rusqlite::DatabaseName::Main, filename, None)?;

    Ok(())
}

/// Store sessions to database `conn`, creating tables and writing rows. Writing to a non-empty
/// database will likely produce an error.
pub fn store_sections(conn: &Connection, courses: &Vec<Course>) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;

        CREATE TABLE course (
            subject_code TEXT NOT NULL,
            course_code TEXT NOT NULL,

            title TEXT NOT NULL,
            campus TEXT NOT NULL,

            PRIMARY KEY (subject_code, course_code)
        ) STRICT;

        CREATE TABLE section (
            crn INTEGER NOT NULL PRIMARY KEY ,

            subject_code TEXT NOT NULL,
            course_code TEXT NOT NULL,
            sequence_code TEXT NOT NULL,

            enrollment INTEGER NOT NULL,
            enrollment_capacity INTEGER NOT NULL,
            waitlist INTEGER NOT NULL,
            waitlist_capacity INTEGER NOT NULL,

            FOREIGN KEY (subject_code, course_code) REFERENCES course(subject_code, course_code)
        ) STRICT;

        CREATE TABLE meeting_time (
            crn INTEGER NOT NULL,

            start_time TEXT,
            end_time TEXT,
            start_date TEXT NOT NULL,
            end_date TEXT NOT NULL,

            monday INTEGER NOT NULL,
            tuesday INTEGER NOT NULL,
            wednesday INTEGER NOT NULL,
            thursday INTEGER NOT NULL,
            friday INTEGER NOT NULL,
            saturday INTEGER NOT NULL,
            sunday INTEGER NOT NULL,

            building TEXT,
            room TEXT,

            FOREIGN KEY (crn) REFERENCES section(crn)
        ) STRICT;
        CREATE INDEX meeting_time_crn ON meeting_time(crn);
        ",
    )?;

    for course in courses {
        conn.execute(
            "INSERT INTO course (
                subject_code, course_code, title, campus
            ) VALUES (?1, ?2, ?3, ?4)",
            (
                &course.subject_code,
                &course.course_code,
                &course.title,
                &course.campus,
            ),
        )?;

        for section in &course.sections {
            conn.execute(
                "INSERT INTO section (
                    crn, subject_code, course_code, sequence_code, 
                    enrollment, enrollment_capacity, waitlist, waitlist_capacity
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
                (
                    section.crn,
                    &course.subject_code,
                    &course.course_code,
                    &section.sequence_code,
                    section.enrollment,
                    section.enrollment_capacity,
                    section.waitlist,
                    section.waitlist_capacity,
                ),
            )?;
            for meeting_time in &section.meeting_times {
                conn.execute(
                    "INSERT INTO meeting_time (
                        crn, start_time, end_time, start_date, end_date, monday,
                        tuesday, wednesday, thursday, friday, saturday, sunday,
                        building, room
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14);",
                    (
                        section.crn,
                        meeting_time.start_time.map(|t| t.to_string()),
                        meeting_time.end_time.map(|t| t.to_string()),
                        meeting_time.start_date.to_string(),
                        meeting_time.end_date.to_string(),
                        meeting_time.days.monday,
                        meeting_time.days.tuesday,
                        meeting_time.days.wednesday,
                        meeting_time.days.thursday,
                        meeting_time.days.friday,
                        meeting_time.days.saturday,
                        meeting_time.days.sunday,
                        &meeting_time.building,
                        &meeting_time.room,
                    ),
                )?;
            }
        }
    }

    Ok(())
}

mod scrape {
    use std::collections::HashMap;

    use anyhow::{anyhow, bail, Context, Ok, Result};
    use jiff::civil::{Date, Time};
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use tracing::{debug, instrument};

    use super::Term;

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct SectionResults {
        page_max_size: u32,
        total_count: u32,
        data: Vec<Section>,
    }

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Section {
        id: u32,
        course_reference_number: String,
        subject: String,
        course_number: String,
        sequence_number: String,
        subject_description: String,
        course_title: String,
        campus_description: String,
        schedule_type_description: String,
        enrollment: u32,
        maximum_enrollment: u32,
        wait_count: u32,
        // NOTE: wait_capacity=None always implies wait_count=0
        wait_capacity: Option<u32>,
        meetings_faculty: Vec<MeetingsFaculty>,
    }

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct MeetingsFaculty {
        meeting_time: MeetingTime,
    }

    #[derive(Deserialize, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct MeetingTime {
        begin_time: Option<String>,
        end_time: Option<String>,
        start_date: String,
        end_date: String,
        monday: bool,
        tuesday: bool,
        wednesday: bool,
        thursday: bool,
        friday: bool,
        saturday: bool,
        sunday: bool,
        meeting_type: String,
        meeting_type_description: String,
        // NOTE: building and room require auth, anonymized otherwise
        building: Option<String>,
        room: Option<String>,
    }

    impl TryFrom<Section> for super::Section {
        type Error = anyhow::Error;

        fn try_from(s: Section) -> Result<Self> {
            Ok(super::Section {
                crn: s.course_reference_number.parse()?,
                subject_code: s.subject,
                course_code: s.course_number,
                sequence_code: s.sequence_number,
                enrollment: s.enrollment,
                enrollment_capacity: s.maximum_enrollment,
                waitlist: s.wait_count,
                waitlist_capacity: s.wait_capacity.unwrap_or(0),
                meeting_times: s
                    .meetings_faculty
                    .into_iter()
                    .map(|m| {
                        // time format is so bad, not even strptime can parse it
                        fn time_from_string(s: String) -> Result<Time> {
                            if s.len() != 4 {
                                bail!("time must be of format \"HHMM\"");
                            }
                            let (hours, minutes) = s.split_at(2);
                            let hours = hours.parse()?;
                            let minutes = minutes.parse()?;
                            Ok(Time::new(hours, minutes, 0, 0)?)
                        }
                        let m = m.meeting_time;
                        Ok(super::MeetingTime {
                            start_time: m.begin_time.map(time_from_string).transpose()?,
                            end_time: m.end_time.map(time_from_string).transpose()?,
                            start_date: Date::strptime("%b %d, %Y", m.start_date)?,
                            end_date: Date::strptime("%b %d, %Y", m.end_date)?,
                            days: super::Days {
                                monday: m.monday,
                                tuesday: m.tuesday,
                                wednesday: m.wednesday,
                                thursday: m.thursday,
                                friday: m.friday,
                                saturday: m.saturday,
                                sunday: m.sunday,
                            },
                            building: m.building,
                            room: m.room.and_then(|r| {
                                if r == "None specified" {
                                    None
                                } else {
                                    Some(r)
                                }
                            }),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            })
        }
    }

    const URL_PREFIX: &str = "https://banner.uvic.ca/StudentRegistrationSsb/ssb";

    #[instrument()]
    pub async fn fetch_sections(term: Term) -> Result<Vec<super::Course>> {
        let client = Client::builder().cookie_store(true).build()?;

        debug!("fetching auth cookie");
        // setup the good cookies
        client
            .get(format!(
                "{}/classSearch/classSearch?term={}&txt_subject=CSUP&txt_courseNumber=000",
                URL_PREFIX, term
            ))
            .send()
            .await?;

        debug!("fetching first sections");
        let res = fetch_sections_partial(client.clone(), term, 0).await?;

        let mut sections = res.data;

        let sections_num = u32::try_from(sections.len())?;
        let sections_left = res.total_count - sections_num;
        // ceil division
        let requests_left = sections_left.div_ceil(res.page_max_size);

        let handles = (0..requests_left).map(|i| {
            fetch_sections_partial(client.clone(), term, sections_num + i * res.page_max_size)
        });

        for res in futures::future::join_all(handles).await {
            let res = res?;
            sections.extend(res.data);
        }

        if res.total_count != u32::try_from(sections.len())? {
            bail!(
                "expected to fetch {} sections, but actually got {}",
                res.total_count,
                sections.len()
            );
        }

        let mut courses: HashMap<(String, String), super::Course> = HashMap::new();

        for section in sections {
            let subject_code = section.subject.clone();
            let course_code = section.course_number.clone();

            let course = courses
                .entry((subject_code.clone(), course_code.clone()))
                .or_insert_with(|| super::Course {
                    subject_code,
                    course_code,
                    title: section.course_title.clone(),
                    campus: section.campus_description.clone(),
                    sections: Vec::new(),
                });

            course.sections.push(section.try_into()?);
        }

        let mut courses = courses.into_values().collect::<Vec<_>>();
        courses.sort_by_cached_key(|k| (k.subject_code.to_string(), k.course_code.to_string()));

        Ok(courses)
    }

    #[instrument(skip(client, term))]
    async fn fetch_sections_partial(
        client: Client,
        term: Term,
        offset: u32,
    ) -> Result<SectionResults> {
        debug!("fetching offset {}", offset);
        let text = client
            .get(format!(
                "{}/searchResults/searchResults?txt_term={}&pageOffset={}&pageMaxSize=10000",
                URL_PREFIX, term, offset
            ))
            .send()
            .await?
            .text()
            .await?;
        match serde_json::from_str::<SectionResults>(&text) {
            Result::Ok(results) => Ok(results),
            Result::Err(e) => {
                let line = text
                    .lines()
                    .nth(e.line() - 1)
                    .with_context(|| anyhow!("can't find line for error: {}", e))?;
                bail!("line: {}\nerr: {}", line, e);
            }
        }
    }

    #[instrument()]
    pub async fn fetch_terms() -> Result<Vec<Term>> {
        #[derive(Deserialize)]
        struct TermResult {
            code: String,
        }

        Ok(Client::new()
            .get(format!(
                "{}/classSearch/getTerms?searchTerm=&offset=1&max=10000",
                URL_PREFIX
            ))
            .send()
            .await?
            .json::<Vec<TermResult>>()
            .await?
            .into_iter()
            .map(|t| t.code.parse::<Term>())
            .collect::<Result<Vec<_>>>()?)
    }
}
