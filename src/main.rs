#[tokio::main]
async fn main() -> anyhow::Result<()> {
    stock_screener::app::run().await
}
