use cleanserve_core::{docker_available, ensure_docker, StackManager};
use cleanserve_core::RuntimeTelemetry;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

pub const MENU_ITEMS: &[&str] = &[
    "Stack: start (docker compose up)",
    "Stack: stop",
    "Stack: show enabled services",
    "Docker: check prerequisites",
    "Copy selected request line",
    "Copy server URL",
    "Close menu (Esc)",
];

pub fn run_menu_action(
    index: usize,
    project_root: &Path,
    url: &str,
    selected_request: Option<&str>,
    telemetry: &Arc<RuntimeTelemetry>,
) -> Option<String> {
    if index == MENU_ITEMS.len() - 1 {
        return None;
    }
    let msg = match index {
        0 => stack_start(project_root, telemetry),
        1 => stack_stop(project_root, telemetry),
        2 => stack_enabled(project_root, telemetry),
        3 => docker_check(telemetry),
        4 => copy_text(selected_request.unwrap_or("(nothing selected)")),
        5 => copy_text(url),
        _ => return None,
    };
    Some(msg)
}

fn stack_start(project_root: &Path, telemetry: &Arc<RuntimeTelemetry>) -> String {
    match StackManager::new(project_root) {
        Ok(stack) => match stack.start() {
            Ok(()) => {
                telemetry.push_log("Stack services started");
                "Stack started".into()
            }
            Err(e) => format!("Stack start failed: {}", e),
        },
        Err(e) => format!("Stack error: {}", e),
    }
}

fn stack_stop(project_root: &Path, telemetry: &Arc<RuntimeTelemetry>) -> String {
    match StackManager::new(project_root) {
        Ok(stack) => match stack.stop() {
            Ok(()) => {
                telemetry.push_log("Stack services stopped");
                "Stack stopped".into()
            }
            Err(e) => format!("Stack stop failed: {}", e),
        },
        Err(e) => format!("Stack error: {}", e),
    }
}

fn stack_enabled(project_root: &Path, telemetry: &Arc<RuntimeTelemetry>) -> String {
    match StackManager::new(project_root) {
        Ok(stack) => match stack.load_state() {
            Ok(state) => {
                if state.enabled.is_empty() {
                    let m = "Stack: no services enabled (cleanserve stack add mysql)".to_string();
                    telemetry.push_log(&m);
                    m
                } else {
                    let m = format!("Stack enabled: {}", state.enabled.join(", "));
                    telemetry.push_log(&m);
                    m
                }
            }
            Err(e) => format!("Stack state: {}", e),
        },
        Err(e) => format!("Stack error: {}", e),
    }
}

fn docker_check(telemetry: &Arc<RuntimeTelemetry>) -> String {
    if docker_available() {
        if ensure_docker().is_ok() {
            telemetry.push_log("Docker OK");
            "Docker is available".into()
        } else {
            let m = "Docker installed but not reachable";
            telemetry.push_log(m);
            m.into()
        }
    } else {
        let m = "Docker missing — run: cleanserve setup docker";
        telemetry.push_log(m);
        m.into()
    }
}

fn copy_text(text: &str) -> String {
    match set_clipboard(text) {
        Ok(()) => format!("Copied to clipboard ({} chars)", text.len()),
        Err(e) => format!("Clipboard failed: {} (install wl-clipboard or xclip)", e),
    }
}

pub fn copy_line(line: &str) -> String {
    copy_text(line)
}

/// Open `url` in the default browser.
pub fn open_url(url: &str) -> String {
    #[cfg(target_os = "macos")]
    let spawned = Command::new("open").arg(url).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    #[cfg(windows)]
    let spawned = Command::new("cmd").args(["/C", "start", "", url]).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    #[cfg(not(any(target_os = "macos", windows)))]
    let spawned = Command::new("xdg-open").arg(url).stdout(Stdio::null()).stderr(Stdio::null()).spawn();

    match spawned {
        Ok(_) => format!("Opening {}", url),
        Err(e) => format!("Could not open browser: {}", e),
    }
}

fn set_clipboard(text: &str) -> Result<(), String> {
    if try_clip_command("wl-copy", &[], text).is_ok() {
        return Ok(());
    }
    try_clip_command("xclip", &["-selection", "clipboard"], text)
}

fn try_clip_command(bin: &str, args: &[&str], text: &str) -> Result<(), String> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    child.wait().map_err(|e| e.to_string())?;
    Ok(())
}
