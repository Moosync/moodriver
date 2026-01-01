use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref GLOBAL_PROGRESS_BAR: Arc<Mutex<Option<ProgressBar>>> = Arc::new(Mutex::new(None));
}

pub async fn initialize_progress_bar(verbose: u8) {
    if verbose == 0 {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        pb.set_message("Waiting for extension...".yellow().to_string());

        {
            let mut pb_guard = GLOBAL_PROGRESS_BAR.lock().await;
            *pb_guard = Some(pb);
        }
    }
}

pub async fn log_ui_request(request_description: &str, response_value: &str) {
    let mut pb_guard = GLOBAL_PROGRESS_BAR.lock().await;
    if let Some(pb) = pb_guard.as_ref() {
        pb.finish_and_clear();
    }

    println!(
        "Responded to request {} with {}",
        request_description.blue(),
        response_value.green()
    );

    if pb_guard.is_some() {
        let new_pb = ProgressBar::new_spinner();
        new_pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner} {msg}")
                .unwrap(),
        );
        new_pb.enable_steady_tick(std::time::Duration::from_millis(100));
        new_pb.set_message("Waiting for extension...".yellow().to_string());
        *pb_guard = Some(new_pb);
    }
}

pub async fn finish_and_clear() {
    let pb_guard = GLOBAL_PROGRESS_BAR.lock().await;
    if let Some(pb) = pb_guard.as_ref() {
        pb.finish_and_clear();
    }
}
