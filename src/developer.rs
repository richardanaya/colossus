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
        let model = code_model.as_ref().expect("Code model should be set from CLI params");
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
            .arg("--no-suggest-shell-commands")
            .output()
            .await
            .expect("Failed to execute aider command");

        // Run make build after aider
        println!("Running make build...");
        let build_output = Command::new("make")
            .current_dir(&project_dir)
            .arg("build")
            .output()
            .await
            .expect("Failed to execute make build");

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            let stdout = String::from_utf8_lossy(&build_output.stdout);
            eprintln!(
                "Make build failed\nSTDERR:\n{}\nSTDOUT:\n{}",
                stderr, stdout
            );
            continue; // Skip testing if build failed
        } else {
            println!("Make build succeeded");
        }

        // Run make test after successful build
        println!("Running make test...");
        let test_output = Command::new("make")
            .current_dir(&project_dir)
            .arg("test")
            .output()
            .await
            .expect("Failed to execute make test");

        if !test_output.status.success() {
            let stderr = String::from_utf8_lossy(&test_output.stderr);
            let stdout = String::from_utf8_lossy(&test_output.stdout);
            eprintln!(
                "Make test failed\nSTDERR:\n{}\nSTDOUT:\n{}",
                stderr, stdout
            );
        } else {
            println!("Make test succeeded");
        }
    }
    println!("Developer thread shutting down cleanly");
}
