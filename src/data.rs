use std::collections::HashMap;

use auth::DiscordClient;
use google_oauth::AsyncClient;
use r2d2_sqlite::SqliteConnectionManager;
use store::DynamoUserStore;

use crate::{common::Stage, scraper::Term};

pub mod auth;
pub mod store;

#[derive(Clone)]
pub struct DatabaseAppState
{
    pub terms: HashMap<Term, r2d2::Pool<SqliteConnectionManager>>,
    pub user_store: DynamoUserStore,
    pub google_client: AsyncClient,
    pub discord_client: DiscordClient,
    pub stage: Stage,
}