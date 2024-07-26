use anyhow::{bail, Ok, Result};
use clap::{Parser, ValueEnum};
use jiff::civil::{Date, Time};
use rusqlite::Connection;
use std::{fmt::Display, path::Path, str::FromStr};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Season {
    Spring,
    Summer,
    Fall,
}

impl From<Season> for u8 {
    fn from(value: Season) -> Self {
        match value {
            Season::Spring => 1,
            Season::Summer => 5,
            Season::Fall => 9,
        }
    }
}

impl TryFrom<u64> for Season {
    type Error = anyhow::Error;

    fn try_from(month: u64) -> Result<Season> {
        Ok(match month {
            1 => Season::Spring,
            5 => Season::Summer,
            9 => Season::Fall,
            _ => bail!("Term month must be 1, 5, or 9, but was {} instead", month),
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Term {
    season: Season,
    year: u32,
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let season_number = match self.season {
            Season::Spring => 1,
            Season::Summer => 5,
            Season::Fall => 9,
        };
        write!(f, "{}{:02}", self.year, season_number)
    }
}

impl FromStr for Term {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.len() != 6 {
            bail!("Term string should be 6 characters: 4 for year, 2 for month")
        }
        let (year, month) = s.split_at(s.len() - 2);
        let year = year.parse()?;
        let season = month.parse::<u64>()?.try_into()?;
        Ok(Term { year, season })
    }
}

#[derive(Parser)]
struct Args {
    /// Term to scrape from. If missing, scrape all terms.
    ///
    /// Format: YYYYMM
    term: Option<Term>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Args { term: arg_term } = Args::parse();

    let terms = if let Some(term) = arg_term {
        vec![term]
    } else {
        println!("fetching list of all terms");
        scrape::fetch_terms().await?
    };

    // no point in parallelizing, UVic's server is the bottleneck
    for &term in terms.iter() {
        let filename = format!("sections_{}.sqlite3", term);
        if arg_term.is_none() && Path::new(&filename).exists() {
            println!("db already downloaded for {}", term);
            continue;
        }
        println!("fetching sections for term {}", term);

        let sections = scrape::fetch_sections(term).await?;
        persist(filename, &sections)?;
    }

    Ok(())
}

fn persist<P: AsRef<Path>>(filename: P, sections: &Vec<Section>) -> Result<()> {
    let conn = Connection::open_in_memory()?;

    store_sections(&conn, sections)?;

    conn.backup(rusqlite::DatabaseName::Main, filename, None)?;

    Ok(())
}

/// Store sessions to database `conn`, creating tables and writing rows. Writing to a non-empty
/// database will likely produce an error.
fn store_sections(conn: &Connection, sections: &Vec<Section>) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;

        CREATE TABLE section (
            crn INTEGER NOT NULL PRIMARY KEY ,

            subject_code TEXT NOT NULL,
            course_code TEXT NOT NULL,
            sequence_code TEXT NOT NULL,

            title TEXT NOT NULL,
            campus TEXT NOT NULL,

            enrollment INTEGER NOT NULL,
            enrollment_capacity INTEGER NOT NULL,
            waitlist INTEGER NOT NULL,
            waitlist_capacity INTEGER NOT NULL
        ) STRICT;
        CREATE INDEX section_subject ON section(subject_code);
        CREATE INDEX section_subject_course ON section(subject_code, course_code);

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

    for section in sections {
        conn.execute(
            "INSERT INTO section (
                crn, subject_code, course_code, sequence_code, title, campus,
                enrollment, enrollment_capacity, waitlist, waitlist_capacity
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
            (
                section.crn,
                &section.subject_code,
                &section.course_code,
                &section.sequence_code,
                &section.title,
                &section.campus,
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

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Days {
    monday: bool,
    tuesday: bool,
    wednesday: bool,
    thursday: bool,
    friday: bool,
    saturday: bool,
    sunday: bool,
}

#[derive(Debug, Clone)]
struct MeetingTime {
    start_time: Option<Time>,
    end_time: Option<Time>,
    start_date: Date,
    end_date: Date,

    days: Days,

    building: Option<String>,
    room: Option<String>,
}

#[derive(Debug, Clone)]
struct Section {
    crn: u64,
    subject_code: String,
    course_code: String,
    sequence_code: String,

    title: String,
    campus: String,

    enrollment: u32,
    enrollment_capacity: u32,
    waitlist: u32,
    waitlist_capacity: u32,

    meeting_times: Vec<MeetingTime>,
}

mod scrape {
    use anyhow::{anyhow, bail, Context, Ok, Result};
    use jiff::civil::{Date, Time};
    use reqwest::Client;
    use serde::{Deserialize, Serialize};

    use crate::Term;

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
                title: s.course_title,
                campus: s.campus_description,
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
                            days: crate::Days {
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

    pub async fn fetch_sections(term: Term) -> Result<Vec<crate::Section>> {
        let client = Client::builder().cookie_store(true).build()?;

        // setup the good cookies
        client
            .get(format!(
                "{}/classSearch/classSearch?term={}&txt_subject=CSUP&txt_courseNumber=000",
                URL_PREFIX, term
            ))
            .send()
            .await?;

        let res = fetch_sections_partial(client.clone(), term, 0).await?;

        let mut sections = res.data;

        let sections_left = res.total_count - u32::try_from(sections.len()).unwrap();
        // ceil division
        let requests_left = (sections_left + res.page_max_size - 1) / res.page_max_size;

        let handles = (0..requests_left).map(|i| {
            fetch_sections_partial(
                client.clone(),
                term,
                u32::try_from(sections.len()).unwrap() + i * res.page_max_size,
            )
        });

        for res in futures::future::join_all(handles).await {
            let res = res?;
            sections.extend(res.data);
        }

        if res.total_count != u32::try_from(sections.len()).unwrap() {
            bail!(
                "expected to fetch {} sections, but actually got {}",
                res.total_count,
                sections.len()
            );
        }

        Ok(sections
            .into_iter()
            .map(crate::Section::try_from)
            .collect::<Result<_>>()?)
    }

    async fn fetch_sections_partial(
        client: Client,
        term: Term,
        offset: u32,
    ) -> Result<SectionResults> {
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
