use crate::{ActivityMode, AppStateWithDir};
use std::fs;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

pub async fn product_manager_loop(
    project_dir: String,
    shutdown_signal: Arc<Mutex<bool>>,
    state_with_dir: Arc<AppStateWithDir>,
) {
    let mut interval = time::interval(Duration::from_secs(10));

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
        let transcript_path = std::path::Path::new(&project_dir).join("TRANSCRIPT.md");
        let project_path = std::path::Path::new(&project_dir).join("PROJECT.md");

        // If PROJECT.md doesn't exist, we should create it
        let should_run_aider = if !project_path.exists() {
            true
        } else if let (Ok(transcript_meta), Ok(project_meta)) =
            (fs::metadata(&transcript_path), fs::metadata(&project_path))
        {
            if let (Ok(transcript_modified), Ok(project_modified)) =
                (transcript_meta.modified(), project_meta.modified())
            {
                println!("Checking file modification times:");
                println!("- TRANSCRIPT.md last modified: {:?}", transcript_modified);
                println!("- PROJECT.md last modified: {:?}", project_modified);

                let transcript_newer = transcript_modified > project_modified;
                println!(
                    "TRANSCRIPT.md is {} than PROJECT.md",
                    if transcript_newer {
                        "newer"
                    } else {
                        "older or same"
                    }
                );

                transcript_newer
            } else {
                println!("Could not get modification times for files");
                false
            }
        } else {
            println!("Could not get metadata for files");
            false
        };

        if should_run_aider {
            println!("📝 Updating PROJECT.md from transcript...");
            let mut cmd = Command::new("aider");
            cmd.current_dir(&project_dir)
                .arg("--no-suggest-shell-commands")
                .arg("--yes-always")
                .arg("--message")
                .arg("given the TRANSCRIPT.md update PROJECT.md")
                .arg("TRANSCRIPT.md")
                .arg("PROJECT.md");

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
                    println!("PROJECT.md updated successfully");
                    println!("Aider output: {}", String::from_utf8_lossy(&output.stdout));
                }
            }
        }
    }
    println!("ProductManagerInterview thread shutting down cleanly");
}
