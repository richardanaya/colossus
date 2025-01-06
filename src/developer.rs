use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tokio::process::Command;
use crate::{AppStateWithDir, ActivityMode};

pub async fn developer_loop(
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

        // Check activity mode - only run in Developing mode
        {
            let mode = state_with_dir.activity_mode.lock().await;
            if !matches!(*mode, ActivityMode::Developing) {
                println!("Not in developing mode - skipping developer work");
                continue;
            }
        }

        // Run aider command
        // Get the code model from state
        let code_model = {
            let state = state_with_dir.state.lock().await;
            state.code_model.clone()
        };

        let output = Command::new("aider")
            .arg("--model")
            .arg(&code_model)
            .arg("--message")
            .arg("given the first important task at the top of the list, implement it, and create some way to test it")
            .arg("--load")
            .arg("CONTEXT.md")
            .arg("--yes-always")
            .output()
            .await
            .expect("Failed to execute aider command");

        // Print command output
        println!("Aider command output:");
        println!("Status: {}", output.status);
        println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    println!("Developer thread shutting down cleanly");
}
