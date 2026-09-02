use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Instant;

use ghostmark_core::image_stripper;
use ghostmark_core::text_scrubber;

// ─── Request / Response Models ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CleanTextRequest {
    pub text: String,
    #[serde(default)]
    pub shatter_synthid: bool,
}

#[derive(Serialize)]
pub struct CleanTextResponse {
    pub ok: bool,
    pub original_len: usize,
    pub cleaned_len: usize,
    pub chars_removed: usize,
    pub cleaned: String,
    pub elapsed_us: u128,
}

#[derive(Deserialize)]
pub struct CleanImageRequest {
    /// Base64-encoded image bytes
    pub file: String,
    /// Original filename (used to detect format)
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Serialize)]
pub struct CleanImageResponse {
    pub ok: bool,
    pub original_size: usize,
    pub cleaned_size: usize,
    pub bytes_removed: usize,
    /// Base64-encoded cleaned image
    pub cleaned: String,
    pub elapsed_us: u128,
}

#[derive(Serialize)]
pub struct InspectTextResponse {
    pub ok: bool,
    pub suspicious: bool,
    pub suspicious_chars: Vec<SuspiciousChar>,
    pub total_suspicious: usize,
}

#[derive(Serialize)]
pub struct SuspiciousChar {
    pub position: usize,
    pub codepoint: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub version: String,
    pub engine: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub ok: bool,
    pub error: String,
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        ok: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        engine: "GhostMark/Rust".to_string(),
    })
}

async fn openapi() -> impl IntoResponse {
    let spec = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "GhostMark API",
            "description": "Memory-safe API for stripping AI provenance watermarks from text and images.",
            "version": env!("CARGO_PKG_VERSION"),
            "license": { "name": "MIT" }
        },
        "paths": {
            "/health": {
                "get": {
                    "summary": "Service health check",
                    "responses": { "200": { "description": "Service is healthy" } }
                }
            },
            "/clean/text": {
                "post": {
                    "summary": "Strip Unicode watermarks from text",
                    "requestBody": { "content": { "application/json": { "schema": { "type": "object", "properties": { "text": { "type": "string" } }, "required": ["text"] } } } },
                    "responses": { "200": { "description": "Cleaned text response" } }
                }
            },
            "/inspect/text": {
                "post": {
                    "summary": "Detect watermark characters in text",
                    "requestBody": { "content": { "application/json": { "schema": { "type": "object", "properties": { "text": { "type": "string" } }, "required": ["text"] } } } },
                    "responses": { "200": { "description": "Inspection result" } }
                }
            },
            "/clean/image": {
                "post": {
                    "summary": "Strip C2PA/EXIF metadata from images",
                    "requestBody": { "content": { "application/json": { "schema": { "type": "object", "properties": { "file": { "type": "string", "format": "byte" }, "name": { "type": "string" } }, "required": ["file", "name"] } } } },
                    "responses": { "200": { "description": "Cleaned image response" } }
                }
            }
        }
    });
    Json(spec)
}

async fn clean_text(Json(payload): Json<CleanTextRequest>) -> impl IntoResponse {
    let start = Instant::now();
    let original_len = payload.text.len();
    let cleaned = if payload.shatter_synthid {
        text_scrubber::shatter_synthid_text(&payload.text)
    } else {
        text_scrubber::sanitize_text(&payload.text, false)
    };
    let cleaned_len = cleaned.len();
    let elapsed = start.elapsed().as_micros();

    Json(CleanTextResponse {
        ok: true,
        original_len,
        cleaned_len,
        chars_removed: original_len.saturating_sub(cleaned_len),
        cleaned,
        elapsed_us: elapsed,
    })
}

async fn inspect_text(Json(payload): Json<CleanTextRequest>) -> impl IntoResponse {
    let mut suspicious_chars = Vec::new();

    for (i, c) in payload.text.chars().enumerate() {
        let (is_suspect, name) = match c {
            '\u{200B}' => (true, "Zero Width Space"),
            '\u{200C}' => (true, "Zero Width Non-Joiner"),
            '\u{200D}' => (true, "Zero Width Joiner"),
            '\u{200E}' => (true, "Left-to-Right Mark"),
            '\u{200F}' => (true, "Right-to-Left Mark"),
            '\u{FEFF}' => (true, "Zero Width No-Break Space (BOM)"),
            '\u{E0000}'..='\u{E007F}' => (true, "Unicode Tag Character"),
            _ => (false, ""),
        };

        if is_suspect {
            suspicious_chars.push(SuspiciousChar {
                position: i,
                codepoint: format!("U+{:04X}", c as u32),
                name: name.to_string(),
            });
        }
    }

    let total = suspicious_chars.len();
    Json(InspectTextResponse {
        ok: true,
        suspicious: total > 0,
        suspicious_chars,
        total_suspicious: total,
    })
}

async fn clean_image(
    Json(payload): Json<CleanImageRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Decode base64 input
    let raw_bytes = BASE64.decode(&payload.file).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                ok: false,
                error: format!("Invalid base64 input: {}", e),
            }),
        )
    })?;

    let start = Instant::now();
    let original_size = raw_bytes.len();

    let cleaned_bytes = image_stripper::strip_image_bytes(&raw_bytes).map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                ok: false,
                error: format!("Image processing failed: {}", e),
            }),
        )
    })?;

    let cleaned_size = cleaned_bytes.len();
    let elapsed = start.elapsed().as_micros();
    let cleaned_b64 = BASE64.encode(&cleaned_bytes);

    Ok(Json(CleanImageResponse {
        ok: true,
        original_size,
        cleaned_size,
        bytes_removed: original_size.saturating_sub(cleaned_size),
        cleaned: cleaned_b64,
        elapsed_us: elapsed,
    }))
}

// ─── Server Entrypoint ───────────────────────────────────────────────────────

/// Builds the axum [`Router`]. Kept separate from [`start_server`] so tests can
/// drive the routes directly without binding a socket.
pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi))
        .route("/clean/text", post(clean_text))
        .route("/inspect/text", post(inspect_text))
        .route("/clean/image", post(clean_image))
}

pub async fn start_server(host: &str, port: u16) -> std::io::Result<()> {
    let app = build_router();

    let addr: SocketAddr = format!("{}:{}", host, port).parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Invalid host:port '{}:{}': {}", host, port, e),
        )
    })?;

    println!();
    println!("  ██████╗ ██╗  ██╗ ██████╗ ███████╗████████╗███╗   ███╗ █████╗ ██████╗ ██╗  ██╗");
    println!("  ██╔════╝ ██║  ██║██╔═══██╗██╔════╝╚══██╔══╝████╗ ████║██╔══██╗██╔══██╗██║ ██╔╝");
    println!("  ██║  ███╗███████║██║   ██║███████╗   ██║   ██╔████╔██║███████║██████╔╝█████╔╝ ");
    println!("  ██║   ██║██╔══██║██║   ██║╚════██║   ██║   ██║╚██╔╝██║██╔══██║██╔══██╗██╔═██╗ ");
    println!("  ╚██████╔╝██║  ██║╚██████╔╝███████║   ██║   ██║ ╚═╝ ██║██║  ██║██║  ██║██║  ██╗");
    println!("   ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝");
    println!();
    println!("  👻 GhostMark HTTP Proxy v{}", env!("CARGO_PKG_VERSION"));
    println!("  🚀 Listening on http://{}", addr);
    println!("  🛡️  Engine: Rust/Axum (memory-safe, zero-copy)");
    println!();
    println!("  Routes:");
    println!("    GET  /health         → Service health check");
    println!("    GET  /openapi.json   → OpenAPI 3.0.3 specification");
    println!("    POST /clean/text     → Strip Unicode watermarks from text");
    println!("    POST /inspect/text   → Detect watermark characters in text");
    println!("    POST /clean/image    → Strip C2PA/Exif metadata from images");
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::ServiceExt;

    async fn call(method: Method, uri: &str, body: Body) -> (StatusCode, Value) {
        let resp = build_router()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::Null);
        (status, json)
    }

    fn json_body(value: Value) -> Body {
        Body::from(value.to_string())
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let (status, json) = call(Method::GET, "/health", Body::empty()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["ok"], true);
        assert!(json["engine"].as_str().unwrap().starts_with("GhostMark"));
    }

    #[tokio::test]
    async fn openapi_is_valid_json() {
        let (status, json) = call(Method::GET, "/openapi.json", Body::empty()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["openapi"], "3.0.3");
        assert!(json["paths"]["/clean/text"].is_object());
    }

    #[tokio::test]
    async fn clean_text_strips_zero_width_spaces() {
        let body = json_body(serde_json::json!({ "text": "a\u{200B}b" }));
        let (status, json) = call(Method::POST, "/clean/text", body).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["ok"], true);
        assert_eq!(json["cleaned"], "ab");
        assert_eq!(json["chars_removed"], 3);
    }

    #[tokio::test]
    async fn clean_text_shatter_mode_runs() {
        let body = json_body(serde_json::json!({
            "text": "The use of modern tools can make hard tasks easy.",
            "shatter_synthid": true
        }));
        let (status, json) = call(Method::POST, "/clean/text", body).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["ok"], true);
        assert!(!json["cleaned"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn inspect_text_flags_watermark_chars() {
        let body = json_body(serde_json::json!({ "text": "a\u{200B}\u{E0061}" }));
        let (status, json) = call(Method::POST, "/inspect/text", body).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["suspicious"], true);
        assert_eq!(json["total_suspicious"], 2);
        assert_eq!(json["suspicious_chars"][0]["name"], "Zero Width Space");
    }

    #[tokio::test]
    async fn clean_image_rejects_bad_base64() {
        let body = json_body(serde_json::json!({ "file": "!!!not-base64!!!", "name": "a.png" }));
        let (status, json) = call(Method::POST, "/clean/image", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["ok"], false);
    }

    #[tokio::test]
    async fn clean_image_rejects_unsupported_format() {
        let b64 = BASE64.encode(b"definitely not an image");
        let body = json_body(serde_json::json!({ "file": b64, "name": "a.bin" }));
        let (status, json) = call(Method::POST, "/clean/image", body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json["ok"], false);
    }
}
