use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;

use super::pump::spawn;
use super::state::{App, Msg, Tx};

// ---------------------------------------------------------------------------
// git sync
// ---------------------------------------------------------------------------

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
        let result = run_git(&dir, &["pull"]).and_then(|_| {
            let status = run_git(&dir, &["status", "--porcelain"])?;
            Ok(!status.trim().is_empty())
        });
        let _ = tx.send_blocking(Msg::GitPulled(result));
    });
}

/// Pull the flake directory without merging. Used by the automatic sync on
/// open, where a failed pull is reported and the scan continues.
pub(super) fn pull(dir: &str) -> Result<(), String> {
    run_git(dir, &["pull", "--ff-only"]).map(|_| ())
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

/// Fetch and reject a commit when the branch is behind or diverged from its
/// upstream. Returns whether the working tree has changes to commit.
fn save_sync_state(dir: &str) -> Result<bool, String> {
    run_git(dir, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|_| "the flake directory is not a git repository".to_string())?;
    run_git(
        dir,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .map_err(|_| "no upstream branch is configured".to_string())?;
    run_git(dir, &["fetch"])?;
    let counts = run_git(dir, &["rev-list", "--left-right", "--count", "HEAD...@{u}"])?;
    match classify(&counts)? {
        RemoteState::Current => {}
        RemoteState::Behind => {
            return Err(
                "the upstream branch has newer commits; the save was written to disk but not committed"
                    .to_string(),
            )
        }
        RemoteState::Diverged => {
            return Err(
                "the branch and its upstream have diverged; the save was written to disk but not committed"
                    .to_string(),
            )
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

/// Stage every change in the flake directory, commit it, and push.
fn git_commit_and_push(dir: &str, message: &str) -> Result<String, String> {
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

/// Run a git subcommand in `dir`, returning stdout on success and a message on
/// failure.
fn run_git(dir: &str, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
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
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                let message = entry.text().to_string();
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
}
