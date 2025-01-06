use crate::{ActivityMode, AppStateWithDir};
use std::fs;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

pub async fn architect_loop(
    project_dir: String,
    shutdown_signal: Arc<Mutex<bool>>,
    state_with_dir: Arc<AppStateWithDir>,
) {
    let mut interval = time::interval(Duration::from_secs(60)); // Runs every 60 seconds

    loop {
        interval.tick().await;

        // Check if we should shutdown
        {
            let shutdown = shutdown_signal.lock().await;
            if *shutdown {
                break;
            }
        }

        // Check activity mode quickly
        let should_continue = {
            let mode = state_with_dir.activity_mode.lock().await;
            matches!(*mode, ActivityMode::Planning)
        };

        if !should_continue {
            continue;
        }

        // Check file modification times
        let project_path = std::path::Path::new(&project_dir).join("PROJECT.md");
        let architecture_path = std::path::Path::new(&project_dir).join("ARCHITECTURE.md");

        // If ARCHITECTURE.md doesn't exist, we should create it
        let should_run_aider = if !architecture_path.exists() {
            true
        } else if let (Ok(project_meta), Ok(architecture_meta)) = (
            fs::metadata(&project_path),
            fs::metadata(&architecture_path),
        ) {
            if let (Ok(project_modified), Ok(architecture_modified)) =
                (project_meta.modified(), architecture_meta.modified())
            {
                let project_newer = project_modified > architecture_modified;

                project_newer
            } else {
                false
            }
        } else {
            false
        };

        if should_run_aider {
            println!("🏗️ Updating ARCHITECTURE.md from project requirements...");
            let mut cmd = Command::new("aider");
            cmd.current_dir(&project_dir)
                .arg("--no-suggest-shell-commands")
                .arg("--yes-always")
                .arg("--message")
                .arg("given the PROJECT.md, update ARCHITECTURE.md with technical architecture details")
                .arg("PROJECT.md")
                .arg("ARCHITECTURE.md");

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
                } else {
                    println!("✨ Aider finished updating ARCHITECTURE.md");
                }
            }
        }
    }
    println!("Architect thread shutting down cleanly");
}
