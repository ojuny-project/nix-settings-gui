use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::flake_inputs::FlakeInput;
use crate::flake_lock;
use crate::flake_ref::{self, FileTransport, FlakeRef, GitTransport, MercurialTransport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputState {
    UpToDate,
    UpdateAvailable,
    NotLocked,
    Skipped(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputStatus {
    pub current_rev: Option<String>,
    pub current_date: Option<i64>,
    pub remote_rev: Option<String>,
    pub remote_date: Option<i64>,
    pub state: InputState,
}

impl InputStatus {
    fn skipped(reason: impl Into<String>) -> Self {
        Self {
            current_rev: None,
            current_date: None,
            remote_rev: None,
            remote_date: None,
            state: InputState::Skipped(reason.into()),
        }
    }
}

/// How long one input check may wait for `nix flake metadata`.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(20);
/// How long one `nix flake update` invocation may run.
pub const UPDATE_TIMEOUT: Duration = Duration::from_secs(120);

/// Compare the locked revision of one direct input with the revision its URL
/// currently resolves to.
#[allow(dead_code)]
pub fn check(base: &str, input: &FlakeInput) -> Result<InputStatus, String> {
    check_with(base, input, CHECK_TIMEOUT, &AtomicBool::new(false))
}

/// `check` with an explicit per-input timeout and a cancellation flag.
pub fn check_with(
    base: &str,
    input: &FlakeInput,
    limit: Duration,
    cancel: &AtomicBool,
) -> Result<InputStatus, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("update check cancelled".to_string());
    }
    let Some(url) = input
        .url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
    else {
        return Ok(InputStatus::skipped("no URL declared"));
    };
    if let Ok(reference) = flake_ref::parse(url) {
        if is_local(&reference) {
            return Ok(InputStatus::skipped("local input"));
        }
    }

    let current = flake_lock::read_input_in(Path::new(base), &input.name);
    let (remote_rev, remote_date) = remote_locked(base, url, limit, cancel)?;
    let Some(remote_rev) = remote_rev else {
        return Ok(InputStatus::skipped("latest revision unavailable"));
    };
    let current_rev = current.as_ref().and_then(|locked| locked.rev.clone());
    let current_date = current.as_ref().and_then(|locked| locked.last_modified);
    let state = match &current_rev {
        Some(current_rev) if current_rev == &remote_rev => InputState::UpToDate,
        Some(_) => InputState::UpdateAvailable,
        None => InputState::NotLocked,
    };

    Ok(InputStatus {
        current_rev,
        current_date,
        remote_rev: Some(remote_rev),
        remote_date,
        state,
    })
}

/// Update exactly one named input.
pub fn update(base: &str, name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("flake input name is empty".to_string());
    }
    run_update(base, &[name])
}

/// Update a list of named inputs. Nixpkgs is deliberately rejected here so the
/// bulk action can never move it.
pub fn update_all(base: &str, names: &[String]) -> Result<usize, String> {
    let names: Vec<&str> = names
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .collect();
    if names.is_empty() {
        return Err("no flake inputs selected".to_string());
    }
    if names.iter().any(|name| crate::nixpkgs::is_nixpkgs(name)) {
        return Err("update all must not include nixpkgs".to_string());
    }
    run_update(base, &names)?;
    Ok(names.len())
}

fn run_update(base: &str, names: &[&str]) -> Result<(), String> {
    if base.trim().is_empty() {
        return Err("no flake directory is open".to_string());
    }
    if names.is_empty() {
        return Err("no flake inputs selected".to_string());
    }
    let mut command = Command::new("nix");
    command
        .args(["flake", "update"])
        .args(names)
        .current_dir(base);
    let output = run_with_timeout(
        command,
        "nix flake update",
        UPDATE_TIMEOUT,
        &AtomicBool::new(false),
    )?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .or_else(|| stdout.lines().find(|line| !line.trim().is_empty()))
        .unwrap_or("nix flake update failed");
    Err(detail.trim().to_string())
}

fn remote_locked(
    base: &str,
    url: &str,
    limit: Duration,
    cancel: &AtomicBool,
) -> Result<(Option<String>, Option<i64>), String> {
    let mut command = Command::new("nix");
    command.args(["flake", "metadata", "--json", "--no-write-lock-file", url]);
    if !base.trim().is_empty() {
        command.current_dir(base);
    }
    let output = run_with_timeout(command, "nix flake metadata", limit, cancel)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("nix flake metadata failed");
        return Err(detail.trim().to_string());
    }
    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("cannot parse nix flake metadata output: {e}"))?;
    Ok(metadata_locked(&value))
}

fn metadata_locked(value: &Value) -> (Option<String>, Option<i64>) {
    let locked = value.get("locked");
    let rev = locked
        .and_then(|locked| locked.get("rev"))
        .and_then(Value::as_str)
        .or_else(|| value.get("revision").and_then(Value::as_str))
        .map(str::to_string);
    let date = locked
        .and_then(|locked| locked.get("lastModified"))
        .and_then(Value::as_i64)
        .or_else(|| value.get("lastModified").and_then(Value::as_i64));
    (rev, date)
}

/// Run a command with a hard deadline and a cancellation flag. The child is
/// killed when either fires, so a network stall cannot leave the check busy.
fn run_with_timeout(
    mut command: Command,
    label: &str,
    limit: Duration,
    cancel: &AtomicBool,
) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot run {label}: {error}"))?;
    let mut out_reader = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = pipe.read_to_end(&mut buffer);
            buffer
        })
    });
    let mut err_reader = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = pipe.read_to_end(&mut buffer);
            buffer
        })
    });
    let deadline = Instant::now() + limit;
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            command_abort(&mut child, &mut out_reader, &mut err_reader);
            return Err(format!("{label} cancelled"));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                command_abort(&mut child, &mut out_reader, &mut err_reader);
                return Err(format!("cannot wait for {label}: {error}"));
            }
        }
        if Instant::now() >= deadline {
            command_abort(&mut child, &mut out_reader, &mut err_reader);
            return Err(format!("{label} timed out after {}s", limit.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let stdout = join_reader(out_reader.take()).unwrap_or_default();
    let stderr = join_reader(err_reader.take()).unwrap_or_default();
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Kill the child and detach its pipe readers, so an abort returns at once even
/// when a grandchild keeps a pipe open.
fn command_abort(
    child: &mut std::process::Child,
    out_reader: &mut Option<std::thread::JoinHandle<Vec<u8>>>,
    err_reader: &mut Option<std::thread::JoinHandle<Vec<u8>>>,
) {
    let _ = child.kill();
    let _ = child.wait();
    drop(out_reader.take());
    drop(err_reader.take());
}

fn join_reader(handle: Option<std::thread::JoinHandle<Vec<u8>>>) -> Option<Vec<u8>> {
    handle.and_then(|handle| handle.join().ok())
}

fn is_local(reference: &FlakeRef) -> bool {
    matches!(
        reference,
        FlakeRef::Path { .. }
            | FlakeRef::Git {
                transport: GitTransport::File,
                ..
            }
            | FlakeRef::Mercurial {
                transport: MercurialTransport::File,
                ..
            }
            | FlakeRef::Tarball {
                transport: FileTransport::File,
                ..
            }
            | FlakeRef::File {
                transport: FileTransport::File,
                ..
            }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_locked_metadata_with_top_level_fallback() {
        let value: Value = serde_json::from_str(
            r#"{
              "locked": {"rev": "abc", "lastModified": 100},
              "revision": "ignored",
              "lastModified": 200
            }"#,
        )
        .unwrap();
        assert_eq!(
            metadata_locked(&value),
            (Some("abc".to_string()), Some(100))
        );

        let fallback: Value =
            serde_json::from_str(r#"{"revision": "def", "lastModified": 300}"#).unwrap();
        assert_eq!(
            metadata_locked(&fallback),
            (Some("def".to_string()), Some(300))
        );
    }

    #[test]
    fn detects_local_references() {
        assert!(is_local(&flake_ref::parse("./flake").unwrap()));
        assert!(is_local(&flake_ref::parse("path:/tmp/flake").unwrap()));
        assert!(!is_local(
            &flake_ref::parse("github:NixOS/nixpkgs").unwrap()
        ));
        assert!(!is_local(
            &flake_ref::parse("git+https://example.org/repo").unwrap()
        ));
    }

    #[test]
    fn skips_inputs_without_a_url() {
        let status = check("/tmp", &FlakeInput::new("foo")).unwrap();
        assert_eq!(status.state, InputState::Skipped("no URL declared".into()));
    }

    #[test]
    fn rejects_an_empty_update_name() {
        assert!(update("/tmp", "  ").is_err());
    }

    #[test]
    fn update_all_rejects_empty_and_nixpkgs() {
        assert!(update_all("/tmp", &[]).is_err());
        assert!(update_all("/tmp", &["nixpkgs".to_string()]).is_err());
    }

    #[test]
    fn reports_a_completed_command_with_its_output() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf ok"]);
        let output = run_with_timeout(
            command,
            "test command",
            Duration::from_secs(5),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"ok");
    }

    #[test]
    fn kills_a_command_that_exceeds_the_deadline() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5"]);
        let error = run_with_timeout(
            command,
            "test command",
            Duration::from_millis(100),
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert!(error.contains("timed out"), "{error}");
    }

    #[test]
    fn stops_a_command_when_cancelled() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5"]);
        let cancel = AtomicBool::new(true);
        let error =
            run_with_timeout(command, "test command", Duration::from_secs(5), &cancel).unwrap_err();
        assert!(error.contains("cancelled"), "{error}");
    }

    #[test]
    fn check_with_rejects_a_preexisting_cancel() {
        let cancel = AtomicBool::new(true);
        let result = check_with("/tmp", &FlakeInput::new("foo"), CHECK_TIMEOUT, &cancel);
        assert!(result.is_err());
    }
}
