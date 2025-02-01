use core::fmt;
use rusqlite::Connection;
use serde::{Deserialize, Deserializer, Serialize};
use tracing::info;
use std::{cmp::Ordering, fmt::Display, path::Path, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};
use clap::ValueEnum;
use jiff::{
    civil::{date, Date, Time},
    ToSpan, Zoned,
};

use crate::routes::SectionType;

#[derive(
    Clone, Copy, Debug, ValueEnum, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Season {
    Spring,
    Summer,
    Fall,
}

impl From<Season> for i8 {
    fn from(value: Season) -> Self {
        match value {
            Season::Spring => 1,
            Season::Summer => 5,
            Season::Fall => 9,
        }
    }
}

impl TryFrom<Option<&str>> for Season {
    type Error = ();

    fn try_from(value: Option<&str>) -> std::result::Result<Self, Self::Error> {
        match value {
            Some(s) => match s {
                "Spring" => Ok(Season::Spring),
                "Summer" => Ok(Season::Summer),
                "Fall" => Ok(Season::Fall),
                _ => Err(()),
            },
            None => Err(()),
        }
    }
}

impl TryFrom<i64> for Season {
    type Error = anyhow::Error;

    fn try_from(month: i64) -> Result<Season> {
        Ok(match month {
            1 => Season::Spring,
            5 => Season::Summer,
            9 => Season::Fall,
            _ => bail!("term month must be 1, 5, or 9, but was {} instead", month),
        })
    }
}

impl Display for Season {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Term {
    year: i16,
    season: Season,
}

impl Term {
    /// Returns the open-closed [x,y) time range for this term
    pub fn time_range(self) -> (Zoned, Zoned) {
        let start = date(self.year, self.season.into(), 1)
            .in_tz("America/Vancouver")
            .expect("bad hardcoded UVic timezone");
        let end = start.saturating_add(4.months());
        (start, end)
    }

    /// Tests whether `time` is during this term
    pub fn during(self, time: &Zoned) -> bool {
        let (start, end) = self.time_range();
        start <= *time && *time < end
    }

    pub fn human_display(self) -> String {
        format!("{} {}", self.season, self.year).to_ascii_lowercase()
    }
}

impl PartialOrd<Zoned> for Term {
    fn partial_cmp(&self, other: &Zoned) -> Option<Ordering> {
        let (start, end) = self.time_range();

        if *other < start {
            Some(Ordering::Greater)
        } else if *other >= end {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl PartialEq<Zoned> for Term {
    fn eq(&self, other: &Zoned) -> bool {
        self.during(other)
    }
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

    // fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    //     let mut split = s.split_ascii_whitespace();
    //     let season: Season = split.nth(0).try_into().map_err(|_e| anyhow!("failed to parse term season"))?;
    //     let year: i16 = match split.nth(1) {
    //         Some(s) => match s.parse() {
    //             Ok(v) => v,
    //             Err(_e) => bail!("failed to parse year")
    //         },
    //         None => bail!("no year present"),
    //     };

    //     Ok(Term {
    //         season,
    //         year,
    //     })
    // }

    fn from_str(s: &str) -> Result<Self> {
        if s.len() != 6 {
            bail!("term string should be 6 characters: 4 for year, 2 for month")
        }
        let (year, month) = s.split_at(s.len() - 2);
        let year = year.parse().context("failed to parse term year")?;
        let season = month
            .parse::<i64>()?
            .try_into()
            .map_err(|e| anyhow!("failed to parse term season: {e}"))?;
        Ok(Term { year, season })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Days {
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
}

impl Display for Days {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = if self.monday { "m" } else { "" }.to_string();
        let t = if self.tuesday { "t" } else { "" }.to_string();
        let w = if self.wednesday { "w" } else { "" }.to_string();
        let r = if self.thursday { "r" } else { "" }.to_string();
        let fr = if self.friday { "f" } else { "" }.to_string();
        let s = if self.saturday { "s" } else { "" }.to_string();
        let u = if self.sunday { "u" } else { "" }.to_string();
        write!(f, "{}{}{}{}{}{}{}", m, t, w, r, fr, s, u)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Day {
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    pub const WEEKDAYS: [Self; 5] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
    ];

    pub(crate) fn is_in_days(&self, days: Days) -> bool {
        match self {
            Day::Monday => days.monday,
            Day::Tuesday => days.tuesday,
            Day::Wednesday => days.wednesday,
            Day::Thursday => days.thursday,
            Day::Friday => days.friday,
            Day::Saturday => days.saturday,
            Day::Sunday => days.sunday,
        }
    }
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct MeetingTime {
    pub start_time: Option<Time>,
    pub end_time: Option<Time>,
    pub start_date: Date,
    pub end_date: Date,

    pub days: Days,

    pub building: Option<String>,
    pub room: Option<String>,
}

impl Display for MeetingTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?} - {:?}", self.days, self.start_time, self.end_time)
    }
}

#[derive(Debug, Clone)]
pub struct Section {
    pub crn: u64,

    pub subject_code: String,
    pub course_code: String,
    pub sequence_code: String,

    pub enrollment: u32,
    pub enrollment_capacity: u32,
    pub waitlist: u32,
    pub waitlist_capacity: u32,

    pub meeting_times: Vec<MeetingTime>,
}

impl Section {
    pub fn get_type(&self) -> SectionType {
        match self.sequence_code.chars().nth(0) {
            Some('A') => SectionType::Lecture,
            Some('B') => SectionType::Lab,
            Some('T') => SectionType::Tutorial,
            _ => panic!()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Course {
    pub subject_code: String,
    pub course_code: String,

    pub title: String,
    pub campus: String,

    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThinSection {
    pub crn: u64,
}

impl From<String> for ThinSection {
    fn from(value: String) -> Self {
        ThinSection {
            crn: u64::from_str(value.as_str()).unwrap(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThinCourse {
    pub subject_code: String,
    pub course_code: String,
}

// required for use as a map key
impl<'de> Deserialize<'de> for ThinCourse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let (subject_code, course_code) = s
            .split_once(' ')
            .ok_or_else(|| serde::de::Error::custom("should contain one space"))?;
        Ok(Self {
            subject_code: subject_code.to_string(),
            course_code: course_code.to_string(),
        })
    }
}

// required for use as a map key
impl Serialize for ThinCourse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("{} {}", self.subject_code, self.course_code).serialize(serializer)
    }
}


pub async fn scrape(force: bool, oldest: Option<Term>) -> Result<()> {
    info!("fetching list of all terms");
    let mut terms = scrape::fetch_terms().await?;
    if let Some(oldest) = oldest {
        terms.retain(|t| t >= &oldest)
    }

    let now = Zoned::now();

    // no point in parallelizing, UVic's server is the bottleneck
    for &term in terms.iter() {
        let filename = format!("sections_{}.sqlite3", term);

        if !force && term < now && Path::new(&filename).exists() {
            info!("db already downloaded for {}", term);
            continue;
        }
        info!("fetching sections for term {}", term);

        let courses = scrape::fetch_sections(term).await?;
        persist(filename, &courses)?;
    }

    Ok(())
}

pub fn persist<P: AsRef<Path>>(filename: P, courses: &Vec<Course>) -> Result<()> {
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

pub mod scrape {
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
