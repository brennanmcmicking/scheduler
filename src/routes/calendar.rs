use crate::{
    components::container::{calendar_view_container, courses_container},
    middlewares::SelectedCourses,
    scraper::{Term, ThinCourse},
};
use axum::{
    extract::{Form, Path, Query, RawForm, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use maud::html;
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;

use super::{AppError, DatabaseAppState};

#[derive(Deserialize, Debug)]
pub struct Search {
    course: ThinCourse,
}

#[instrument(level = "debug", skip(state))]
pub async fn add_to_calendar<'a, 'b>(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    selected: SelectedCourses,
    Form(Search { course }): Form<Search>,
) -> Result<impl IntoResponse, AppError> {
    let course_exists = selected.courses.keys().any(|c| *c == course);

    let (jar, selected) = if course_exists {
        // no-op if course is already in state
        (CookieJar::new(), selected)
    } else {
        let default_sections = state.default_thin_sections(&term, course.clone())?;

        let mut new_selected = selected.clone();
        new_selected.courses.insert(course, default_sections);

        (
            CookieJar::new().add(new_selected.make_cookie(term)),
            new_selected,
        )
    };

    let courses = state.courses(term, &selected.thin_courses())?;

    Ok((
        jar,
        html! {
            (calendar_view_container(true))
            (courses_container(true, term, &courses, &selected))
        },
    ))
}

#[instrument(level = "debug", skip(state))]
pub async fn rm_from_calendar(
    Path(term): Path<Term>,
    State(state): State<Arc<DatabaseAppState>>,
    selected: SelectedCourses,
    Query(Search { course }): Query<Search>,
) -> Result<impl IntoResponse, AppError> {
    // no-op if course is not in cookie
    if !selected.courses.keys().any(|c| *c == course) {
        let courses = state.courses(term, &selected.thin_courses())?;

        return Ok((
            CookieJar::new(),
            html! {
                (calendar_view_container(true))
                (courses_container(true, term, &courses, &selected))
            },
        ));
    }

    let mut new_selected = selected.clone();
    new_selected
        .courses
        .retain(|thin_course, _| thin_course.course_code != course.course_code);

    let jar = CookieJar::new().add(new_selected.make_cookie(term));

    let courses = state.courses(term, &new_selected.thin_courses())?;

    Ok((
        jar,
        html! {
            (calendar_view_container(true))
            (courses_container(true, term, &courses, &new_selected))
        },
    ))
}

// HANDLER FOR COURSE SECTIONS
// because Axum doesn't support duplicated fields in form data...
#[derive(Debug, PartialEq)]
struct SectionQuery(ThinCourse, Vec<u64>);

impl TryFrom<&RawForm> for SectionQuery {
    type Error = anyhow::Error;

    fn try_from(form: &RawForm) -> Result<Self, Self::Error> {
        use std::collections::HashMap;

        let parsed_form = url::form_urlencoded::parse(&form.0);
        let mut query_map: HashMap<String, Vec<String>> = HashMap::new();

        for (key, val) in parsed_form {
            match query_map.get_mut(key.as_ref()) {
                None => {
                    query_map.insert(key.to_string(), vec![val.to_string()]);
                }
                Some(entry) => {
                    entry.push(val.to_string());
                }
            };
        }

        let course_code = if let Some(c) = query_map.get("course_code") {
            if c.len() != 1 {
                Err(anyhow::anyhow!("duplicates found for `course_code`"))
            } else {
                Ok(c[0].to_string())
            }
        } else {
            Err(anyhow::anyhow!("`course_code` not found"))
        }?;

        let subject_code = if let Some(c) = query_map.get("subject_code") {
            if c.len() != 1 {
                Err(anyhow::anyhow!("duplicates found for `subject_code`"))
            } else {
                Ok(c[0].to_string())
            }
        } else {
            Err(anyhow::anyhow!("`subject_code` not found"))
        }?;

        let mut crn: Vec<u64> = Vec::new();
        if let Some(c) = query_map.get("crns") {
            for code in c.iter() {
                crn.push(code.parse()?);
            }
        }

        Ok(Self(
            ThinCourse {
                course_code,
                subject_code,
            },
            crn,
        ))
    }
}

#[instrument(level = "debug")]
pub async fn course_section(
    Path(term): Path<Term>,
    selected: SelectedCourses,
    formdata: RawForm,
) -> Result<impl IntoResponse, AppError> {
    dbg!(&formdata);
    let sections = SectionQuery::try_from(&formdata)?;
    dbg!(&sections);
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::body::Bytes;

    use super::*;

    #[test]
    fn section_query_form_deserialization() {
        let bytes = b"course_code=355&subject_code=CSC&crns=10795&crns=10796";
        let rawform = RawForm(Bytes::copy_from_slice(bytes));

        let expected = SectionQuery(
            ThinCourse {
                course_code: "355".to_string(),
                subject_code: "CSC".to_string(),
            },
            vec![10795, 10796],
        );

        let result = SectionQuery::try_from(&rawform).unwrap();

        assert_eq!(expected, result);

        // with 1 crn only
        let bytes = b"course_code=355&subject_code=CSC&crns=10795";
        let rawform = RawForm(Bytes::copy_from_slice(bytes));

        let expected = SectionQuery(
            ThinCourse {
                course_code: "355".to_string(),
                subject_code: "CSC".to_string(),
            },
            vec![10795],
        );

        let result = SectionQuery::try_from(&rawform).unwrap();

        assert_eq!(expected, result);
    }
}
