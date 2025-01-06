use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
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

        println!("DEVELOPING!!");
    }
    println!("Developer thread shutting down cleanly");
}
