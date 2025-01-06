use std::path::Path;
use std::process::Command;
use std::fs;
use std::io::{self, Write};

pub fn project_init(dir: &str) -> Result<(), String> {
    let path = Path::new(dir);
    
    // Ensure directory exists
    if !path.exists() {
        return Err(format!("Directory '{}' does not exist", dir));
    }

    // Check for .git directory
    let git_dir = path.join(".git");
    if !git_dir.exists() {
        println!("No git repository found, initializing one...");
        
        // Initialize git repository
        let status = Command::new("git")
            .current_dir(path)
            .arg("init")
            .status()
            .map_err(|e| format!("Failed to run git init: {}", e))?;

        if !status.success() {
            return Err("Git initialization failed".to_string());
        }
        
        println!("Git repository initialized successfully");
    }

    // Check for .env file
    let env_path = path.join(".env");
    if !env_path.exists() {
        println!("No .env file found, creating template...");
        
        let env_template = r#"# API Keys for various services
# Replace <API_KEY> with your actual API keys

DEEPSEEK_API_KEY=<API_KEY>
PERPLEXITY_API_KEY=<API_KEY>
OPENAI_API_KEY=<API_KEY>
ANTHROPIC_API_KEY=<API_KEY>
"#;
        
        fs::write(&env_path, env_template)
            .map_err(|e| format!("Failed to create .env template: {}", e))?;
        
        println!("Created .env template file");
        println!("Please edit .env and add your API keys");
    }

    // Ask for programming language preference
    let language = select_language()?;
    println!("Selected language: {}", language);
    
    println!("Initialized project in '{}'", dir);
    Ok(())
}

fn create_language_context(dir: &str, language: &str) -> Result<(), String> {
    let path = Path::new(dir);
    let context_path = path.join("CONTEXT.md");
    
    let context_content = match language {
        "Rust" => r#"/add TASKS.md
/read-only ARCHITECTURE.md
/read-only PROJECT.md
/read-only TEST_STRATEGY.md
/read-only Makefile
/add src/**/*.rs
/add tests/**/*.rs
/add Cargo.toml"#,
        
        "Python" => r#"/add TASKS.md
/read-only ARCHITECTURE.md
/read-only PROJECT.md
/read-only TEST_STRATEGY.md
/read-only Makefile
/add **/*.py
/add requirements.txt
/add pyproject.toml"#,
        
        "JavaScript" => r#"/add TASKS.md
/read-only ARCHITECTURE.md
/read-only PROJECT.md
/read-only TEST_STRATEGY.md
/read-only Makefile
/add package.json
/add **/*.js
/add **/*.css
/add **/*.html
/add jest.config.js"#,
        
        "TypeScript" => r#"/add TASKS.md
/read-only ARCHITECTURE.md
/read-only PROJECT.md
/read-only TEST_STRATEGY.md
/read-only Makefile
/add package.json
/add tsconfig.json
/add **/*.ts
/add **/*.tsx
/add **/*.css
/add **/*.html
/add jest.config.ts"#,
        
        _ => return Err("Unsupported language".to_string()),
    };
    
    fs::write(&context_path, context_content)
        .map_err(|e| format!("Failed to create CONTEXT.md: {}", e))?;

    // Create language-specific Makefile
    let makefile_path = path.join("Makefile");
    let makefile_content = match language {
        "Rust" => r#".PHONY: build test

build:
	cargo build

test:
	cargo test"#,
        
        "Python" => r#".PHONY: build test

build:
	python -m pip install -r requirements.txt

test:
	python -m pytest"#,
        
        "JavaScript" => r#".PHONY: build test

build:
	npm install
	npm run build

test:
	npm test"#,
        
        "TypeScript" => r#".PHONY: build test

build:
	npm install
	npm run build

test:
	npm test"#,
        
        _ => return Err("Unsupported language".to_string()),
    };
    
    fs::write(&makefile_path, makefile_content)
        .map_err(|e| format!("Failed to create Makefile: {}", e))?;

    println!("Created CONTEXT.md and Makefile for {} development", language);
    Ok(())
}

fn select_language() -> Result<String, String> {
    let languages = vec!["Rust", "Python", "JavaScript", "TypeScript"];
    
    println!("\nSelect your preferred programming language:");
    for (i, lang) in languages.iter().enumerate() {
        println!("{}. {}", i + 1, lang);
    }
    
    print!("Enter the number (1-4): ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read input: {}", e))?;
    
    let selection = input.trim().parse::<usize>()
        .map_err(|_| "Please enter a valid number (1-4)".to_string())?;
    
    if selection < 1 || selection > languages.len() {
        return Err(format!("Please enter a number between 1 and {}", languages.len()));
    }
    
    let selected_lang = languages[selection - 1].to_string();
    
    // Create context file based on selected language
    create_language_context(dir, &selected_lang)?;
    
    Ok(selected_lang)
}
