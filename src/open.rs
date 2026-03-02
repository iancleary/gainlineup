use crate::file_operations;
use std::process;

pub fn browser(url: &str) {
    // 1. Determine the OS-specific command and arguments
    let (cmd, args) = if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", "start", "", url])
    } else if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else {
        ("xdg-open", vec![url])
    };

    tracing::debug!(command = cmd, url, "Opening browser");

    // 2. Spawn the process
    match process::Command::new(cmd).args(&args).spawn() {
        Ok(_) => tracing::info!("Opening in browser: {}", url),
        Err(e) => tracing::error!("Failed to open {} in default browser: {}", url, e),
    }
}

pub fn plot(file_path: String) {
    let html_file_url = file_operations::get_file_url(&file_path);

    tracing::info!("Plot available at: {}", html_file_url);

    // if not part of cargo test, open the created file
    if cfg!(test) {
        // pass
    } else {
        tracing::debug!("Attempting to open plot in default browser...");
        browser(&html_file_url);
    }
}
