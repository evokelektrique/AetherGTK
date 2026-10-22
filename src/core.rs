use serde_json::Value;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum Event {
    Log(String),
    State(State),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Starting,
    Connected,
    Stopping,
    Disconnected,
}

pub enum Control {
    Start(Vec<String>, Vec<(String, String)>, String),
    Stop,
}

static LOG_SENDER: OnceLock<Sender<String>> = OnceLock::new();
static MAX_LOG_LEVEL: AtomicU8 = AtomicU8::new(3);

struct UiLogger;

impl log::Log for UiLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level().to_level_filter() as u8 <= MAX_LOG_LEVEL.load(Ordering::Relaxed)
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if let Some(sender) = LOG_SENDER.get() {
            let _ = sender.send(format!("{} {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}

pub fn init_logger(sender: Sender<String>) {
    let _ = LOG_SENDER.set(sender);
    let _ = log::set_logger(&UiLogger);
    log::set_max_level(log::LevelFilter::Trace);
}

fn ffi_json(raw: *mut c_char) -> Result<Value, String> {
    if raw.is_null() {
        return Err("Aether returned an empty response".into());
    }
    let text = unsafe { CStr::from_ptr(raw) }
        .to_string_lossy()
        .into_owned();
    unsafe { aether::ffi::aether_string_free(raw) };
    serde_json::from_str(&text).map_err(|error| format!("Aether returned invalid JSON: {error}"))
}

fn check_reply(value: Value) -> Result<Value, String> {
    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(value
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Aether operation failed")
            .to_owned());
    }
    Ok(value)
}

fn start_job(arguments: &[String]) -> Result<u64, String> {
    let json = serde_json::to_string(arguments).map_err(|error| error.to_string())?;
    let json = CString::new(json).map_err(|error| error.to_string())?;
    let reply = ffi_json(unsafe { aether::ffi::aether_core_start(json.as_ptr()) })?;
    let reply = check_reply(reply)?;
    reply
        .get("job")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Aether did not return a job identifier".into())
}

fn poll_job(job: u64) -> Result<bool, String> {
    let reply = check_reply(ffi_json(aether::ffi::aether_job_poll(job))?)?;
    match reply.get("state").and_then(Value::as_str) {
        Some("running") => Ok(false),
        Some("done") => {
            if let Some(error) = reply.pointer("/result/error").and_then(Value::as_str) {
                Err(error.to_owned())
            } else {
                Ok(true)
            }
        }
        _ => Err("Aether returned an unknown job state".into()),
    }
}

fn set_log_level(level: &str) {
    let value = match level {
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    } as u8;
    MAX_LOG_LEVEL.store(value, Ordering::Relaxed);
}

fn environment_snapshot() -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    std::env::vars_os()
        .filter(|(key, _)| key.to_string_lossy().starts_with("AETHER_") || key == "RUST_LOG")
        .collect()
}

fn isolate_environment() -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    let old = environment_snapshot();
    for (key, _) in &old {
        unsafe { std::env::remove_var(key) };
    }
    old
}

fn restore_environment(old: Vec<(std::ffi::OsString, std::ffi::OsString)>) {
    for (key, _) in environment_snapshot() {
        unsafe { std::env::remove_var(key) };
    }
    for (key, value) in old {
        unsafe { std::env::set_var(key, value) };
    }
}

pub fn spawn_worker() -> (Sender<Control>, Receiver<Event>) {
    let (control_tx, control_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    let (log_tx, log_rx) = mpsc::channel();
    init_logger(log_tx);

    let events = event_tx.clone();
    thread::spawn(move || loop {
        let (arguments, environment, log_level) = match control_rx.recv() {
            Ok(Control::Start(arguments, environment, log_level)) => {
                (arguments, environment, log_level)
            }
            Ok(Control::Stop) => continue,
            Err(_) => break,
        };
        set_log_level(&log_level);
        let saved_environment = isolate_environment();
        for (key, value) in environment {
            unsafe { std::env::set_var(key, value) };
        }
        let job = match start_job(&arguments) {
            Ok(job) => {
                let _ = events.send(Event::State(State::Starting));
                job
            }
            Err(error) => {
                restore_environment(saved_environment);
                let _ = events.send(Event::Log(format!("ERROR {error}")));
                let _ = events.send(Event::State(State::Disconnected));
                continue;
            }
        };

        let mut stopping = false;
        let mut connected = false;
        loop {
            while let Ok(line) = log_rx.try_recv() {
                let lower = line.to_lowercase();
                if !connected
                    && (lower.contains("socks5 server listening on")
                        || lower.contains("psiphon is ready;"))
                {
                    connected = true;
                    let _ = events.send(Event::State(State::Connected));
                }
                let _ = events.send(Event::Log(line));
            }
            match control_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(Control::Stop) => {
                    if !stopping {
                        stopping = true;
                        let _ = events.send(Event::State(State::Stopping));
                        let _ = ffi_json(aether::ffi::aether_job_cancel(job));
                    }
                }
                Ok(Control::Start(_, _, _)) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    if !stopping {
                        stopping = true;
                        let _ = ffi_json(aether::ffi::aether_job_cancel(job));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            match poll_job(job) {
                Ok(true) => break,
                Err(error) => {
                    let _ = events.send(Event::Log(format!("ERROR {error}")));
                    break;
                }
                Ok(false) => {}
            }
        }
        let _ = ffi_json(aether::ffi::aether_job_free(job));
        restore_environment(saved_environment);
        let _ = events.send(Event::State(State::Disconnected));
    });

    (control_tx, event_rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_job_can_be_cancelled_polled_and_freed() {
        let job = start_job(&["--version".into()]).unwrap();
        check_reply(ffi_json(aether::ffi::aether_job_cancel(job)).unwrap()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let reply = check_reply(ffi_json(aether::ffi::aether_job_poll(job)).unwrap()).unwrap();
            if reply.get("state").and_then(Value::as_str) == Some("done") {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "Aether job did not finish after cancellation"
            );
            thread::sleep(Duration::from_millis(20));
        }
        check_reply(ffi_json(aether::ffi::aether_job_free(job)).unwrap()).unwrap();
    }
}
