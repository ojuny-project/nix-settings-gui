use std::process::Command;

use serde_json::Value;

use crate::flake_ref::encode_query_value;

/// Search GitHub or GitLab for users and organizations matching `query`.
pub fn scan_users(kind: &str, query: &str) -> Result<Vec<String>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Enter a user or organization to scan".to_string());
    }
    match kind {
        "github" => {
            let url = format!(
                "https://api.github.com/search/users?q={}&per_page=20",
                encode_query_value(query)
            );
            let value = http_json(&url, "github")?;
            Ok(sorted(
                value
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|item| item.get("login").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect(),
            ))
        }
        "gitlab" => {
            let url = format!(
                "https://gitlab.com/api/v4/users?search={}&per_page=20",
                encode_query_value(query)
            );
            let value = http_json(&url, "gitlab")?;
            Ok(sorted(
                value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|user| user.get("username").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect(),
            ))
        }
        "sourcehut" => Err(
            "SourceHut has no unauthenticated owner search; type the owner manually".to_string(),
        ),
        _ => Err(format!("`{kind}` does not support owner scans")),
    }
}

/// List repositories for `owner` on the selected forge.
pub fn scan_repos(kind: &str, host: Option<&str>, owner: &str) -> Result<Vec<String>, String> {
    let owner = owner.trim();
    if owner.is_empty() {
        return Err("Enter an owner before scanning repositories".to_string());
    }
    match kind {
        "github" => {
            let api = github_api(host);
            let user = format!(
                "{api}/users/{}/repos?per_page=100&sort=pushed",
                encode_query_value(owner)
            );
            let value = match http_json(&user, "github") {
                Ok(value) => value,
                Err(_) => {
                    let org = format!(
                        "{api}/orgs/{}/repos?per_page=100&sort=pushed",
                        encode_query_value(owner)
                    );
                    http_json(&org, "github")?
                }
            };
            Ok(sorted(
                value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|repo| repo.get("name").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect(),
            ))
        }
        "gitlab" => {
            let api = gitlab_api(host);
            let user = format!(
                "{api}/users/{}/projects?per_page=100",
                encode_query_value(owner)
            );
            let value = match http_json(&user, "gitlab") {
                Ok(value) => value,
                Err(_) => {
                    let group = format!(
                        "{api}/groups/{}/projects?per_page=100",
                        encode_query_value(owner)
                    );
                    http_json(&group, "gitlab")?
                }
            };
            Ok(sorted(
                value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|repo| repo.get("path").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect(),
            ))
        }
        "sourcehut" => Err(
            "SourceHut has no unauthenticated repository search; type the repository manually"
                .to_string(),
        ),
        _ => Err(format!("`{kind}` does not support repository scans")),
    }
}

/// List branch or tag names for a hosted repository.
pub fn scan_refs(
    kind: &str,
    host: Option<&str>,
    owner: &str,
    repo: &str,
    tags: bool,
) -> Result<Vec<String>, String> {
    let owner = require("owner", owner)?;
    let repo = require("repository", repo)?;
    match kind {
        "github" => {
            let api = github_api(host);
            let section = if tags { "tags" } else { "branches" };
            let url = format!(
                "{api}/repos/{}/{}/{section}?per_page=100",
                encode_query_value(owner),
                encode_query_value(repo)
            );
            match http_json(&url, "github") {
                Ok(value) => Ok(sorted(
                    value
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|item| item.get("name").and_then(Value::as_str))
                        .map(str::to_string)
                        .collect(),
                )),
                Err(_) => git_refs(&host_url(kind, host, owner, repo), tags),
            }
        }
        "gitlab" => {
            let api = gitlab_api(host);
            let project = encode_query_value(&format!("{owner}/{repo}"));
            let section = if tags { "tags" } else { "branches" };
            let url = format!("{api}/projects/{project}/repository/{section}?per_page=100");
            match http_json(&url, "gitlab") {
                Ok(value) => Ok(sorted(
                    value
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|item| item.get("name").and_then(Value::as_str))
                        .map(str::to_string)
                        .collect(),
                )),
                Err(_) => git_refs(&host_url(kind, host, owner, repo), tags),
            }
        }
        "sourcehut" => git_refs(&host_url(kind, host, owner, repo), tags),
        _ => Err(format!("`{kind}` does not support ref scans")),
    }
}

/// One scanned commit: the hash written into the flake URL and its date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitEntry {
    pub sha: String,
    /// `YYYY-MM-DD` when the forge reports a date, otherwise empty.
    pub date: String,
}

/// List commit hashes on `branch`.
#[allow(dead_code)]
pub fn scan_commits(
    kind: &str,
    host: Option<&str>,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Vec<String>, String> {
    Ok(scan_commit_details(kind, host, owner, repo, branch)?
        .into_iter()
        .map(|commit| commit.sha)
        .collect())
}

/// List commits on `branch` with their dates, for the commit picker.
pub fn scan_commit_details(
    kind: &str,
    host: Option<&str>,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Vec<CommitEntry>, String> {
    let owner = require("owner", owner)?;
    let repo = require("repository", repo)?;
    let branch = require("branch", branch)?;
    match kind {
        "github" => {
            let api = github_api(host);
            let url = format!(
                "{api}/repos/{}/{}/commits?sha={}&per_page=50",
                encode_query_value(owner),
                encode_query_value(repo),
                encode_query_value(branch)
            );
            match http_json(&url, "github") {
                Ok(value) => Ok(sorted_commits(github_commits(&value))),
                Err(_) => git_tip_details(&host_url(kind, host, owner, repo), branch),
            }
        }
        "gitlab" => {
            let api = gitlab_api(host);
            let project = encode_query_value(&format!("{owner}/{repo}"));
            let url = format!(
                "{api}/projects/{project}/repository/commits?ref_name={}&per_page=50",
                encode_query_value(branch)
            );
            match http_json(&url, "gitlab") {
                Ok(value) => Ok(sorted_commits(gitlab_commits(&value))),
                Err(_) => git_tip_details(&host_url(kind, host, owner, repo), branch),
            }
        }
        "sourcehut" => git_tip_details(&host_url(kind, host, owner, repo), branch),
        _ => Err(format!("`{kind}` does not support commit scans")),
    }
}

/// List branch and tag names for an arbitrary git URL.
pub fn scan_git_refs(url: &str) -> Result<Vec<String>, String> {
    let url = require("repository URL", url)?;
    let mut refs = git_refs(url, false)?;
    refs.extend(git_refs(url, true)?);
    Ok(sorted(refs))
}

/// List the tip commit of `branch` for an arbitrary git URL.
#[allow(dead_code)]
pub fn scan_git_commits(url: &str, branch: &str) -> Result<Vec<String>, String> {
    Ok(scan_git_commit_details(url, branch)?
        .into_iter()
        .map(|commit| commit.sha)
        .collect())
}

/// List the tip commit of `branch` with its date for an arbitrary git URL.
/// `git ls-remote` reports no date, so the date stays empty.
pub fn scan_git_commit_details(url: &str, branch: &str) -> Result<Vec<CommitEntry>, String> {
    let url = require("repository URL", url)?;
    let branch = require("branch", branch)?;
    git_tip_details(url, branch)
}

/// List flake ids from the local flake registry.
pub fn scan_registry() -> Result<Vec<String>, String> {
    let output = Command::new("nix")
        .args([
            "registry",
            "list",
            "--extra-experimental-features",
            "nix-command flakes",
        ])
        .output()
        .map_err(|e| format!("cannot run nix registry list: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("nix registry list failed");
        return Err(detail.trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let ids = registry_ids(&stdout);
    if ids.is_empty() {
        return Ok(vec!["nixpkgs".to_string()]);
    }
    Ok(ids)
}

fn registry_ids(text: &str) -> Vec<String> {
    sorted(
        text.lines()
            .flat_map(str::split_whitespace)
            .filter_map(|token| token.strip_prefix("flake:"))
            .map(str::to_string)
            .collect(),
    )
}

fn require<'a>(label: &str, value: &'a str) -> Result<&'a str, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("Enter a {label} before scanning"))
    } else {
        Ok(trimmed)
    }
}

fn github_api(host: Option<&str>) -> String {
    match host.map(str::trim).filter(|host| !host.is_empty()) {
        Some(host) if host != "github.com" => format!("https://{host}/api/v3"),
        _ => "https://api.github.com".to_string(),
    }
}

fn gitlab_api(host: Option<&str>) -> String {
    let host = host
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .unwrap_or("gitlab.com");
    format!("https://{host}/api/v4")
}

fn host_url(kind: &str, host: Option<&str>, owner: &str, repo: &str) -> String {
    let host = host
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .unwrap_or(match kind {
            "github" => "github.com",
            "gitlab" => "gitlab.com",
            "sourcehut" => "git.sr.ht",
            _ => "example.org",
        });
    let owner = if kind == "sourcehut" && !owner.starts_with('~') {
        format!("~{owner}")
    } else {
        owner.to_string()
    };
    format!("https://{host}/{owner}/{repo}")
}

fn git_refs(url: &str, tags: bool) -> Result<Vec<String>, String> {
    let kind = if tags { "--tags" } else { "--heads" };
    let output = Command::new("git")
        .args(["ls-remote", kind, url])
        .output()
        .map_err(|e| format!("cannot run git ls-remote: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("git ls-remote failed");
        return Err(detail.trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let prefix = if tags { "refs/tags/" } else { "refs/heads/" };
    Ok(sorted(
        stdout
            .lines()
            .filter_map(|line| line.split('\t').nth(1))
            .filter_map(|reference| reference.strip_prefix(prefix))
            .filter(|name| !name.ends_with("^{}"))
            .map(str::to_string)
            .collect(),
    ))
}

fn git_tip_details(url: &str, branch: &str) -> Result<Vec<CommitEntry>, String> {
    let reference = format!("refs/heads/{branch}");
    let output = Command::new("git")
        .args(["ls-remote", url, &reference])
        .output()
        .map_err(|e| format!("cannot run git ls-remote: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("git ls-remote failed");
        return Err(detail.trim().to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let tip = stdout
        .lines()
        .find_map(|line| line.split('\t').next())
        .map(str::to_string);
    match tip {
        Some(sha) => Ok(vec![CommitEntry {
            sha,
            date: String::new(),
        }]),
        None => Err(format!("branch `{branch}` was not found")),
    }
}

fn github_commits(value: &Value) -> Vec<CommitEntry> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|commit| {
            let sha = commit.get("sha").and_then(Value::as_str)?;
            let date = commit
                .get("commit")
                .and_then(|commit| commit.get("committer").or_else(|| commit.get("author")))
                .and_then(|author| author.get("date"))
                .and_then(Value::as_str)
                .unwrap_or("");
            Some(CommitEntry {
                sha: sha.to_string(),
                date: commit_date(date),
            })
        })
        .collect()
}

fn gitlab_commits(value: &Value) -> Vec<CommitEntry> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|commit| {
            let sha = commit.get("id").and_then(Value::as_str)?;
            let date = commit
                .get("committed_date")
                .or_else(|| commit.get("created_at"))
                .and_then(Value::as_str)
                .unwrap_or("");
            Some(CommitEntry {
                sha: sha.to_string(),
                date: commit_date(date),
            })
        })
        .collect()
}

/// A forge timestamp is ISO 8601; the picker shows its calendar date.
fn commit_date(timestamp: &str) -> String {
    if timestamp.len() >= 10 && timestamp.as_bytes().get(4) == Some(&b'-') {
        timestamp[..10].to_string()
    } else {
        String::new()
    }
}

fn sorted_commits(mut commits: Vec<CommitEntry>) -> Vec<CommitEntry> {
    commits.sort_by(|a, b| a.sha.cmp(&b.sha));
    commits.dedup_by(|a, b| a.sha == b.sha);
    commits
}

fn http_json(url: &str, forge: &str) -> Result<Value, String> {
    let mut command = Command::new("curl");
    command.args([
        "-fsSL",
        "--max-time",
        "20",
        "-H",
        "Accept: application/json",
        "-H",
        "User-Agent: nix-settings-gui",
    ]);
    match forge {
        "github" => {
            command.args(["-H", "Accept: application/vnd.github+json"]);
            if let Ok(token) = std::env::var("GITHUB_TOKEN") {
                if !token.trim().is_empty() {
                    command.arg("-H");
                    command.arg(format!("Authorization: Bearer {}", token.trim()));
                }
            }
        }
        "gitlab" => {
            if let Ok(token) = std::env::var("GITLAB_TOKEN") {
                if !token.trim().is_empty() {
                    command.arg("-H");
                    command.arg(format!("PRIVATE-TOKEN: {}", token.trim()));
                }
            }
        }
        _ => {}
    }
    command.arg(url);
    let output = command
        .output()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("request failed");
        return Err(detail.trim().to_string());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("cannot parse response: {e}"))
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_registry_ids() {
        let text = "user   flake:nixpkgs github:NixOS/nixpkgs/nixos-unstable\n\
                    global flake:home-manager github:nix-community/home-manager\n\
                    global flake:nixpkgs github:NixOS/nixpkgs/nixos-unstable\n";
        assert_eq!(
            registry_ids(text),
            vec!["home-manager".to_string(), "nixpkgs".to_string()]
        );
    }

    #[test]
    fn rejects_blank_scan_inputs() {
        assert!(scan_repos("github", None, " ").is_err());
        assert!(scan_refs("github", None, "NixOS", "", false).is_err());
        assert!(scan_commits("github", None, "NixOS", "nixpkgs", "").is_err());
        assert!(scan_commit_details("github", None, "NixOS", "nixpkgs", "").is_err());
        assert!(scan_git_refs("").is_err());
    }

    #[test]
    fn reads_github_commit_shas_and_dates() {
        let json: Value = serde_json::from_str(
            r#"[{"sha":"abc123","commit":{"committer":{"date":"2026-11-27T10:00:00Z"}}},
                {"sha":"def456","commit":{"author":{"date":"2026-11-26T09:30:00Z"}}}]"#,
        )
        .unwrap();
        assert_eq!(
            github_commits(&json),
            vec![
                CommitEntry {
                    sha: "abc123".to_string(),
                    date: "2026-11-27".to_string(),
                },
                CommitEntry {
                    sha: "def456".to_string(),
                    date: "2026-11-26".to_string(),
                },
            ]
        );
    }

    #[test]
    fn reads_gitlab_commit_shas_and_dates() {
        let json: Value = serde_json::from_str(
            r#"[{"id":"abc123","committed_date":"2026-11-27T10:00:00.000+00:00"},
                {"id":"def456","created_at":"2026-11-26T09:30:00.000+00:00"}]"#,
        )
        .unwrap();
        assert_eq!(
            gitlab_commits(&json),
            vec![
                CommitEntry {
                    sha: "abc123".to_string(),
                    date: "2026-11-27".to_string(),
                },
                CommitEntry {
                    sha: "def456".to_string(),
                    date: "2026-11-26".to_string(),
                },
            ]
        );
    }

    #[test]
    fn commit_date_keeps_only_the_calendar_day() {
        assert_eq!(commit_date("2026-11-27T10:00:00Z"), "2026-11-27");
        assert_eq!(commit_date("not-a-date"), "");
        assert_eq!(commit_date(""), "");
    }

    #[test]
    fn commit_hashes_are_sorted_and_deduplicated() {
        let commits = sorted_commits(vec![
            CommitEntry {
                sha: "b".to_string(),
                date: String::new(),
            },
            CommitEntry {
                sha: "a".to_string(),
                date: String::new(),
            },
            CommitEntry {
                sha: "a".to_string(),
                date: "2026-11-27".to_string(),
            },
        ]);
        assert_eq!(
            commits
                .iter()
                .map(|commit| commit.sha.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }
}
