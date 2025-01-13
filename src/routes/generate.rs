use std::sync::Arc;

use anyhow::Result;
use axum::extract::{Path, Query, State};
use itertools::Itertools;
use maud::{html, Markup};
use serde::Deserialize;
use tracing::instrument;

use crate::{components, middlewares::{Schedule, Session}, scraper::ThinSection};

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
    session: Option<Session>
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

    let next_state =
        algo::next_schedule(&courses, state.as_deref(), generation_state.prev.is_some());

    let Some(sections) = next_state else {
        return Ok(html! {("no sections")});
    };

    let next_url = format!(
        "/schedule/{}/generate?state={}",
        schedule_id,
        sections.iter().map(|s| s.crn.to_string()).join("_")
    );
    let prev_url = next_url.clone() + "&prev=true";

    // let section_refs = sections.iter().collect();

    Ok(components::base(html! {
        (components::container::generator_container(&schedule_id, &sections, &prev_url, &next_url))
    }, session))
}

// http://localhost:8443/schedule/ce966dcd-8ff5-4168-8728-2da8cac5269e/generate?state=20654_20664_21144_21160_21194_21196_21887_21914_22540_22563
mod algo {
    /*
     * High-level overview:
     * - partition sections by (course, sequence_code[0])
     * - map state to indices (purify)
     * - map input to nested Vec's of times (purify)
     * - lazy dfs
     *   - find next non-conflicting option
     *   - if no options, return None
     *   - continue DFS from last state. input always means "find one after this"
     *   - next means:
     *     - next sibling
     *     - then next child
     *     - then finally next parent
     *   - root is special: no sibling, groups.len() children, no parent
     *   - leaf is no children
     *   - any valid leaf found is a solution
     *   - all states are valid leaves
     *   - stack invariant is always:
     *     - all parents (stack[0:n-1]) don't conflict
     *     - current is stack[n]
     * - map next_state back to sections (hydrate)
     */
    use itertools::Itertools;
    use jiff::civil::{Date, Time};

    use crate::scraper;

    pub fn next_schedule(
        courses: &[scraper::Course],
        state: Option<&[scraper::ThinSection]>,
        reverse: bool,
    ) -> Option<Vec<scraper::Section>> {
        let mut section_groups = partition(courses);
        if reverse {
            for group in &mut section_groups {
                group.reverse();
            }
        }

        let groups = Groups::new(&section_groups);

        let next_state_inner = find_next(&groups, state)?;

        // array offset -> ThinSection
        Some(
            next_state_inner
                .into_iter()
                .enumerate()
                .map(|(i, j)| section_groups[i][j].clone())
                .collect(),
        )

        //next_state.and_then(|next_state| {
        //    next_state
        //        .into_iter()
        //        .map(|state_section| {
        //            courses
        //                .iter()
        //                .map(|c| &c.sections)
        //                .flatten()
        //                .find(|s| s.crn == state_section.crn)
        //                .cloned()
        //        })
        //        .collect::<Option<Vec<_>>>()
        //})
    }

    // group sections into (course, sequence type)
    fn partition(courses: &[scraper::Course]) -> Vec<Vec<scraper::Section>> {
        courses
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
                    .map(|(_, chunk)| chunk.cloned().collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    struct Groups {
        groups: Vec<Group>,
    }

    impl Groups {
        fn new(groups: &[Vec<scraper::Section>]) -> Self {
            let groups = groups.iter().map(|g| Group::new(g)).collect();
            Groups { groups }
        }
    }

    struct Group {
        sections: Vec<Section>,
    }

    impl Group {
        fn new(sections: &[scraper::Section]) -> Self {
            let sections = sections.iter().map(Section::new).collect::<Vec<_>>();
            Group { sections }
        }
    }

    struct Section {
        crn: u64,
        times: Vec<MeetingTime>,
    }

    impl Section {
        fn new(s: &scraper::Section) -> Self {
            let times = s
                .meeting_times
                .iter()
                .map(MeetingTime::new)
                .collect::<Vec<_>>();
            Section { crn: s.crn, times }
        }
    }

    struct MeetingTime {
        start_time: Option<Time>,
        end_time: Option<Time>,
        start_date: Date,
        end_date: Date,

        days: u8,
    }

    impl MeetingTime {
        fn new(t: &scraper::MeetingTime) -> Self {
            let mut day_mask = 0u8;
            if t.days.monday {
                day_mask |= 1 << 0;
            }
            if t.days.tuesday {
                day_mask |= 1 << 1;
            }
            if t.days.wednesday {
                day_mask |= 1 << 2;
            }
            if t.days.thursday {
                day_mask |= 1 << 3;
            }
            if t.days.friday {
                day_mask |= 1 << 4;
            }
            if t.days.saturday {
                day_mask |= 1 << 5;
            }
            if t.days.sunday {
                day_mask |= 1 << 6;
            }
            MeetingTime {
                start_time: t.start_time,
                end_time: t.end_time,
                start_date: t.start_date,
                end_date: t.end_date,
                days: day_mask,
            }
        }
    }

    impl PartialEq for MeetingTime {
        fn eq(&self, other: &Self) -> bool {
            // no conflict in term
            if self.end_date < other.start_date || other.end_date < self.start_date {
                return false;
            }
            // no conflict in week
            if (self.days & other.days) == 0 {
                return false;
            }
            // no conflict in day
            if self.end_time < other.start_time || other.end_time < self.start_time {
                return false;
            }

            true
        }
    }

    fn find_next(groups: &Groups, state: Option<&[scraper::ThinSection]>) -> Option<Vec<usize>> {
        // ThinSection -> array offset
        let state: Option<Vec<usize>> = state.and_then(|state| {
            state
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    groups.groups[i]
                        .sections
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
            let done = find_next_inner(groups, &mut state);
            if state.is_empty() {
                return None;
            } else if done {
                break;
            }
        }

        Some(state)
    }

    fn find_next_inner(groups: &Groups, state: &mut Vec<usize>) -> bool {
        let height = state.len() - 1;
        let row = &groups.groups[height];

        let start = state.pop().unwrap();

        for (i, section) in row.sections.iter().enumerate().skip(start) {
            state.push(i);
            if height > 0
                && state
                    .iter()
                    .enumerate()
                    .take(height)
                    .any(|(i, &j)| section_conflict(section, &groups.groups[i].sections[j]))
            {
                state.pop();
                continue;
            } else if state.len() < groups.groups.len() {
                // parent is valid, check children
                state.push(0);
                return false;
            }
            return true;
        }

        // no more children match, start loop at next parent
        if let Some(last) = state.last_mut() {
            *last += 1;
        }
        false
    }

    fn section_conflict(a: &Section, b: &Section) -> bool {
        a.times
            .iter()
            .any(|a_time| b.times.iter().any(|b_time| a_time == b_time))
    }

    #[cfg(test)]
    mod tests {

        #[test]
        fn test_empty() {}
    }
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
