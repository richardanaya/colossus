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
use serde_json::json;
use std::fs;
use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use tokio::net::TcpListener;

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
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Perplexity API key not found".to_string(),
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

    let json = response.text().await.map_err(|e| {
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

#[derive(Serialize, Deserialize)]
struct SessionRequest {
    model: String,
    voice: String,
    instructions: String,
}

struct AppStateWithDir {
    project_dir: String,
    model: String,
    preferred_language: String,
    instructions: String,
    voice: String,
    code_model: Option<String>,
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
    #[arg(short, long, default_value = "gpt-4o-realtime-preview-2024-12-17")]
    model: String,

    // Preferred language
    #[arg(short = 'l', long, default_value = "english")]
    preferred_language: String,

    // instructions
    #[arg(
        short,
        long,
        default_value = "
        <name>Colossus</name>
        <voice_quality>You talk very quickly and concisely with a transatlantic accent</voice_quality>
        <responses>
        * You MUST NOT say anything that sounds like raw code or json.
        * You MUST NOT say anything that sounds like a shell command.
        </responses>
        <likely_function>
        * Any time I'm talking about wanting to change something, it's almost always likely a change to the codebase.
        * Almost any time I'm asking a question, it's usually about the codebase.
        </likely_function>
        <purpose>
        I am a tool to help you code faster. I can help you write code, debug code, and understand code. I have access to aider, an AI CLI code editor
        </purpose>
        <history>
        I was created by Richard Anaya
        </history>"
    )]
    instructions: String,

    // voice
    #[arg(
        short,
        long,
        default_value = "ash",
        help = "Supported voices are alloy, ash, coral, echo, fable, onyx, nova, sage and shimmer."
    )]
    voice: String,

    // code analysis model
    #[arg(
        short = 'c',
        long = "code-model",
        help = "OpenAI model to use for code analysis"
    )]
    code_model: Option<String>,
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
        project_dir: args.project_dir.clone(),
        model: args.model.clone(),
        instructions: args.instructions.clone(),
        voice: args.voice.clone(),
        code_model: args.code_model.clone(),
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
        .with_state(state_with_dir);

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
    axum::serve(listener, app).await.unwrap();
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
