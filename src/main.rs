use axum::{
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use clap::Parser;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::fs;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct Context {
    filename: String,
    content: String,
}

#[derive(Deserialize)]
struct ContextSelection {
    filename: String,
}

#[derive(Deserialize)]
struct ChangeCodeRequest {
    change: String,
}

#[derive(Deserialize)]
struct QuestionRequest {
    question: String,
}

#[derive(Serialize, Deserialize)]
struct SessionRequest {
    model: String,
    voice: String,
    instructions: String,
}

async fn get_contexts(
    static_dir: String,
) -> Result<Json<Vec<Context>>, (StatusCode, Json<ErrorResponse>)> {
    let mut contexts = Vec::new();

    // Read all files in the static directory
    let entries = fs::read_dir(&static_dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read directory: {}", e),
            }),
        )
    })?;

    // Filter and process context files
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("CONTEXT_") && filename.ends_with(".md") {
                    let content = fs::read_to_string(&path).map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: format!("Failed to read file {}: {}", filename, e),
                            }),
                        )
                    })?;

                    contexts.push(Context {
                        filename: filename.to_string(),
                        content,
                    });
                }
            }
        }
    }

    Ok(Json(contexts))
}

async fn create_session(
    Json(payload): Json<SessionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "API key not found".to_string(),
            }),
        )
    })?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/realtime/sessions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create session: {}", e),
                }),
            )
        })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("API error: {}", response.status()),
            }),
        ));
    }

    let json = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse response: {}", e),
            }),
        )
    })?;

    Ok(Json(json))
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to serve static files from
    #[arg(short, long, default_value = "static")]
    static_dir: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let args = Args::parse();

    // Build our application with routes
    let static_dir = args.static_dir.clone();
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../static/index.html")) }),
        )
        .route("/api/sessions", post(create_session))
        .route("/contexts", get(move || get_contexts(static_dir.clone())))
        .route("/select-context", post(handle_context_selection))
        .route("/change-code", post(handle_change_code))
        .route("/ask-question", post(handle_question))
        .nest_service("/static", ServeDir::new("./static"));

    // Run it with hyper on localhost:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn handle_context_selection(
    Json(payload): Json<ContextSelection>,
) -> StatusCode {
    println!("Context selected: {}", payload.filename);
    StatusCode::OK
}

async fn handle_change_code(
    Json(payload): Json<ChangeCodeRequest>,
) -> Json<String> {
    println!("Code change requested: {}", payload.change);
    Json(String::from("I've analyzed your code change request. Here's what I would suggest..."))
}

async fn handle_question(
    Json(payload): Json<QuestionRequest>,
) -> Json<String> {
    println!("Question asked: {}", payload.question);
    Json(String::from("Based on the codebase, here's what I can tell you..."))
}
