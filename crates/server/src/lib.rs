//! Minimal loopback HTTP application. Library services follow the feasibility gates.
pub mod inspection;
pub mod source;

use std::path::Path;

use axum::{Json, Router, http::StatusCode, routing::get};
use rustune_contracts::HealthResponse;
use tower_http::services::{ServeDir, ServeFile};

pub fn router(web_dist: &Path) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .fallback(|| async { StatusCode::NOT_FOUND });

    let static_files =
        ServeDir::new(web_dist).not_found_service(ServeFile::new(web_dist.join("index.html")));

    Router::new()
        .nest("/api/v1", api)
        .fallback_service(static_files)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "rustune".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        stage: "foundation".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_works_without_assets_database_or_audio() {
        let response = router(Path::new("/nonexistent/rustune-assets"))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 4096).await.unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(health.service, "rustune");
        assert_eq!(health.stage, "foundation");
    }

    #[tokio::test]
    async fn unknown_api_does_not_fall_back_to_html() {
        let response = router(Path::new("/nonexistent/rustune-assets"))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
