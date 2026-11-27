use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;

use super::pump::spawn;
use super::state::{App, Msg, Tx};

// ---------------------------------------------------------------------------
// git sync
// ---------------------------------------------------------------------------

/// The branch currently checked out. A detached HEAD cannot be pulled by name.
fn current_branch(dir: &str) -> Result<String, String> {
    let branch = run_git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = branch.trim();
    if branch.is_empty() || branch == "HEAD" {
        return Err("the flake repository is not on a checked-out branch".to_string());
    }
    Ok(branch.to_string())
}

/// Pull the checked-out branch from its configured upstream.
fn pull_args(ff_only: bool) -> Vec<&'static str> {
    let mut args = vec!["pull", "--no-edit"];
    if ff_only {
        args.push("--ff-only");
    }
    args
}

fn upstream_ref(dir: &str) -> Result<String, String> {
    let upstream = run_git(
        dir,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )?;
    let upstream = upstream.trim();
    if upstream.is_empty() {
        return Err("the checked-out branch has no upstream".to_string());
    }
    Ok(upstream.to_string())
}

fn pull_branch(dir: &str, ff_only: bool) -> Result<(), String> {
    current_branch(dir)?;
    run_git(dir, &pull_args(ff_only)).map(|_| ())
}

/// Pull the flake directory, then report whether the tree is dirty or has
/// untracked files. Runs off the UI thread.
pub(super) fn start_git_sync(app: &Rc<App>, tx: &Tx) {
    let dir = app.base.borrow().clone();
    if dir.is_empty() {
        app.status.set_text("No flake directory open");
        return;
    }
    app.status.set_text("Git: pulling...");
    spawn(tx, move |tx| {
        let result = pull_branch(&dir, false).and_then(|_| {
            let status = run_git(&dir, &["status", "--porcelain"])?;
            Ok(!status.trim().is_empty())
        });
        let _ = tx.send_blocking(Msg::GitPulled(result));
    });
}

/// Pull the flake directory without merging. Used by the automatic sync on
/// open, where a failed pull is reported and the scan continues.
pub(super) fn pull(dir: &str) -> Result<(), String> {
    let _ = run_git(dir, &["fetch"]);
    pull_branch(dir, true).map_err(|error| pull_error(dir, error))
}

fn pull_error(dir: &str, error: String) -> String {
    if error.contains("Your local changes") || error.contains("would be overwritten") {
        let upstream = upstream_ref(dir).unwrap_or_else(|_| "the upstream".to_string());
        let counts = ahead_behind(dir, &upstream).unwrap_or_default();
        return format!(
            "{dir}: local changes block the pull of {upstream} (ahead/behind {}); commit or stash these files, then reopen: {}",
            counts.trim(),
            blocking_paths(dir)
        );
    }
    fast_forward_error(dir, error)
}

fn ahead_behind(dir: &str, upstream: &str) -> Result<String, String> {
    let range = format!("HEAD...{upstream}");
    run_git(
        dir,
        &["rev-list", "--left-right", "--count", range.as_str()],
    )
}

fn blocking_paths(dir: &str) -> String {
    let paths = run_git(dir, &["diff", "--name-only"]).unwrap_or_default();
    let listed: Vec<&str> = paths
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if listed.is_empty() {
        "unknown".to_string()
    } else {
        listed.join(", ")
    }
}

/// A fast-forward refusal is a divergence the user must resolve. Report the
/// repository path, the checked-out branch's remote and the ahead/behind
/// counts so a lookalike branch cannot masquerade as a synced one.
fn fast_forward_error(dir: &str, error: String) -> String {
    if error.contains("fast-forward") {
        let upstream = match upstream_ref(dir) {
            Ok(upstream) => upstream,
            Err(_) => return format!("{dir}: git pull refused: {error}"),
        };
        let counts = ahead_behind(dir, &upstream).unwrap_or_default();
        match classify(counts.trim()) {
            Ok(RemoteState::Current) => {
                format!("{dir}: git pull refused by {upstream}: {error}")
            }
            Ok(RemoteState::Behind) => behind_message(dir, &upstream, counts.trim()),
            Ok(RemoteState::Diverged) => divergence_message(dir, &upstream, counts.trim()),
            Err(_) => format!("{dir}: git pull refused by {upstream}: {error}"),
        }
    } else {
        format!("{dir}: {error}")
    }
}

fn behind_message(dir: &str, upstream: &str, counts: &str) -> String {
    format!(
        "{dir}: the local branch is behind {upstream} (ahead/behind {counts}); pull it, then reopen"
    )
}

fn divergence_message(dir: &str, upstream: &str, counts: &str) -> String {
    format!(
        "{dir}: the local branch has diverged from {upstream} (ahead/behind {counts}); merge it or push it, then reopen"
    )
}

/// Whether the local branch can accept a new commit and push.
#[derive(Debug, PartialEq, Eq)]
enum RemoteState {
    Current,
    Behind,
    Diverged,
}

fn classify(output: &str) -> Result<RemoteState, String> {
    let mut fields = output.split_whitespace();
    let mut count = || -> Result<u64, String> {
        fields
            .next()
            .ok_or_else(|| "cannot read git ahead/behind count".to_string())?
            .parse()
            .map_err(|_| "cannot read git ahead/behind count".to_string())
    };
    let ahead = count()?;
    let behind = count()?;
    Ok(if behind == 0 {
        RemoteState::Current
    } else if ahead == 0 {
        RemoteState::Behind
    } else {
        RemoteState::Diverged
    })
}

/// Fetch and reject a commit when the checked-out branch is behind or diverged
/// from its same-named origin branch. Returns whether the working tree has
/// changes to commit.
fn save_sync_state(dir: &str) -> Result<bool, String> {
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| "the flake directory is not a git repository".to_string())?;
    current_branch(dir)?;
    let upstream = upstream_ref(dir)?;
    run_git(dir, &["fetch"])?;
    let range = format!("HEAD...{upstream}");
    let counts = run_git(
        dir,
        &["rev-list", "--left-right", "--count", range.as_str()],
    )?;
    match classify(&counts)? {
        RemoteState::Current => {}
        RemoteState::Behind => {
            return Err(format!(
                "{upstream} has newer commits; the save was written to disk but not committed"
            ))
        }
        RemoteState::Diverged => {
            return Err(format!(
                "the branch and {upstream} have diverged; the save was written to disk but not committed"
            ))
        }
    }
    let status = run_git(dir, &["status", "--porcelain"])?;
    Ok(!status.trim().is_empty())
}

/// Check the repository, then let the caller offer the commit prompt when the
/// save left changes to commit.
pub(super) fn start_save_sync(app: &Rc<App>, tx: &Tx) {
    let dir = app.base.borrow().clone();
    if dir.is_empty() {
        return;
    }
    app.status.set_text("Git: checking...");
    spawn(tx, move |tx| {
        let result = save_sync_state(&dir);
        let _ = tx.send_blocking(Msg::GitSaveChecked(result));
    });
}

/// Stage every change in the flake directory, commit it, and push the
/// checked-out branch to its same-named origin branch.
fn git_commit_and_push(dir: &str, message: &str) -> Result<String, String> {
    current_branch(dir)?;
    run_git(dir, &["add", "./*"])?;
    run_git(dir, &["commit", "-m", message])?;
    run_git(dir, &["push"])?;
    Ok(message.to_string())
}

/// Stage one file inside the flake directory.
pub(super) fn stage_path(dir: &str, path: &Path) -> Result<(), String> {
    let relative = relative_in_repo(dir, path)?;
    run_git(dir, &["add", "--", &relative]).map(|_| ())
}

/// The path of `path` relative to the repository root, for use as a git
/// pathspec. The file must exist.
pub(super) fn relative_in_repo(dir: &str, path: &Path) -> Result<String, String> {
    let base = std::fs::canonicalize(dir).unwrap_or_else(|_| PathBuf::from(dir));
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let relative = target.strip_prefix(&base).map_err(|_| {
        format!(
            "{} is outside the repository {}",
            path.display(),
            base.display()
        )
    })?;
    Ok(relative.to_string_lossy().into_owned())
}

/// Stage the deletion of a file that has already been removed.
pub(super) fn stage_deletion(dir: &str, relative: &str) -> Result<(), String> {
    run_git(dir, &["add", "-u", "--", relative]).map(|_| ())
}

/// A git invocation with prompts and editors disabled, so a merge never waits
/// for a terminal the GUI does not have.
fn git_command(dir: &str, args: &[&str]) -> std::process::Command {
    let mut command = std::process::Command::new("git");
    command
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_EDITOR", "true");
    command
}

/// Run a git subcommand in `dir`, returning stdout on success and a message on
/// failure.
fn run_git(dir: &str, args: &[&str]) -> Result<String, String> {
    let output = git_command(dir, args)
        .output()
        .map_err(|e| format!("cannot run git {}: {e}", args.join(" ")))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    Err(if detail.is_empty() {
        format!("git {} failed", args.join(" "))
    } else {
        format!("git {}: {detail}", args.join(" "))
    })
}

/// Ask for a commit message, then commit and push the staged changes.
pub(super) fn commit_message_dialog(app: &Rc<App>, tx: &Tx) {
    let dialog = gtk::Dialog::builder()
        .title("Commit changes")
        .transient_for(&app.window)
        .modal(true)
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    let commit = dialog.add_button("Commit & push", gtk::ResponseType::Accept);
    commit.add_css_class("suggested-action");
    dialog.set_default_response(gtk::ResponseType::Accept);

    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some("Commit message"));
    entry.set_hexpand(true);
    entry.set_activates_default(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.append(
        &gtk::Label::builder()
            .label("Commit message")
            .xalign(0.0)
            .build(),
    );
    content.append(&entry);
    dialog.content_area().append(&content);

    {
        let app = app.clone();
        let tx = tx.clone();
        let dialog_entry = entry.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let message = dialog_entry.text().to_string();
                if message.trim().is_empty() {
                    app.status.set_text("Commit message cannot be empty");
                    dialog_entry.grab_focus();
                    dialog_entry.select_region(0, -1);
                    return;
                }
                let dir = app.base.borrow().clone();
                app.status.set_text("Git: committing and pushing...");
                spawn(&tx, move |tx| {
                    let result = git_commit_and_push(&dir, &message);
                    let _ = tx.send_blocking(Msg::GitSynced(result));
                });
            }
            dialog.close();
        });
    }
    entry.grab_focus();
    entry.select_region(0, -1);
    dialog.present();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_remote_state() {
        assert_eq!(classify("0\t0").unwrap(), RemoteState::Current);
        assert_eq!(classify("2\t0").unwrap(), RemoteState::Current);
        assert_eq!(classify("0\t2").unwrap(), RemoteState::Behind);
        assert_eq!(classify("1\t2").unwrap(), RemoteState::Diverged);
    }

    fn env_value(command: &std::process::Command, key: &str) -> Option<String> {
        command
            .get_envs()
            .find(|(name, _)| name.to_string_lossy() == key)
            .and_then(|(_, value)| value)
            .map(|value| value.to_string_lossy().into_owned())
    }

    #[test]
    fn pull_uses_the_configured_upstream() {
        assert_eq!(pull_args(false), vec!["pull", "--no-edit"]);
        assert_eq!(pull_args(true), vec!["pull", "--no-edit", "--ff-only"]);
    }

    #[test]
    fn pull_is_non_interactive() {
        let sync = git_command("/tmp", &pull_args(false));
        assert_eq!(
            sync.get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["pull", "--no-edit"]
        );
        assert_eq!(
            env_value(&sync, "GIT_TERMINAL_PROMPT").as_deref(),
            Some("0")
        );
        assert_eq!(env_value(&sync, "GIT_EDITOR").as_deref(), Some("true"));

        let open = git_command("/tmp", &pull_args(true));
        assert_eq!(
            open.get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["pull", "--no-edit", "--ff-only"]
        );
        assert_eq!(env_value(&open, "GIT_EDITOR").as_deref(), Some("true"));
    }

    #[test]
    fn fast_forward_refusal_names_the_real_state() {
        let behind = behind_message("/repo", "origin/main", "0\t2");
        assert!(behind.contains("/repo"));
        assert!(behind.contains("origin/main"));
        assert!(behind.contains("0\t2"));
        assert!(behind.contains("behind"));
        assert!(!behind.contains("diverged"));

        let diverged = divergence_message("/repo", "origin/main", "1\t2");
        assert!(diverged.contains("1\t2"));
        assert!(diverged.contains("diverged"));

        assert_eq!(
            classify("0\t0").unwrap(),
            RemoteState::Current,
            "a zero ahead/behind count is not a divergence"
        );
        assert_eq!(
            fast_forward_error("/repo", "git pull: no such remote".to_string()),
            "/repo: git pull: no such remote"
        );
    }

    fn git_in(dir: &Path, args: &[&str]) -> bool {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn commit_file(dir: &Path, name: &str, text: &str) -> bool {
        std::fs::write(dir.join(name), text).is_ok()
            && git_in(dir, &["add", name])
            && git_in(dir, &["commit", "-m", text])
    }

    /// A synced, ahead-only, behind-only and diverged repository must never be
    /// reported as diverged just because `git pull --ff-only` refused.
    #[test]
    fn only_a_real_divergence_reports_diverged() {
        if std::process::Command::new("git")
            .arg("--version")
            .output()
            .map(|output| !output.status.success())
            .unwrap_or(true)
        {
            eprintln!("skipping git divergence regression test: git is unavailable");
            return;
        }
        let root = std::env::temp_dir().join(format!(
            "nsg-git-divergence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let seed = root.join("seed");
        assert!(std::fs::create_dir_all(&root).is_ok());
        assert!(git_in(&root, &["init", "--bare", "origin.git"]));
        assert!(git_in(&root, &["clone", "origin.git", "seed"]));
        assert!(git_in(&seed, &["checkout", "-B", "main"]));
        assert!(commit_file(&seed, "file.txt", "one"));
        assert!(git_in(&seed, &["push", "-u", "origin", "main"]));

        let clone = |name: &str| {
            let path = root.join(name);
            assert!(git_in(&root, &["clone", "origin.git", name]));
            path
        };
        let synced = clone("synced");
        let ahead = clone("ahead");
        assert!(commit_file(&ahead, "local.txt", "ahead"));
        let behind = clone("behind");
        let diverged = clone("diverged");
        assert!(commit_file(&diverged, "local.txt", "local"));
        assert!(commit_file(&seed, "remote.txt", "remote"));
        assert!(git_in(&seed, &["push", "origin", "main"]));
        assert!(git_in(&behind, &["fetch"]));
        assert!(git_in(&diverged, &["fetch"]));

        for (name, dir) in [("synced", &synced), ("ahead", &ahead), ("behind", &behind)] {
            let message = fast_forward_error(
                &dir.display().to_string(),
                "fatal: Not possible to fast-forward, aborting.".to_string(),
            );
            assert!(
                !message.contains("diverged"),
                "{name} must not be reported as diverged: {message}"
            );
        }
        let message = fast_forward_error(
            &diverged.display().to_string(),
            "fatal: Not possible to fast-forward, aborting.".to_string(),
        );
        assert!(message.contains("diverged"), "{message}");

        let no_upstream = root.join("no-upstream");
        assert!(git_in(&root, &["init", "no-upstream"]));
        let message = fast_forward_error(
            &no_upstream.display().to_string(),
            "fatal: Not possible to fast-forward, aborting.".to_string(),
        );
        assert!(!message.contains("diverged"), "{message}");
    }
}
