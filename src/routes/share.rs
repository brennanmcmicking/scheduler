use axum::extract::{Host, OriginalUri, Path};
use maud::{html, Markup};
use tracing::instrument;

use crate::{
    common::{AppError, Schedule},
    components,
    data::store::Session,
};

#[instrument(level = "debug")]
pub async fn get(
    OriginalUri(uri): OriginalUri,
    Host(host): Host,
    Path(schedule_id): Path<String>,
    schedule: Schedule,
    session: Option<Session>,
) -> Result<Markup, AppError> {
    Ok(components::base(
        html! {
            div class="flex h-full w-full items-center justify-center" {
                div class="w-4/5 h-4/5 flex flex-col items-start gap-2" {
                    a href={"/schedule/" (schedule_id)} class="bg-green-500 dark:bg-green-600 hover:bg-green-700 hover:dark:bg-green-800 rounded-lg transition px-1 lg:p-1" {
                        "back"
                    }
                    textarea class="p-2 w-full text-black rounded-lg grow border-neutral-400 border-2 dark:border-0" readonly {
                        (uri.scheme_str().unwrap_or("http")) "://" (host) "/import?blob=" (schedule.to_base64())
                    }
                }
            }
        },
        session,
    ))
}
