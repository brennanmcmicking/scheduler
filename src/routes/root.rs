use crate::middlewares::CookieUserState;
use axum::extract::{Extension, State};
use maud::{html, Markup};

use crate::components;

use super::AppState;

pub async fn root<S: AppState>(
    State(state): State<S>,
    Extension(user_state): CookieUserState,
) -> Markup {
    dbg!(&user_state);

    let courses: Vec<String> = state
        .courses()
        .iter()
        .filter(|x| {
            user_state
                .selection
                .clone()
                .iter()
                .filter(|course| course.name == **x)
                .count()
                == 0
        })
        .map(|course| course.to_owned())
        .collect();

    return components::base(html! {
        (components::container::c(&courses))
    });
}
