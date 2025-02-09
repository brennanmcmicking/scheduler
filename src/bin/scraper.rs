use anyhow::Result;
use clap::Parser;
use scheduler::scraper::{scrape, Term};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
/// Downloads section info to SQLite databases in the current folder.
///
/// By default, it will only download past terms once, and will always redownload current or future
/// terms
struct Args {
    /// Term to scrape from. If missing, scrape all terms.
    ///
    /// Format: YYYYMM
    term: Option<Term>,

    /// Force download term, even if we already have an up-to-date copy
    #[arg(long, short, default_value_t = false)]
    force: bool,

    /// Oldest term to possibly fetch, refusing any older terms. This is overridden by the
    /// positional TERM argument if present
    #[arg(long, short, value_name = "TERM")]
    oldest: Option<Term>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| ["scraper=info", "scheduler=debug"].join(",").into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    scrape(args.force, args.oldest).await
}
