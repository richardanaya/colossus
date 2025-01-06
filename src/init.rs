use std::path::Path;
use std::process::Command;

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

    println!("Initialized project in '{}'", dir);
    Ok(())
}
