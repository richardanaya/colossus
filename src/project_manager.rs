use std::fs;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use crate::{AppStateWithDir, ActivityMode};

pub async fn project_manager_loop(
    project_dir: String,
    shutdown_signal: Arc<Mutex<bool>>,
    state_with_dir: Arc<AppStateWithDir>,
) {
    let mut interval = time::interval(Duration::from_secs(60)); // Check every minute

    loop {
        interval.tick().await;

        // Check if we should shutdown
        {
            let shutdown = shutdown_signal.lock().await;
            if *shutdown {
                break;
            }
        }

        // Check activity mode
        {
            let mode = state_with_dir.activity_mode.lock().await;
            if !matches!(*mode, ActivityMode::Planning) {
                println!("Not in planning mode - skipping update check");
                continue;
            }
        }

        println!("\nChecking if we should update TASKS.md");

        // Check file modification times
        let project_path = std::path::Path::new(&project_dir).join("PROJECT.md");
        let architecture_path = std::path::Path::new(&project_dir).join("ARCHITECTURE.md");
        let tasks_path = std::path::Path::new(&project_dir).join("TASKS.md");

        // If TASKS.md doesn't exist, we should create it
        let should_run_aider = if !tasks_path.exists() {
            println!("TASKS.md doesn't exist - creating it");
            true
        } else if let (Ok(project_meta), Ok(architecture_meta), Ok(tasks_meta)) = (
            fs::metadata(&project_path),
            fs::metadata(&architecture_path),
            fs::metadata(&tasks_path),
        ) {
            if let (Ok(project_modified), Ok(architecture_modified), Ok(tasks_modified)) = (
                project_meta.modified(),
                architecture_meta.modified(),
                tasks_meta.modified(),
            ) {
                println!("Checking file modification times:");
                println!("- PROJECT.md last modified: {:?}", project_modified);
                println!("- ARCHITECTURE.md last modified: {:?}", architecture_modified);
                println!("- TASKS.md last modified: {:?}", tasks_modified);

                let project_newer = project_modified > tasks_modified;
                let architecture_newer = architecture_modified > tasks_modified;
                
                println!(
                    "PROJECT.md is {} than TASKS.md",
                    if project_newer { "newer" } else { "older or same" }
                );
                println!(
                    "ARCHITECTURE.md is {} than TASKS.md",
                    if architecture_newer { "newer" } else { "older or same" }
                );

                project_newer || architecture_newer
            } else {
                println!("Could not get modification times for files");
                false
            }
        } else {
            println!("Could not get metadata for files");
            false
        };

        println!(
            "Decision to update TASKS.md: {}",
            if should_run_aider { "YES" } else { "NO" }
        );

        if should_run_aider {
            println!("Updating TASKS.md");
            let mut cmd = Command::new("aider");
            cmd.current_dir(&project_dir)
                .arg("--no-suggest-shell-commands")
                .arg("--yes-always")
                .arg("--message")
                .arg("Given the PROJECT.md and ARCHITECTURE.md, create or update TASKS.md with an ordered list of technical tasks for developers to work on today. Follow these rules:
1. Tasks must be ordered by dependency - things needed first must be at the top
2. Each task should be a small, incremental unit of work
3. Tasks should be clear and actionable with relevant technical details
4. The goal is to have a testable product by end of day
5. Break down large tasks into smaller steps
6. Include any setup/config tasks needed early
7. Focus on delivering working functionality over perfection
8. Mark tasks that are critical path for testing
9. Include estimates of time required for each task
10. Ensure the sequence leads to a testable product by end of day")
                .arg("PROJECT.md")
                .arg("ARCHITECTURE.md")
                .arg("TASKS.md");

            if let Some(model) = &state_with_dir.code_model {
                cmd.arg("--model").arg(model);
            }

            let output = cmd.output().map_err(|e| {
                eprintln!("Failed to run aider: {}", e);
            });

            if let Ok(output) = output {
                if !output.status.success() {
                    eprintln!(
                        "Aider command failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
        }
    }
    println!("ProjectManager thread shutting down cleanly");
}
