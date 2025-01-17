use std::sync::Arc;

async fn handle_make_test(project_dir: &str, model: &str) -> bool {
    println!("Running make test...");
    let test_output = Command::new("make")
        .current_dir(project_dir)
        .arg("test")
        .output()
        .await
        .expect("Failed to execute make test");

    if !test_output.status.success() {
        let stderr = String::from_utf8_lossy(&test_output.stderr);
        let stdout = String::from_utf8_lossy(&test_output.stdout);

        // Send test error to aider to fix
        println!("🔧 Attempting to fix test failures with aider...");
        // give me the complete command
        let fix_message = format!("Fix these test failures:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
        let _output = Command::new("aider")
            .current_dir(project_dir)
            .arg("--model")
            .arg(model)
            .arg("--message")
            .arg(&fix_message)
            .arg("--load")
            .arg("CONTEXT.md")
            .arg("--yes-always")
            .arg("--no-suggest-shell-commands")
            .output()
            .await
            .expect("Failed to execute aider command");
        
        println!("✨ Aider finished attempting test fix");
        false
    } else {
        println!("Make test succeeded");
        true
    }
}
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tokio::process::Command;
use crate::{AppStateWithDir, ActivityMode};

async fn handle_make_build(project_dir: &str, model: &str) -> bool {
    // Run make build
    println!("Running make build...");
    let build_output = Command::new("make")
        .current_dir(project_dir)
        .arg("build")
        .output()
        .await
        .expect("Failed to execute make build");

    if !build_output.status.success() {
        let stderr = String::from_utf8_lossy(&build_output.stderr);
        let stdout = String::from_utf8_lossy(&build_output.stdout);

        // Send build error to aider to fix
        println!("🔧 Attempting to fix build error with aider...");
        let fix_message = format!("Fix this build error:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
        let _output = Command::new("aider")
            .current_dir(project_dir)
            .arg("--model")
            .arg(model)
            .arg("--message")
            .arg(&fix_message)
            .arg("--load")
            .arg("CONTEXT.md")
            .arg("--yes-always")
            .arg("--no-suggest-shell-commands")
            .output()
            .await
            .expect("Failed to execute aider command");
        
        println!("✨ Aider finished attempting build fix");
        false
    } else {
        println!("Make build succeeded");
        true
    }
}

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

        // Check activity mode quickly
        let should_continue = {
            let mode = state_with_dir.activity_mode.lock().await;
            matches!(*mode, ActivityMode::Developing)
        };
        
        if !should_continue {
            // Check if we're in error state
            let is_error = {
                let mode = state_with_dir.activity_mode.lock().await;
                matches!(*mode, ActivityMode::ErrorNeedsHuman)
            };
            
            if is_error {
                println!("⚠️  Development halted - human intervention required to fix critical errors!");
            }
            continue;
        }

        // Run aider command
        // Get the code model from state
        let code_model = state_with_dir.code_model.clone();

        println!("Running aider in directory: {}", project_dir);
        let model = code_model.as_ref().expect("Code model should be set from CLI params");
        let _output = Command::new("aider")
            .current_dir(&project_dir)
            .arg("--model")
            .arg(model)
            .arg("--message")
            
            .arg("find the first UNCOMPLETED task (one without a checkmark ✓) in TASKS.md, working in strict numerical order from top to bottom, implement it, and create some way to test it")
            .arg("--load")
            .arg("CONTEXT.md")
            .arg("--yes-always")
            .arg("--no-suggest-shell-commands")
            .output()
            .await
            .expect("Failed to execute aider command");
    
        println!("✨ Aider finished task assignment");

        // Run make build after aider with retries
        let mut build_success = false;
        for attempt in 1..=5 {
            println!("Build attempt {} of 5", attempt);
            if handle_make_build(&project_dir, model).await {
                build_success = true;
                break;
            }
            if attempt == 5 {
                println!("SOMETHING IS SERIOUSLY WRONG - Build failed after 5 attempts");
                let mut mode = state_with_dir.activity_mode.lock().await;
                *mode = ActivityMode::ErrorNeedsHuman;
            }
        }
        if !build_success {
            continue; // Restart loop after all attempts failed
        }

        // Run make test after successful build with retries
        let mut test_success = false;
        for attempt in 1..=5 {
            println!("Test attempt {} of 5", attempt);
            if handle_make_test(&project_dir, model).await {
                test_success = true;
                break;
            }
            if attempt == 5 {
                println!("SOMETHING IS SERIOUSLY WRONG - Tests failed after 5 attempts");
                let mut mode = state_with_dir.activity_mode.lock().await;
                *mode = ActivityMode::ErrorNeedsHuman;
            }
        }
        if !test_success {
            continue; // Restart loop after all attempts failed
        }
            
            // Tell aider to mark the completed task
            println!("Marking off task complete!");
            let output = Command::new("aider")
                .current_dir(&project_dir)
                .arg("--model")
                .arg(model)
                .arg("--message")
                .arg("Mark the task we just completed in TASKS.md as done")
                .arg("--load")
                .arg("CONTEXT.md")
                .arg("--yes-always")
                .arg("--no-suggest-shell-commands")
                .output()
                .await
                .expect("Failed to execute aider command");
            
            // Print aider's response
            println!("Aider response:");
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    println!("Developer thread shutting down cleanly");
}
