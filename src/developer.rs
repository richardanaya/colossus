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
    let mut interval = time::interval(Duration::from_secs(30));

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
        let code_model = state_with_dir.code_model.clone();

        println!("Running aider in directory: {}", project_dir);
        let model = code_model.as_ref().unwrap_or(&"gpt-4".to_string());
        println!("Command: aider --model {} --message 'given the first important task at the top of the list, implement it, and create some way to test it' --load CONTEXT.md --yes-always", model);

        let output = Command::new("aider")
            .current_dir(&project_dir)
            .arg("--model")
            .arg(model)
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
