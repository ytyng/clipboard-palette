//! Lifecycle logging.
//!
//! The app is often launched by a script or an AI agent that only sees stdout.
//! Before this module existed, a run where the window never appeared produced
//! exactly the same output as a successful run, so the caller had to shell out
//! to `lsappinfo` to tell them apart. Every line here starts with [`LOG_PREFIX`]
//! so a caller can decide the outcome by reading stdout alone.

use std::io::Write;
#[cfg(all(desktop, unix))]
use std::path::Path;
use tauri::{AppHandle, Manager, Runtime, WindowEvent};

/// Prefix shared by every lifecycle line, so a caller can select them with a
/// single grep.
pub const LOG_PREFIX: &str = "[lifecycle]";

/// Write one lifecycle line to stdout, ignoring write errors.
///
/// `println!` panics when stdout is gone (a caller that stopped reading, for
/// example). A diagnostic line must never be what takes the app down, and the
/// panic hook logs through here as well, so a panic raised here would abort the
/// process instead of reporting the original one.
pub fn log(message: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{} {}", LOG_PREFIX, message);
    let _ = stdout.flush();
}

/// Report this process id and any other process running a program of the same
/// name.
///
/// A left over instance (for example an orphan from an earlier run) can keep a
/// new window from showing up, and the caller has no way to notice that from
/// the outside. Detection is best effort: it never fails the startup, and an
/// unknown result is logged as such rather than as "none".
pub fn report_instances() {
    log(&format!("pid={}", std::process::id()));
    log(&other_instances_line());
}

/// Log that the main window object was created. This says the window was built,
/// not that it is on screen; [`log_window_state`] reports that once the event
/// loop is up.
pub fn log_window_created(label: &str) {
    log(&format!("window created: label={}", label));
}

/// Log that the app could not finish its startup. Printed to stdout on purpose:
/// the panic that follows only reaches stderr, and a caller reading stdout has
/// to be able to see that the run failed.
pub fn log_startup_failed(error: &str) {
    log(&format!("startup failed: {}", error));
}

/// Mirror a panic to stdout before the usual stderr report.
///
/// A panic is how the app dies when the GUI backend itself will not come up,
/// which happens before `setup` runs and so before any other failure line could
/// be printed. Without this, such a run only differs from a good one by what is
/// missing from stdout.
pub fn install_panic_logger() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown");
        log(&format!("exit: reason=panic message={}", payload));
        default_hook(info);
    }));
}

/// Report whether the window actually made it on screen, and end with a verdict
/// line that a caller can match without parsing the details.
pub fn log_window_state<R: Runtime>(app: &AppHandle<R>, label: &str) {
    let Some(window) = app.get_webview_window(label) else {
        log(&format!("window missing: label={}", label));
        log("startup incomplete: window was not found");
        return;
    };
    let visible = window.is_visible();
    let minimized = window.is_minimized();
    let position = window
        .outer_position()
        .map(|p| format!("{},{}", p.x, p.y))
        .unwrap_or_else(|_| "unknown".to_string());
    let size = window
        .outer_size()
        .map(|s| format!("{}x{}", s.width, s.height))
        .unwrap_or_else(|_| "unknown".to_string());
    log(&format!(
        "window state: label={} visible={} minimized={} focused={} position={} size={}",
        label,
        flag(&visible),
        flag(&minimized),
        flag(&window.is_focused()),
        position,
        size
    ));
    // A minimized window is not on screen either, so it does not count as a
    // good startup even though it is "visible" as far as the window flag goes.
    // Anything short of a confirmed yes to both is reported as incomplete: a
    // false "it started" is exactly what this log exists to stop
    match (&visible, &minimized) {
        (Ok(true), Ok(false)) => log("startup ok: window is visible"),
        (Ok(true), Ok(true)) => log("startup incomplete: window is minimized"),
        (Ok(true), Err(e)) => log(&format!(
            "startup incomplete: window is visible but its minimized state is unknown: {}",
            e
        )),
        (Ok(false), _) => log("startup incomplete: window is not visible"),
        (Err(e), _) => log(&format!(
            "startup incomplete: window visibility is unknown: {}",
            e
        )),
    }
}

/// Log the window events that tell the exit reason apart. Focus, resize and
/// move events are left out on purpose; they say nothing about the outcome and
/// would bury the lines that do.
pub fn log_window_event(label: &str, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested { .. } => {
            log(&format!("window close requested: label={}", label))
        }
        WindowEvent::Destroyed => log(&format!("window destroyed: label={}", label)),
        _ => {}
    }
}

/// Log that the event loop is winding down. `code` is the exit code the caller
/// asked for, if any.
pub fn log_exit_requested(code: Option<i32>) {
    match code {
        Some(code) => log(&format!("exit requested: code={}", code)),
        None => log("exit requested: code=none"),
    }
}

/// Log a normal end of the event loop. Reaching this means the app shut down on
/// its own terms; a run that stops without any `exit:` line was killed.
pub fn log_exit() {
    log("exit: reason=normal");
}

/// Print an exit line when the process is asked to stop by a signal, then let
/// the default action run so the exit status stays the usual 128+signum.
///
/// Without this, `kill <pid>` produces no output at all and cannot be told
/// apart from a crash. SIGKILL still cannot be caught, which is why the absence
/// of any `exit:` line has to be read as "terminated without cleanup".
#[cfg(all(desktop, unix))]
pub fn install_signal_logger() {
    let handler = on_terminating_signal as *const () as libc::sighandler_t;
    for signum in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
        // SAFETY: `handler` is a valid `extern "C" fn(c_int)` cast the way
        // `sighandler_t` expects, `signum` is one of the constants above, and
        // the handler itself only calls async-signal-safe functions. Replacing
        // an existing disposition is the intent here, and the previous one is
        // not needed because the handler restores SIG_DFL before re-raising
        let previous = unsafe { libc::signal(signum, handler) };
        if previous == libc::SIG_ERR {
            // Not fatal: only the exit line for this signal is lost, so say so
            // rather than letting a later run look like it was killed outright
            log(&format!(
                "signal logger not installed: signal={} errno={}",
                signum,
                std::io::Error::last_os_error()
            ));
        }
    }
}

#[cfg(not(all(desktop, unix)))]
pub fn install_signal_logger() {}

/// Signal handler. Only `write(2)`, `signal(2)` and `raise(3)` are used, all of
/// which are async-signal-safe, so the message is a fixed byte string rather
/// than something built with `format!`.
///
/// `write(2)` on a blocking stdout could in theory stall here, but the app
/// writes well under a pipe buffer over its whole life, so the buffer cannot be
/// full of its own output.
#[cfg(all(desktop, unix))]
extern "C" fn on_terminating_signal(signum: libc::c_int) {
    let message: &[u8] = match signum {
        libc::SIGINT => b"[lifecycle] exit: reason=signal signal=SIGINT\n",
        libc::SIGTERM => b"[lifecycle] exit: reason=signal signal=SIGTERM\n",
        libc::SIGHUP => b"[lifecycle] exit: reason=signal signal=SIGHUP\n",
        _ => b"[lifecycle] exit: reason=signal\n",
    };
    // SAFETY: writing a static buffer to stdout, then restoring the default
    // action and re-raising so the process dies the way it would have anyway.
    unsafe {
        libc::write(
            libc::STDOUT_FILENO,
            message.as_ptr().cast(),
            message.len() as libc::size_t,
        );
        libc::signal(signum, libc::SIG_DFL);
        libc::raise(signum);
    }
}

fn flag(result: &tauri::Result<bool>) -> &'static str {
    match result {
        Ok(true) => "true",
        Ok(false) => "false",
        Err(_) => "unknown",
    }
}

/// Stand-in for the platforms the check is not implemented for. The line is
/// always printed so that a caller never has to tell a missing line apart from
/// a check that found nothing.
#[cfg(not(all(desktop, unix)))]
fn other_instances_line() -> String {
    "other instances: unknown (the check is only implemented for unix desktops)".to_string()
}

/// Build the "other instances" line by asking `ps` for the running processes.
///
/// `ps` is used instead of a process listing crate because the same invocation
/// works on macOS and Linux and keeps the dependency list unchanged.
#[cfg(all(desktop, unix))]
fn other_instances_line() -> String {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => return format!("other instances: unknown (current_exe failed: {})", e),
    };
    let Some(exe_name) = exe.file_name().and_then(|name| name.to_str()) else {
        return "other instances: unknown (executable name is not valid UTF-8)".to_string();
    };
    let output = match std::process::Command::new("ps")
        .args(["-A", "-o", "pid=,comm="])
        .output()
    {
        Ok(output) if output.status.success() => output.stdout,
        Ok(output) => {
            return format!(
                "other instances: unknown (ps exited with {})",
                output.status
            )
        }
        Err(e) => return format!("other instances: unknown (ps failed to run: {})", e),
    };
    let pids = other_pids_from_ps(
        &String::from_utf8_lossy(&output),
        exe_name,
        std::process::id(),
    );
    if pids.is_empty() {
        return "other instances: none".to_string();
    }
    let listed = pids
        .iter()
        .map(|pid| format!("pid={}", pid))
        .collect::<Vec<_>>()
        .join(" ");
    format!("other instances: {}", listed)
}

/// Pick the pids of other processes running `exe_name` out of the output of
/// `ps -A -o pid=,comm=`.
#[cfg(all(desktop, unix))]
fn other_pids_from_ps(output: &str, exe_name: &str, self_pid: u32) -> Vec<u32> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, command) = line.split_once(char::is_whitespace)?;
            let pid = pid.parse::<u32>().ok()?;
            if pid == self_pid || !is_same_program(exe_name, command.trim()) {
                return None;
            }
            Some(pid)
        })
        .collect()
}

/// Whether a `comm` column names the same program as `exe_name`.
///
/// macOS reports the full executable path, Linux reports the base name cut down
/// to 15 characters, so a truncated name that prefixes ours counts as a match.
///
/// The comparison is on the base name rather than the whole path on purpose: an
/// installed build and a `cargo run` build sit at different paths but are both
/// instances of this app, and either one can be the leftover that has to be
/// reported. The cost is that an unrelated program of the same name, or on
/// Linux one whose first 15 characters match, is reported too — which is why
/// the line names the pids instead of claiming anything about them.
#[cfg(all(desktop, unix))]
fn is_same_program(exe_name: &str, command: &str) -> bool {
    let name = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command);
    name == exe_name || (name.len() == 15 && exe_name.starts_with(name))
}

#[cfg(all(test, desktop, unix))]
mod tests {
    use super::{is_same_program, other_pids_from_ps};

    #[test]
    fn matches_the_full_path_reported_by_macos() {
        assert!(is_same_program(
            "clipboard-palette",
            "/Applications/clipboard-palette.app/Contents/MacOS/clipboard-palette"
        ));
    }

    #[test]
    fn matches_the_name_truncated_by_linux() {
        assert!(is_same_program("clipboard-palette", "clipboard-palet"));
    }

    #[test]
    fn does_not_match_another_program() {
        assert!(!is_same_program("clipboard-palette", "node"));
        assert!(!is_same_program("clipboard-palette", "/usr/bin/pbcopy"));
    }

    #[test]
    fn collects_other_pids_and_skips_this_process() {
        let output = "    1 /sbin/launchd\n 4242 /Applications/clipboard-palette.app/Contents/MacOS/clipboard-palette\n 4243 clipboard-palet\n 4244 node\n";
        assert_eq!(
            other_pids_from_ps(output, "clipboard-palette", 4243),
            vec![4242]
        );
    }

    #[test]
    fn reports_nothing_when_only_this_process_runs() {
        let output = " 4242 clipboard-palette\n 4244 node\n";
        assert!(other_pids_from_ps(output, "clipboard-palette", 4242).is_empty());
    }

    #[test]
    fn ignores_the_header_of_a_ps_run_without_the_trailing_equals() {
        let output = "  PID COMMAND\n 4242 clipboard-palette\n";
        assert_eq!(
            other_pids_from_ps(output, "clipboard-palette", 1),
            vec![4242]
        );
    }
}
