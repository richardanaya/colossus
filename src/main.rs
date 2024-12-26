use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use clap::Parser;
use colored::*;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    project_dir: String,
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

struct AppStateWithDir {
    current_context: Mutex<Option<Context>>,
    project_dir: String,
    model: String,
    preferred_language: String,
    instructions: String,
    voice: String,
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

    Ok(Json(contexts))
}

async fn create_session(
    State(state): State<Arc<AppStateWithDir>>,
    Json(mut payload): Json<SessionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    payload.voice = state.voice.clone();
    payload.instructions = format!(
        "The preferred language is {}. {}",
        state.preferred_language, state.instructions
    );
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "API key not found".to_string(),
                project_dir: String::new(),
            }),
        )
    })?;

    payload.model = state.model.clone();
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
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("API error: {}", response.status()),
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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to serve project files from
    #[arg(short = 'd', long, default_value = "./")]
    project_dir: String,

    /// Port number to run the server on
    #[arg(short, long, default_value = "49999")]
    port: u16,

    /// OpenAI model name to use
    #[arg(short, long, default_value = "gpt-4o-mini-realtime-preview-2024-12-17")]
    model: String,

    // Preferred language
    #[arg(short = 'l', long, default_value = "english")]
    preferred_language: String,

    // instructions
    #[arg(
        short,
        long,
        default_value = "You're name is COLOSSUS. You are a lighthearted, and serious AI that takes code seriously, but you have some wit.  Avoid saying anything that sounds like raw code or json. You are a helpful assistant working with a user to understand and modify a codebase. You can help answer questions about the codebase and make changes to the codebase. You talk very quickly and concisely so I don't have to hear alot of words.  Any time i'm talking about wanting to change something, it's almost always likely a change to the codebase.  Almost any time i'm asking a question, it's usually about the codebase."
    )]
    instructions: String,

    // voice
    #[arg(short, long, default_value = "ash")]
    voice: String,
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

    let args = Args::parse();

    // Check requirements before starting
    if let Err(error) = check_requirements(&args.project_dir) {
        eprintln!("{}", error.bright_red());
        std::process::exit(1);
    }

    // Initialize global state
    let state_with_dir = Arc::new(AppStateWithDir {
        preferred_language: args.preferred_language.clone(),
        current_context: Mutex::new(None),
        project_dir: args.project_dir.clone(),
        model: args.model.clone(),
        instructions: args.instructions.clone(),
        voice: args.voice.clone(),
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
        .route("/change-code", post(handle_change_code))
        .route("/ask-question", post(handle_question))
        .with_state(state_with_dir)
        .nest_service("/static", ServeDir::new("./static"));

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
        "Colossus Server:".bright_green(),
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

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_context_selection(
    State(state_with_dir): State<Arc<AppStateWithDir>>,
    Json(payload): Json<ContextSelection>,
) -> StatusCode {
    let mut current_context = state_with_dir.current_context.lock().unwrap();
    *current_context = Some(Context {
        filename: payload.filename.clone(),
        content: String::new(), // You might want to load the content here
    });
    println!("Context selected: {}", payload.filename);
    StatusCode::OK
}

async fn handle_change_code(
    State(state_with_dir): State<Arc<AppStateWithDir>>,
    Json(payload): Json<ChangeCodeRequest>,
) -> Result<Json<String>, (StatusCode, Json<ErrorResponse>)> {
    let project_dir = &state_with_dir.project_dir;
    let current_context = state_with_dir.current_context.lock().unwrap();

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
        .arg(&payload.change)
        .output()
        .map_err(|e| {
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

async fn handle_question(
    State(state_with_dir): State<Arc<AppStateWithDir>>,
    Json(payload): Json<QuestionRequest>,
) -> Json<String> {
    let current_context = state_with_dir.current_context.lock().unwrap();
    let project_dir = &state_with_dir.project_dir;
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
