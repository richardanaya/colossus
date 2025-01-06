use std::fs;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use crate::{AppStateWithDir, ActivityMode};

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

        println!("\nChecking if we should update TEST_STRATEGY.md");

        // Check file modification times
        let tasks_path = std::path::Path::new(&project_dir).join("TASKS.md");
        let architecture_path = std::path::Path::new(&project_dir).join("ARCHITECTURE.md");
        let test_plan_path = std::path::Path::new(&project_dir).join("TEST_STRATEGY.md");

        // If TEST_PLAN.md doesn't exist, we should create it
        let should_run_aider = if !test_plan_path.exists() {
            println!("TEST_STRATEGY.md doesn't exist - creating it");
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
                println!("Checking file modification times:");
                println!("- TASKS.md last modified: {:?}", tasks_modified);
                println!("- ARCHITECTURE.md last modified: {:?}", architecture_modified);
                println!("- TEST_STRATEGY.md last modified: {:?}", test_plan_modified);

                let tasks_newer = tasks_modified > test_plan_modified;
                let architecture_newer = architecture_modified > test_plan_modified;
                
                println!(
                    "TASKS.md is {} than TEST_STRATEGY.md",
                    if tasks_newer { "newer" } else { "older or same" }
                );
                println!(
                    "ARCHITECTURE.md is {} than TEST_STRATEGY.md",
                    if architecture_newer { "newer" } else { "older or same" }
                );

                tasks_newer || architecture_newer
            } else {
                println!("Could not get modification times for files");
                false
            }
        } else {
            println!("Could not get metadata for files");
            false
        };

        println!(
            "Decision to update TEST_STRATEGY.md: {}",
            if should_run_aider { "YES" } else { "NO" }
        );

        if should_run_aider {
            println!("Updating TEST_STRATEGY.md");
            let mut cmd = Command::new("aider");
            cmd.current_dir(&project_dir)
                .arg("--no-suggest-shell-commands")
                .arg("--yes-always")
                .arg("--message")
                .arg("Given the ARCHITECTURE.md, create or update TEST_STRATEGY.md with a comprehensive testing approach. Focus on:
1. Testing tools and frameworks we'll use for each layer (unit, integration, e2e)
2. Test automation principles and practices
3. CI/CD pipeline configuration and tools
4. Code coverage tools and minimum thresholds
5. Performance testing tools and benchmarking approach
6. Security testing tools and scanning strategy
7. Test data generation and management tools
8. Logging and monitoring tools for test results
9. TDD workflow and tooling setup
10. Testing standards and best practices to follow")
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
                }
            }
        }
    }
    println!("Tester thread shutting down cleanly");
}
