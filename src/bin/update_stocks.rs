//! Manually refresh all persisted stock screener data.

use std::time::Instant;

use clap::Parser;
use stock_screener::app::init_logging;
use stock_screener::services::fundamental::fundamental_screener_service::FundamentalScreenerService;
use stock_screener::services::integrated::integrated_screener_builder::IntegratedScreenerBuilder;
use stock_screener::services::technical::technical_screener_service::TechnicalScreenerService;
use stock_screener::utils::stock_database::StockDatabase;

const DEFAULT_LIMIT: i64 = 10_000;

#[derive(Parser)]
#[command(about = "Manually refresh all persisted stock screener data.")]
struct Args {
    /// Maximum Finviz screener rows to pull.
    #[arg(long, default_value_t = DEFAULT_LIMIT)]
    limit: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_logging();

    let args = Args::parse();
    let started_at = Instant::now();
    let database = StockDatabase::default();

    tracing::info!(
        "開始手動更新 stocks table limit={} db={}",
        args.limit,
        database.display_name()
    );
    database.initialize()?;

    let builder = IntegratedScreenerBuilder::new(
        FundamentalScreenerService::default(),
        TechnicalScreenerService::default(),
    );
    let rows = builder.build(args.limit).await;
    if rows.is_empty() || !rows.iter().any(|row| row.ticker.is_some()) {
        tracing::error!(
            "手動更新失敗：獲取返嚟嘅資料冇 ticker rows={} elapsed={:.2}s",
            rows.len(),
            started_at.elapsed().as_secs_f64()
        );
        std::process::exit(1);
    }

    let stored = database.replace_scored_stocks(rows)?;
    if stored.is_empty() {
        tracing::error!(
            "手動更新失敗：過濾冇 Total Score stocks 後冇 ticker rows=0 elapsed={:.2}s",
            started_at.elapsed().as_secs_f64()
        );
        std::process::exit(1);
    }

    tracing::info!(
        "手動更新完成 rows={} elapsed={:.2}s",
        stored.len(),
        started_at.elapsed().as_secs_f64()
    );
    Ok(())
}
