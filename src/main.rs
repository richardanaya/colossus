use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
mod cli;
use cli::{Cli, Commands};
use clap::Parser;
use colored::*;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    project_dir: String,
}

async fn handle_web_search(
    Json(payload): Json<WebSearchRequest>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let api_key = std::env::var("PERPLEXITY_API_KEY").map_err(|_| {
        (
            StatusCode::OK,
            Json(ErrorResponse {
                error: "I can't search the web right now because the Perplexity API key is not configured.".to_string(),
                project_dir: String::new(),
            }),
        )
    })?;

    let client = reqwest::Client::new();
    let search_request = json!({
        "model": "llama-3.1-sonar-small-128k-online",
        "messages": [
            {
                "role": "system",
                "content": "Be precise and concise."
            },
            {
                "role": "user",
                "content": payload.question
            }
        ],
        "temperature": 0.2,
        "top_p": 0.9,
        "search_domain_filter": ["perplexity.ai"],
        "return_images": false,
        "return_related_questions": false,
        "search_recency_filter": "month",
        "top_k": 0,
        "stream": false,
        "presence_penalty": 0,
        "frequency_penalty": 1
    });

    let response = client
        .post("https://api.perplexity.ai/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&search_request)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to search web: {}", e),
                    project_dir: String::new(),
                }),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error response".to_string());
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("API error: {} - {}", status, error_body),
                project_dir: String::new(),
            }),
        ));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse response: {}", e),
                project_dir: String::new(),
            }),
        )
    })?;

    // Extract the content from the response
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("No content found in response")
        .to_string();

    Ok(Json(content))
}

#[derive(Serialize)]
struct Context {
    filename: String,
    content: String,
}

#[derive(Deserialize)]
struct ChangeCodeRequest {
    change: String,
    context: String,
}

#[derive(Deserialize)]
struct QuestionRequest {
    question: String,
    context: String,
}

#[derive(Deserialize)]
struct WebSearchRequest {
    question: String,
}

#[derive(Deserialize)]
struct ModeToggleRequest {
    mode: String,
}

#[derive(Deserialize)]
struct TranscriptUpdate {
    content: String,
}

#[derive(Serialize, Deserialize)]
struct SessionRequest {
    model: String,
    voice: String,
    instructions: String,
}

#[derive(Clone)]
pub enum ActivityMode {
    Planning,
    Developing,
    ErrorNeedsHuman,
}

struct AppStateWithDir {
    shutdown_signal: Arc<Mutex<bool>>,
    project_dir: String,
    model: String,
    preferred_language: String,
    instructions: String,
    voice: String,
    code_model: Option<String>,
    activity_mode: Arc<Mutex<ActivityMode>>,
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
                project_dir: static_dir.clone(),
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
                                project_dir: static_dir.clone(),
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

    // If no contexts were found, add a default "None" context
    if contexts.is_empty() {
        contexts.push(Context {
            filename: "None".to_string(),
            content: String::new(),
        });
    }

    Ok(Json(contexts))
}

async fn create_session(
    State(state): State<Arc<AppStateWithDir>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let payload = SessionRequest {
        model: state.model.clone(),
        voice: state.voice.clone(),
        instructions: format!(
            "The preferred language is {}. {}",
            state.preferred_language, state.instructions
        ),
    };
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "API key not found".to_string(),
                project_dir: String::new(),
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
                    project_dir: String::new(),
                }),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error response".to_string());
        println!("Session creation failed:");
        println!("Status: {}", status);
        println!("Error body: {}", error_body);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("API error: {} - {}", status, error_body),
                project_dir: String::new(),
            }),
        ));
    }

    let json = response.json().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse response: {}", e),
                project_dir: String::new(),
            }),
        )
    })?;

    Ok(Json(json))
}


fn check_requirements(project_dir: &str) -> Result<(), String> {
    // Check for .git directory
    let git_dir = std::path::Path::new(project_dir).join(".git");
    if !git_dir.exists() {
        return Err("No .git directory found. Please run this from a git repository.".to_string());
    }

    // Check for OPENAI_API_KEY
    if std::env::var("OPENAI_API_KEY").is_err() {
        return Err("OPENAI_API_KEY environment variable is not set. Put it in your environment variables or a .env file.".to_string());
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let cli = Cli::parse();
    
    match cli.command {
        Commands::Serve(args) => {
            // Start server with args

    // Check requirements before starting
    if let Err(error) = check_requirements(&args.project_dir) {
        eprintln!("{}", error.bright_red());
        std::process::exit(1);
    }

    // Initialize global state
    let shutdown_signal = Arc::new(Mutex::new(false));

    let state_with_dir = Arc::new(AppStateWithDir {
        shutdown_signal: shutdown_signal.clone(),
        preferred_language: args.preferred_language.clone(),
        project_dir: args.project_dir.clone(),
        model: args.model.clone(),
        instructions: args.instructions.clone(),
        voice: args.voice.clone(),
        code_model: args.code_model.clone(),
        activity_mode: Arc::new(Mutex::new(ActivityMode::Planning)), // Default to Planning mode
    });

    // Start ProductManager thread
    let product_manager_shutdown = shutdown_signal.clone();
    let project_dir_clone = args.project_dir.clone();
    let state_with_dir_clone = state_with_dir.clone();
    tokio::spawn(async move {
        product_manager_loop(
            project_dir_clone,
            product_manager_shutdown,
            state_with_dir_clone,
        )
        .await;
    });

    // Start Architect thread
    let architect_shutdown = shutdown_signal.clone();
    let project_dir_clone = args.project_dir.clone();
    let state_with_dir_clone = state_with_dir.clone();
    tokio::spawn(async move {
        architect_loop(
            project_dir_clone,
            architect_shutdown,
            state_with_dir_clone,
        )
        .await;
    });

    // Start ProjectManager thread
    let project_manager_shutdown = shutdown_signal.clone();
    let project_dir_clone = args.project_dir.clone();
    let state_with_dir_clone = state_with_dir.clone();
    tokio::spawn(async move {
        project_manager_loop(
            project_dir_clone,
            project_manager_shutdown,
            state_with_dir_clone,
        )
        .await;
    });

    // Start Tester thread
    let tester_shutdown = shutdown_signal.clone();
    let project_dir_clone = args.project_dir.clone();
    let state_with_dir_clone = state_with_dir.clone();
    tokio::spawn(async move {
        tester_loop(
            project_dir_clone,
            tester_shutdown,
            state_with_dir_clone,
        )
        .await;
    });

    // Start Developer thread
    let developer_shutdown = shutdown_signal.clone();
    let project_dir_clone = args.project_dir.clone();
    let state_with_dir_clone = state_with_dir.clone();
    tokio::spawn(async move {
        developer_loop(
            project_dir_clone,
            developer_shutdown,
            state_with_dir_clone,
        )
        .await;
    });
    let project_dir = args.project_dir.clone();
    let app = Router::new()
        .route("/", get(|| async { Html(include_str!("html/index.html")) }))
        .route(
            "/app.js",
            get(|| async {
                axum::response::Response::builder()
                    .header("Content-Type", "application/javascript")
                    .body(include_str!("js/app.js").to_string())
                    .unwrap()
            }),
        )
        .route("/api/sessions", post(create_session))
        .route("/contexts", get(move || get_contexts(project_dir.clone())))
        .route("/change-code", post(handle_change_code))
        .route("/ask-question", post(handle_question))
        .route("/web-search", post(handle_web_search))
        .route("/update-transcript", post(handle_transcript_update))
        .route("/toggle-mode", post(handle_toggle_mode))
        .route("/current-mode", get(get_current_mode))
        .with_state(state_with_dir.clone());

    println!("{}", "          /\\          ".bright_cyan());
    println!("{}", "         /  \\         ".bright_cyan());
    println!("{}", "        /    \\        ".bright_cyan());
    println!("{}", "       /      \\       ".bright_cyan());
    println!("{}", "      /   __   \\      ".bright_cyan());
    println!("{}", "     /   |  |   \\     ".bright_cyan());
    println!("{}", "    /    |  |    \\    ".bright_cyan());
    println!("{}", "   /     |  |     \\   ".bright_cyan());
    println!("{}", "  /_____/|  |\\_____\\  ".bright_cyan());
    println!("{}", " /_____/ |__| \\_____\\ ".bright_cyan());
    println!("{}", "/______/_|__|_\\______\\".bright_cyan());
    println!(
        "\n{} {}",
        "Colossus server:".bright_green(),
        format!("http://localhost:{}", args.port).yellow()
    );
    println!(
        "{} {}",
        "Language:".bright_green(),
        args.preferred_language.yellow()
    );
    println!("{} {}", "Model:".bright_green(), args.model.yellow());
    println!(
        "{} {}",
        "Project directory:".bright_green(),
        args.project_dir.yellow()
    );
    println!("{} {}", "Voice:".bright_green(), args.voice.yellow());

    // Construct example aider command preview
    let mut example_cmd = String::from("aider --no-suggest-shell-commands --yes-always");
    if let Some(model) = &args.code_model {
        example_cmd.push_str(&format!(" --model {}", model));
    }
    example_cmd.push_str(" --load \"<context_file>\" --message \"<message>\"");
    println!(
        "{} {}",
        "Example aider command:".bright_green(),
        example_cmd.yellow()
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    let listener = TcpListener::bind(addr).await.unwrap();
    // Run server and handle graceful shutdown
    let server = axum::serve(listener, app);

    // Wait for server to finish
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

            // Signal shutdown to ProductManagerInterview thread
            let mut shutdown = state_with_dir.shutdown_signal.lock().await;
            *shutdown = true;
        }
    }
}

async fn handle_change_code(
    State(state_with_dir): State<Arc<AppStateWithDir>>,
    Json(payload): Json<ChangeCodeRequest>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let project_dir = &state_with_dir.project_dir;

    let mut cmd = Command::new("aider");
    cmd.current_dir(project_dir)
        .arg("--no-suggest-shell-commands")
        .arg("--yes-always")
        .arg("--message")
        .arg(&payload.change);

    if let Some(model) = &state_with_dir.code_model {
        cmd.arg("--model").arg(model);
    }

    if payload.context != "None" {
        cmd.arg("--load").arg(&payload.context);
    }

    let output = cmd.output().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to execute aider: {}", e),
                project_dir: project_dir.clone(),
            }),
        )
    })?;

    if output.status.success() {
        Ok(Json(String::from_utf8_lossy(&output.stdout).to_string()))
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!(
                    "Aider command failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
                project_dir: project_dir.clone(),
            }),
        ))
    }
}

async fn get_current_mode(
    State(state): State<Arc<AppStateWithDir>>,
) -> Json<String> {
    let mode = state.activity_mode.lock().await;
    Json(match *mode {
        ActivityMode::Planning => "planning".to_string(),
        ActivityMode::Developing => "developing".to_string(),
        ActivityMode::ErrorNeedsHuman => "error".to_string(),
    })
}

async fn handle_toggle_mode(
    State(state): State<Arc<AppStateWithDir>>,
    Json(payload): Json<ModeToggleRequest>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let new_mode = match payload.mode.as_str() {
        "planning" => ActivityMode::Planning,
        "developing" => ActivityMode::Developing,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid mode specified".to_string(),
                    project_dir: state.project_dir.clone(),
                }),
            ))
        }
    };

    let mut mode = state.activity_mode.lock().await;
    *mode = new_mode;

    Ok(Json(format!("Mode changed to {}", payload.mode)))
}

async fn handle_transcript_update(
    State(state_with_dir): State<Arc<AppStateWithDir>>,
    Json(payload): Json<TranscriptUpdate>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let transcript_path = std::path::Path::new(&state_with_dir.project_dir).join("TRANSCRIPT.md");

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&transcript_path)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to open transcript file: {}", e),
                    project_dir: state_with_dir.project_dir.clone(),
                }),
            )
        })?;

    file.write_all(payload.content.as_bytes()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to write transcript: {}", e),
                project_dir: state_with_dir.project_dir.clone(),
            }),
        )
    })?;

    Ok(Json("Transcript updated successfully".to_string()))
}

mod product_manager;
mod architect;
mod project_manager;
mod tester;
mod developer;

use product_manager::product_manager_loop;
use architect::architect_loop;
use project_manager::project_manager_loop;
use tester::tester_loop;
use developer::developer_loop;

async fn handle_question(
    State(state_with_dir): State<Arc<AppStateWithDir>>,
    Json(payload): Json<QuestionRequest>,
) -> Json<String> {
    let project_dir = &state_with_dir.project_dir;

    let mut cmd = Command::new("aider");
    cmd.current_dir(project_dir)
        .arg("--no-suggest-shell-commands")
        .arg("--yes-always")
        .arg("--message")
        .arg(&payload.question);

    if let Some(model) = &state_with_dir.code_model {
        cmd.arg("--model").arg(model);
    }

    if payload.context != "None" {
        cmd.arg("--load").arg(&payload.context);
    }

    let output = cmd.output().expect("Failed to execute aider");

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
