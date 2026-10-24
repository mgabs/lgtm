//! Opt-in JSONL timing diagnostics. Never records command arguments, bodies, or credentials.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, OnceLock,
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static LOG: OnceLock<Mutex<File>> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(1);

pub fn enabled() -> bool {
    LOG.get().is_some()
}

/// Called explicitly by the CLI only when --debug is supplied.
/// Environment variables alone never enable diagnostics.
pub fn init(path: &std::path::Path) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let file = options.open(path)?;
    LOG.set(Mutex::new(file)).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "debug logging already initialized",
        )
    })
}

fn write(file: &Mutex<File>, value: serde_json::Value) {
    if let Ok(mut bytes) = serde_json::to_vec(&value) {
        bytes.push(b'\n');
        if let Ok(mut file) = file.lock() {
            let _ = file.write_all(&bytes);
        }
    }
}

/// Only structured categories and numeric codes are written; stderr may contain secrets.
pub fn command_failure(
    operation: &'static str,
    category: &'static str,
    exit_code: Option<i32>,
    http_status: Option<u16>,
) {
    if let Some(file) = LOG.get() {
        write(
            file,
            serde_json::json!({"event":"failure", "pid":std::process::id(), "operation":operation,
            "category":category, "exit_code":exit_code, "http_status":http_status}),
        );
    }
}

/// A start/end pair with a shared id; an unmatched start identifies a stalled operation.
pub struct Span {
    file: Option<&'static Mutex<File>>,
    id: u64,
    started: Option<Instant>,
    operation: &'static str,
    outcome: &'static str,
    counts: Option<serde_json::Map<String, serde_json::Value>>,
}

impl Span {
    pub fn new(operation: &'static str) -> Self {
        let file = LOG.get();
        let id = if file.is_some() {
            SEQ.fetch_add(1, Ordering::Relaxed)
        } else {
            0
        };
        if let Some(file) = file {
            write(
                file,
                serde_json::json!({"event":"start", "id":id, "pid":std::process::id(),
                "operation":operation, "unix_ms":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()}),
            );
        }
        Self {
            file,
            id,
            started: file.map(|_| Instant::now()),
            operation,
            outcome: "finished",
            counts: None,
        }
    }

    /// Numeric context only. Disabled diagnostics allocate nothing and read no clock.
    pub fn count(&mut self, name: &'static str, value: u64) {
        if self.file.is_some() {
            self.counts
                .get_or_insert_with(Default::default)
                .insert(name.into(), value.into());
        }
    }

    pub fn result<T, E>(&mut self, result: &Result<T, E>) {
        self.outcome = if result.is_ok() { "ok" } else { "error" };
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if let (Some(file), Some(started)) = (self.file, self.started) {
            let mut record = serde_json::json!({"event":"end", "id":self.id, "pid":std::process::id(),
                "operation":self.operation, "elapsed_ms":started.elapsed().as_secs_f64()*1000.0,
                "outcome":if std::thread::panicking() { "panic" } else { self.outcome }});
            if let Some(counts) = self.counts.take() {
                record["counts"] = counts.into();
            }
            write(file, record);
        }
    }
}

pub fn measure<T, E>(operation: &'static str, work: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let mut span = Span::new(operation);
    let result = work();
    span.result(&result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Separate process so logger initialization never changes the environment of other tests.
    #[test]
    fn diagnostic_child() {
        if std::env::var_os("LGTM_DEBUG_TEST_CHILD").is_none() {
            return;
        }
        if std::env::var_os("LGTM_DEBUG_TEST_ENABLE").is_some() {
            init(&std::path::PathBuf::from(
                std::env::var_os("LGTM_DEBUG_LOG").unwrap(),
            ))
            .unwrap();
        }
        if std::env::var_os("LGTM_DEBUG_TEST_COUNTS").is_some() {
            let mut span = Span::new("test.counts");
            span.count("files", 4);
            span.count("files", 10);
            assert_eq!(span.started.is_some(), enabled());
            assert_eq!(span.counts.is_some(), enabled());
            return;
        }
        let ok: Result<(), &str> = measure("test.ok", || Ok(()));
        let error: Result<(), &str> = measure("test.error", || Err("sentinel-secret-body"));
        assert!(ok.is_ok());
        assert!(error.is_err());
        command_failure("command.curl", "authentication", Some(22), Some(401));
    }

    #[test]
    fn environment_alone_does_not_enable_logging() {
        let path =
            std::env::temp_dir().join(format!("lgtm-debug-disabled-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "debug::tests::diagnostic_child"])
            .env("LGTM_DEBUG_TEST_CHILD", "1")
            .env("LGTM_DEBUG_TEST_COUNTS", "1")
            .env_remove("LGTM_DEBUG_TEST_ENABLE")
            .env("LGTM_DEBUG_LOG", &path)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(!path.exists());
    }

    #[test]
    fn numeric_context_is_attached_to_end_record() {
        let path =
            std::env::temp_dir().join(format!("lgtm-debug-counts-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "debug::tests::diagnostic_child"])
            .env("LGTM_DEBUG_TEST_CHILD", "1")
            .env("LGTM_DEBUG_TEST_COUNTS", "1")
            .env("LGTM_DEBUG_TEST_ENABLE", "1")
            .env("LGTM_DEBUG_LOG", &path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let contents = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(path);
        let records = contents
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["id"], records[1]["id"]);
        assert_eq!(records[1]["counts"], serde_json::json!({"files":10}));
    }

    #[test]
    fn writes_paired_timings_without_error_contents() {
        let path =
            std::env::temp_dir().join(format!("lgtm-debug-test-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "debug::tests::diagnostic_child"])
            .env("LGTM_DEBUG_TEST_CHILD", "1")
            .env("LGTM_DEBUG_TEST_ENABLE", "1")
            .env("LGTM_DEBUG_LOG", &path)
            .output()
            .unwrap();
        assert!(status.status.success());
        let contents = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(!contents.contains("sentinel-secret-body"));
        let records: Vec<serde_json::Value> = contents
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(records.len(), 5);
        for pair in records[..4].as_chunks::<2>().0 {
            assert_eq!(pair[0]["event"], "start");
            assert_eq!(pair[1]["event"], "end");
            assert_eq!(pair[0]["id"], pair[1]["id"]);
            assert_eq!(pair[0]["pid"], pair[1]["pid"]);
            assert_eq!(pair[0]["operation"], pair[1]["operation"]);
            assert!(pair[1]["elapsed_ms"].as_f64().unwrap() >= 0.0);
        }
        assert_eq!(records[1]["outcome"], "ok");
        assert_eq!(records[3]["outcome"], "error");
        assert_eq!(records[4]["event"], "failure");
        assert_eq!(records[4]["category"], "authentication");
        assert_eq!(records[4]["http_status"], 401);
    }
}
