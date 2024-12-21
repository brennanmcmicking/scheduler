use std::{sync::Arc, vec};

use anyhow::Result;
use axum::extract::{Path, Query, State};
use itertools::Itertools;
use maud::{html, Markup};
use serde::Deserialize;
use tracing::instrument;

use crate::{
    middlewares::Schedule,
    scraper::{Course, MeetingTime, Section, ThinSection},
};

use super::{AppError, DatabaseAppState};

#[derive(Debug, Deserialize)]
pub struct GenerationState {
    state: Option<String>,
    prev: Option<bool>,
}

#[instrument(level = "debug", skip(state))]
pub async fn get(
    Path(schedule_id): Path<String>,
    State(state): State<Arc<DatabaseAppState>>,
    Query(generation_state): Query<GenerationState>,
    schedule: Schedule,
) -> Result<Markup, AppError> {
    let courses = state.courses(schedule.term, &schedule.selected.thin_courses())?;
    let state = generation_state
        .state
        .map(|s| {
            s.split("_")
                .map(|x| Ok(ThinSection { crn: x.parse()? }))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?;

    let next_state = lazy_dfs(&courses, state.as_deref(), generation_state.prev.is_some());
    println!("{next_state:?}");

    let Some(sections) = next_state else {
        return Ok(html! {("no sections")});
    };

    let next_url = format!(
        "/schedule/{}/generate?state={}",
        schedule_id,
        sections.iter().map(|s| s.crn.to_string()).join("_")
    );
    let prev_url = next_url.clone() + "&prev=true";

    Ok(html! {
        div {
            a href=(prev_url) { ("< Prev") };
            (" | ")
            a href=(next_url) { ("Next >") };
        }
        @for section in &sections {
            div {(section.subject_code) (" ") (section.course_code) (" ") (section.sequence_code)};
        }
    })
}

fn lazy_dfs(
    courses: &[Course],
    state: Option<&[ThinSection]>,
    reverse: bool,
) -> Option<Vec<Section>> {
    // group sections into (course, section, type)
    let mut groups = courses
        .iter()
        .flat_map(|c| {
            c.sections
                .iter()
                .chunk_by(|&s| {
                    (
                        s.subject_code.clone(),
                        s.course_code.clone(),
                        s.sequence_code.chars().next(),
                    )
                })
                .into_iter()
                .map(|(_, chunk)| chunk.collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    if reverse {
        for group in &mut groups {
            group.reverse();
        }
    }

    find_next(groups.as_slice(), state)
}

fn find_next(groups: &[Vec<&Section>], state: Option<&[ThinSection]>) -> Option<Vec<Section>> {
    // ThinSection -> array offset
    let state: Option<Vec<usize>> = state.and_then(|state| {
        state
            .iter()
            .enumerate()
            .map(|(i, g)| {
                groups[i]
                    .iter()
                    .find_position(|s| s.crn == g.crn)
                    .map(|(pos, _)| pos)
            })
            .collect::<Option<Vec<_>>>()
    });
    let mut state = match state {
        Some(mut state) => {
            // don't repeat last solution, go next!
            if let Some(last) = state.last_mut() {
                *last += 1;
            };
            state
        }
        None => vec![0],
    };

    // actual DFS here
    loop {
        find_next_inner(groups, &mut state);
        if state.is_empty() {
            return None;
        } else if state.len() == groups.len() {
            break;
        }
    }

    // array offset -> ThinSection
    Some(
        state
            .into_iter()
            .enumerate()
            .map(|(i, j)| groups[i][j])
            .cloned()
            .collect(),
    )
}

fn find_next_inner(groups: &[Vec<&Section>], state: &mut Vec<usize>) {
    let height = state.len() - 1;
    let row = &groups[height];

    let start = state.pop().unwrap();

    for (i, section) in row.iter().enumerate().skip(start) {
        state.push(i);
        if height > 0
            && state
                .iter()
                .enumerate()
                .take(height)
                .any(|(i, &j)| section_conflict(section, groups[i][j]))
        {
            state.pop();
            continue;
        } else if state.len() < groups.len() {
            // parent is valid, check children
            state.push(0);
        }
        return;
    }

    // no more children match, start loop at next parent
    if let Some(last) = state.last_mut() {
        *last += 1;
    }
}

fn section_conflict(a: &Section, b: &Section) -> bool {
    a.meeting_times.iter().any(|a_time| {
        b.meeting_times
            .iter()
            .any(|b_time| time_conflict(a_time, b_time))
    })
}

fn time_conflict(a: &MeetingTime, b: &MeetingTime) -> bool {
    if !((a.days.monday && b.days.monday)
        || (a.days.tuesday && b.days.tuesday)
        || (a.days.wednesday && b.days.wednesday)
        || (a.days.thursday && b.days.thursday)
        || (a.days.friday && b.days.friday)
        || (a.days.saturday && b.days.saturday)
        || (a.days.sunday && b.days.sunday))
    {
        return false;
    }

    if a.end_date < b.start_date || b.end_date < a.start_date {
        return false;
    }

    if a.end_time < b.start_time || b.end_time < a.start_time {
        return false;
    }

    true
}

// TODO: overwrite "selected" cookie with generated schedule
#[instrument(level = "debug", skip(_state))]
pub async fn post(
    Path(schedule_id): Path<String>,
    State(_state): State<Arc<DatabaseAppState>>,
    schedule: Schedule,
) -> Result<Markup, AppError> {
    Ok(html! {
        ("hi")
    })
}
