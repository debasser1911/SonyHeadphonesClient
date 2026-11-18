use std::backtrace::Backtrace;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Installs a process-wide panic hook that writes a crash log file before
/// delegating to the default panic hook.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::force_capture();
        let report = format_panic_report(info, &backtrace);
        match write_crash_log_to(&crash_log_dir(), "crash", &report) {
            Ok(path) => eprintln!("Crash log written to: {}", path.display()),
            Err(err) => eprintln!("Failed to write crash log: {err}"),
        }
        default_hook(info);
    }));
}

/// Writes a crash log for a fatal, non-panic error (such as GUI startup failure).
pub fn log_fatal_error(context: &str, err: &dyn std::fmt::Display) {
    let backtrace = Backtrace::force_capture();
    let report = format_error_report(context, err, &backtrace);
    match write_crash_log_to(&crash_log_dir(), "crash", &report) {
        Ok(path) => eprintln!("Crash log written to: {}", path.display()),
        Err(write_err) => eprintln!("Failed to write crash log: {write_err}"),
    }
}

/// Returns the directory where crash logs are stored.
pub fn crash_log_dir() -> PathBuf {
    if let Some(dir) = non_empty_env("SONY_HEADPHONES_CRASH_LOG_DIR") {
        return PathBuf::from(dir);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(base) = non_empty_env("LOCALAPPDATA").or_else(|| non_empty_env("APPDATA")) {
            return PathBuf::from(base)
                .join("SonyHeadphonesClient")
                .join("crash-logs");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = non_empty_env("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Logs")
                .join("SonyHeadphonesClient");
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(state_home) = non_empty_env("XDG_STATE_HOME") {
            return PathBuf::from(state_home)
                .join("sony-headphones-client")
                .join("crash-logs");
        }
        if let Some(home) = non_empty_env("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("sony-headphones-client")
                .join("crash-logs");
        }
    }

    std::env::temp_dir().join("sony-headphones-client-crash-logs")
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn format_header(out: &mut String, kind: &str) {
    let _ = writeln!(
        out,
        "{kind}: {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(out, "timestamp_unix: {}", unix_timestamp_secs());
    let _ = writeln!(
        out,
        "os: {} ({})",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let thread = std::thread::current();
    let _ = writeln!(out, "thread: {}", thread.name().unwrap_or("<unnamed>"));
}

fn format_panic_report(info: &PanicHookInfo<'_>, backtrace: &Backtrace) -> String {
    let mut out = String::new();
    format_header(&mut out, "panic");

    let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
        *s
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.as_str()
    } else {
        "<non-string panic payload>"
    };
    let _ = writeln!(out, "message: {payload}");

    if let Some(loc) = info.location() {
        let _ = writeln!(
            out,
            "location: {}:{}:{}",
            loc.file(),
            loc.line(),
            loc.column()
        );
    } else {
        let _ = writeln!(out, "location: <unknown>");
    }

    let _ = writeln!(out, "\nbacktrace:\n{backtrace}");
    out
}

fn format_error_report(
    context: &str,
    err: &dyn std::fmt::Display,
    backtrace: &Backtrace,
) -> String {
    let mut out = String::new();
    format_header(&mut out, "fatal_error");
    let _ = writeln!(out, "context: {context}");
    let _ = writeln!(out, "error: {err}");
    let _ = writeln!(out, "\nbacktrace:\n{backtrace}");
    out
}

fn write_crash_log_to(dir: &Path, prefix: &str, contents: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let ts = unix_timestamp_secs();
    let pid = std::process::id();
    let seq = LOG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let filename = format!("{prefix}-{ts}-{pid}-{seq}.log");
    let path = dir.join(filename);
    fs::write(&path, contents)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_unique_crash_logs_with_metadata() {
        let dir = std::env::temp_dir().join(format!(
            "sony-headphones-crash-log-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);

        let bt = Backtrace::disabled();
        let report1 = format_error_report("gui_launch", &"window init failed", &bt);
        let report2 = format_error_report("gui_launch", &"second error", &bt);

        let path1 = write_crash_log_to(&dir, "crash", &report1).expect("write first log");
        let path2 = write_crash_log_to(&dir, "crash", &report2).expect("write second log");

        assert_ne!(path1, path2);
        let body = fs::read_to_string(&path1).expect("read first log");
        assert!(body.contains("fatal_error: sony-headphones-client"));
        assert!(body.contains("context: gui_launch"));
        assert!(body.contains("error: window init failed"));

        let _ = fs::remove_dir_all(&dir);
    }
}
