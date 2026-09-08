//! HTTP route handlers.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};

use crate::services::api::screener_request::Payload;
use crate::services::api::screener_service::ScreenerService;
use crate::utils::stock_database::StockDatabase;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<ScreenerService<StockDatabase>>,
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/screener", get(screener))
        .route("/auth", post(auth))
        .with_state(state)
}

/// Query parameters first, then a JSON body if one was sent, so the body wins.
fn get_request_payload(query: HashMap<String, String>, body: &Bytes) -> Payload {
    let mut payload = Payload::new();
    for (key, value) in query {
        payload.insert(key, Value::String(value));
    }
    if let Ok(Value::Object(body)) = serde_json::from_slice::<Value>(body) {
        payload.extend(body);
    }
    payload
}

async fn screener(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let payload = get_request_payload(query, &body);
    if !state.service.is_authorized(&payload) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    let service = state.service.clone();
    let response = tokio::task::spawn_blocking(move || service.get_screener_response(&payload))
        .await
        .expect("screener task does not panic");

    match response {
        Ok(response) => Json(response).into_response(),
        Err(error) => {
            tracing::error!("讀取 stocks table 失敗: {}", error);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal Server Error"})),
            )
                .into_response()
        }
    }
}

async fn auth(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let payload = get_request_payload(query, &body);
    Json(json!({"authorized": state.service.is_authorized(&payload)})).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lets_the_json_body_override_query_parameters() {
        let query = HashMap::from([
            ("limit".to_string(), "10".to_string()),
            ("sector".to_string(), "Technology".to_string()),
        ]);
        let body = Bytes::from_static(br#"{"limit": 25}"#);

        let payload = get_request_payload(query, &body);

        assert_eq!(payload["limit"], 25);
        assert_eq!(payload["sector"], "Technology");
    }

    #[test]
    fn ignores_a_body_that_is_not_a_json_object() {
        let query = HashMap::from([("limit".to_string(), "10".to_string())]);

        let payload = get_request_payload(query, &Bytes::from_static(b"not json"));

        assert_eq!(payload["limit"], "10");
    }
}
