use core::fmt;
use serde::{Deserialize, Deserializer, Serialize};
use std::{cmp::Ordering, fmt::Display, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};
use clap::ValueEnum;
use jiff::{
    civil::{date, Date, Time},
    ToSpan, Zoned,
};

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
            Some(s) => {
                match s {
                    "Spring" => Ok(Season::Spring),
                    "Summer" => Ok(Season::Summer),
                    "Fall" => Ok(Season::Fall),
                    _ => Err(())
                }
            },
            None => Err(())
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
            .intz("America/Vancouver")
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
