//! End-to-end checks against the real router and a temporary SQLite file.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use stock_screener::app::create_app_with_token;
use stock_screener::models::StockRow;
use stock_screener::utils::stock_database::StockDatabase;
use tower::ServiceExt;

const API_TOKEN: &str = "test-token";

fn stock(ticker: &str, company: &str, total_score: f64, potential: bool) -> StockRow {
    StockRow {
        company: Some(company.to_string()),
        sector: Some("Technology".to_string()),
        market_cap: Some(3_000_000_000_000.0),
        volume: Some(10_000_000.0),
        price: Some(100.0),
        change: Some(2.0),
        fundamental_score: Some(90.0),
        technical_score: Some(80.0),
        total_score: Some(total_score),
        potential_stock: Some(potential),
        ..StockRow::with_ticker(ticker)
    }
}

/// Each test gets its own database file so the suite can run in parallel.
fn app(name: &str) -> Router {
    let path = std::env::temp_dir()
        .join("stock-screener-tests")
        .join(format!("api-{name}.sqlite"));
    let _ = std::fs::remove_file(&path);
    let database = StockDatabase::new(&path);
    database.initialize().expect("database initializes");
    database
        .replace_scored_stocks(vec![
            stock("AAPL", "Apple Inc", 90.0, true),
            stock("MSFT", "Microsoft Corp", 80.0, false),
            stock("NVDA", "NVIDIA Corp", 70.0, true),
        ])
        .expect("stocks are stored");

    create_app_with_token(database, Some(API_TOKEN.to_string()))
}

async fn get_json(name: &str, uri: &str) -> (StatusCode, Value) {
    let response = app(name)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .expect("router responds");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn rejects_a_request_without_the_api_token() {
    let (status, body) = get_json("no-token", "/screener").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "Unauthorized");
}

#[tokio::test]
async fn returns_the_full_page_sorted_by_total_score() {
    let (status, body) = get_json("full-page", &format!("/screener?api_token={API_TOKEN}")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["count"], 3);
    assert_eq!(body["limit"], 100);
    assert_eq!(body["offset"], 0);
    assert_eq!(body["has_more"], false);
    assert_eq!(body["next_offset"], Value::Null);
    let tickers: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["ticker"].as_str().unwrap())
        .collect();
    assert_eq!(tickers, ["AAPL", "MSFT", "NVDA"]);
    assert_eq!(body["data"][0]["name"], "Apple Inc");
    assert_eq!(body["data"][0]["fundamental"]["fundamental_score"], 90.0);
    assert_eq!(body["data"][0]["technical"]["technical_score"], 80.0);
}

#[tokio::test]
async fn paginates_with_limit_and_offset() {
    let (_, first_page) = get_json(
        "page-1",
        &format!("/screener?api_token={API_TOKEN}&limit=2"),
    )
    .await;

    assert_eq!(first_page["has_more"], true);
    assert_eq!(first_page["next_offset"], 2);
    assert_eq!(first_page["data"].as_array().unwrap().len(), 2);

    let (_, second_page) = get_json(
        "page-2",
        &format!("/screener?api_token={API_TOKEN}&limit=2&offset=2"),
    )
    .await;

    assert_eq!(second_page["has_more"], false);
    assert_eq!(second_page["data"][0]["ticker"], "NVDA");
}

#[tokio::test]
async fn filters_by_potential_stock_search_and_ticker() {
    let (_, potential) = get_json(
        "potential",
        &format!("/screener?api_token={API_TOKEN}&potential_stock=true"),
    )
    .await;
    assert_eq!(potential["count"], 2);

    let (_, searched) = get_json(
        "search",
        &format!("/screener?api_token={API_TOKEN}&search=microsoft"),
    )
    .await;
    assert_eq!(searched["data"][0]["ticker"], "MSFT");

    let (_, by_ticker) = get_json(
        "tickers",
        &format!("/screener?api_token={API_TOKEN}&tickers=nvda,aapl"),
    )
    .await;
    assert_eq!(by_ticker["count"], 2);
}

#[tokio::test]
async fn sorts_ascending_when_asked() {
    let (_, body) = get_json(
        "ascend",
        &format!("/screener?api_token={API_TOKEN}&order=total_score&ascend=true"),
    )
    .await;

    assert_eq!(body["data"][0]["ticker"], "NVDA");
}

#[tokio::test]
async fn reports_whether_a_token_is_valid() {
    let response = app("auth")
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth")
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"api_token":"{API_TOKEN}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(body["authorized"], true);
}
