use crate::{ActivityMode, AppStateWithDir};
use std::fs;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

pub async fn tester_loop(
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

        // Check activity mode quickly
        let should_continue = {
            let mode = state_with_dir.activity_mode.lock().await;
            matches!(*mode, ActivityMode::Planning)
        };

        if !should_continue {
            continue;
        }

        // Check file modification times
        let tasks_path = std::path::Path::new(&project_dir).join("TASKS.md");
        let architecture_path = std::path::Path::new(&project_dir).join("ARCHITECTURE.md");
        let test_plan_path = std::path::Path::new(&project_dir).join("TEST_STRATEGY.md");

        // If TEST_PLAN.md doesn't exist, we should create it
        let should_run_aider = if !test_plan_path.exists() {
            true
        } else if let (Ok(tasks_meta), Ok(architecture_meta), Ok(test_plan_meta)) = (
            fs::metadata(&tasks_path),
            fs::metadata(&architecture_path),
            fs::metadata(&test_plan_path),
        ) {
            if let (Ok(tasks_modified), Ok(architecture_modified), Ok(test_plan_modified)) = (
                tasks_meta.modified(),
                architecture_meta.modified(),
                test_plan_meta.modified(),
            ) {
                let tasks_newer = tasks_modified > test_plan_modified;
                let architecture_newer = architecture_modified > test_plan_modified;
                tasks_newer || architecture_newer
            } else {
                println!("Could not get modification times for files");
                false
            }
        } else {
            println!("Could not get metadata for files");
            false
        };

        if should_run_aider {
            println!("🧪 Updating TEST_STRATEGY.md from architecture...");
            let mut cmd = Command::new("aider");
            cmd.current_dir(&project_dir)
                .arg("--no-suggest-shell-commands")
                .arg("--yes-always")
                .arg("--message")
                .arg("Given the ARCHITECTURE.md, create or update TEST_STRATEGY.md with a minimal testing strategy. Focus on:
1. Simple unit tests using the language's built-in test framework
2. Basic integration tests for critical paths
3. Test-driven development workflow using vanilla tools
4. Keep everything as simple and maintainable as possible
5. Avoid complex tooling or CI pipelines - stick to local development testing")
                .arg("TASKS.md")
                .arg("ARCHITECTURE.md")
                .arg("TEST_STRATEGY.md");

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
                    println!("✨ Aider finished updating TEST_STRATEGY.md");
                }
            }
        }
    }
    println!("Tester thread shutting down cleanly");
}
