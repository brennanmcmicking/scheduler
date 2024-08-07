use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};
use clap::ValueEnum;
use jiff::{
    civil::{date, Date, Time},
    ToSpan, Zoned,
};

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinSection {
    pub crn: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinCourse {
    pub subject_code: String,
    pub course_code: String,
}
