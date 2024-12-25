use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use clap::Parser;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::{Arc, Mutex};
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

struct AppState {
    current_context: Mutex<Option<Context>>,
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
    /// Directory to serve project files from
    #[arg(short, long, default_value = "static")]
    project_dir: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let args = Args::parse();

    // Initialize global state
    let state = Arc::new(AppState {
        current_context: Mutex::new(None),
    });
    let project_dir = args.project_dir.clone();
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../static/index.html")) }),
        )
        .route("/api/sessions", post(create_session))
        .route("/contexts", get(move || get_contexts(project_dir.clone())))
        .route("/select-context", post(handle_context_selection))
        .with_state(state.clone())
        .route("/change-code", post(handle_change_code))
        .route("/ask-question", post(handle_question))
        .with_state(project_dir.clone())
        .nest_service("/static", ServeDir::new("./static"));

    // Run it with hyper on localhost:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn handle_context_selection(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ContextSelection>,
) -> StatusCode {
    let mut current_context = state.current_context.lock().unwrap();
    *current_context = Some(Context {
        filename: payload.filename.clone(),
        content: String::new(), // You might want to load the content here
    });
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
    State(state): State<Arc<AppState>>,
    State(project_dir): State<String>,
    Json(payload): Json<QuestionRequest>,
) -> Json<String> {
    let current_context = state.current_context.lock().unwrap();
    let context_file = if let Some(context) = &*current_context {
        &context.filename
    } else {
        ""
    };

    let output = Command::new("aider")
        .current_dir(project_dir)
        .arg("--load")
        .arg(context_file)
        .arg("--no-suggest-shell-commands")
        .arg("--yes-always")
        .arg("--message")
        .arg(&payload.question)
        .output()
        .expect("Failed to execute aider");

    let response_message = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        format!(
            "Failed to get response from aider: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    };

    Json(response_message)
}
