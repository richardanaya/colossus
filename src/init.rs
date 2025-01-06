use std::path::Path;
use std::process::Command;
use std::fs;

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

    println!("Initialized project in '{}'", dir);
    Ok(())
}
