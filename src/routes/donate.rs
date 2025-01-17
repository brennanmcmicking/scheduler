use axum::response::IntoResponse;
use maud::html;

use crate::{components, middlewares::Session};

use super::AppError;



pub async fn get(
    session: Option<Session>
) -> Result<impl IntoResponse, AppError> {
    Ok(components::base(html!(
        div class="w-full h-full flex justify-center items-center" {
            div class="m-2" {
                p { "this website does not run for free, it costs approximately CAD$11 per month" }
                p { 
                    "please consider donating by e-transfering"
                    strong { " brennanmcmicking@gmail.com " }
                    "and make sure to put "
                    em { " scheduler " }
                    "in the memo"
                }
            }
        }
    ), session))
}