//! Minimal local backend for comment-driven Nix settings frontends.
//!
//! Run with `--stdio` for an embedding client, or with `--http` for a local
//! JSON-RPC endpoint. The HTTP listener binds to loopback only.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nix_settings_gui::{edit, scan};
use serde_json::{json, Value};

struct Session {
    root: String,
    scanned: scan::ScanResult,
    staged: HashMap<String, String>,
}

impl Session {
    fn open(root: String) -> Result<Self, String> {
        let scanned = scan::scan_root(&root)?;
        Ok(Self {
            root,
            scanned,
            staged: HashMap::new(),
        })
    }

    fn record(&self, path: &str) -> Option<&nix_settings_gui::model::Record> {
        self.scanned
            .menus
            .iter()
            .flat_map(|menu| menu.records.iter())
            .find(|record| record.path() == path)
    }

    fn pending_edits(&self) -> Result<Vec<edit::PendingEdit>, String> {
        let mut edits = Vec::new();
        for (path, value) in &self.staged {
            let record = self
                .record(path)
                .ok_or_else(|| format!("unknown option path: {path}"))?;
            edits.push(edit::PendingEdit {
                file: record.file.clone(),
                start: record.byte_start,
                end: record.byte_end,
                value: value.clone(),
            });
        }
        Ok(edits)
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut stdio = false;
    let mut http: Option<String> = None;
    let mut token_file: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stdio" => stdio = true,
            "--http" => http = args.next(),
            "--token-file" => token_file = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                println!(
                    "Usage: nix-settings-backend [--stdio] [--http <addr>] [--token-file <path>]"
                );
                return;
            }
            other => {
                eprintln!("nix-settings-backend: unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    if stdio {
        run_stdio();
        return;
    }

    let addr = http.unwrap_or_else(|| "127.0.0.1:0".to_string());
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("nix-settings-backend: cannot bind {addr}: {error}");
            std::process::exit(1);
        }
    };
    let token_path = token_file.or_else(default_token_path);
    let token = match token_path.as_deref().map(load_or_create_token) {
        Some(Ok(token)) => Some(token),
        Some(Err(error)) => {
            eprintln!("nix-settings-backend: {error}");
            std::process::exit(1);
        }
        None => None,
    };
    let address = listener.local_addr().ok();
    if let Some(address) = address {
        eprintln!("nix-settings-backend listening on {address}");
    }
    if let Some(token) = token.as_deref() {
        eprintln!("nix-settings-backend token: {token}");
    }
    let session = Arc::new(Mutex::new(None));
    for stream in listener.incoming().flatten() {
        let _ = serve_http(stream, token.as_deref(), &session);
    }
}

fn run_stdio() {
    let mut session = None;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle(&mut session, request),
            Err(error) => rpc_error(Value::Null, -32700, &format!("parse error: {error}")),
        };
        println!("{response}");
        let _ = std::io::stdout().flush();
    }
}

fn serve_http(
    mut stream: TcpStream,
    token: Option<&str>,
    session: &Arc<Mutex<Option<Session>>>,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut content_length = 0usize;
    let mut authorized = token.is_none();
    let mut origin_ok = true;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            } else if name == "authorization" {
                authorized = token.is_some_and(|token| value == format!("Bearer {token}"));
            } else if name == "origin" {
                origin_ok = value.starts_with("http://127.0.0.1")
                    || value.starts_with("http://localhost")
                    || value.starts_with("https://127.0.0.1")
                    || value.starts_with("https://localhost");
            }
        }
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    let reply = if !request_line.starts_with("POST /rpc ") {
        http_reply(404, &rpc_error(Value::Null, -32601, "use POST /rpc"))
    } else if !authorized {
        http_reply(401, &rpc_error(Value::Null, -32001, "missing or invalid token"))
    } else if !origin_ok {
        http_reply(403, &rpc_error(Value::Null, -32002, "origin is not allowed"))    } else {
        let request = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
        let mut guard = session.lock().unwrap_or_else(|error| error.into_inner());
        http_reply(200, &handle(&mut guard, request))
    };
    stream.write_all(reply.as_bytes())?;
    stream.flush()
}

fn http_reply(status: u16, body: &Value) -> String {
    let body = body.to_string();
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn handle(session: &mut Option<Session>, request: Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "session.open" => {
            let root = params
                .get("root")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            match Session::open(root) {
                Ok(opened) => {
                    let menus = opened.scanned.menus.len();
                    let records: usize = opened
                        .scanned
                        .menus
                        .iter()
                        .map(|menu| menu.records.len())
                        .sum();
                    *session = Some(opened);
                    Ok(json!({ "apiVersion": 1, "menus": menus, "records": records }))
                }
                Err(error) => Err((-32010, error)),
            }
        }
        "session.close" => {
            *session = None;
            Ok(json!({ "closed": true }))
        }
        "scan" => with_session(session, |session| {
            Ok(json!({
                "menus": session.scanned.menus.iter().map(|menu| json!({
                    "file": menu.file,
                    "displayname": menu.display_name(),
                    "records": menu.records.iter().map(record_json).collect::<Vec<_>>(),
                })).collect::<Vec<_>>()
            }))
        }),
        "page.list" => with_session(session, |session| {
            Ok(json!({
                "pages": session.scanned.menus.iter().map(|menu| json!({
                    "id": menu.file,
                    "title": menu.display_name(),
                    "description": menu.spec.description,
                    "records": menu.records.iter().map(|record| record.path()).collect::<Vec<_>>(),
                })).collect::<Vec<_>>()
            }))
        }),
        "control.get" => with_session(session, |session| {
            let path = params.get("path").and_then(Value::as_str).unwrap_or_default();
            session
                .record(path)
                .map(|record| Ok(record_json(record)))
                .unwrap_or_else(|| Err((-32011, format!("unknown option path: {path}"))))
        }),
        "control.set" => with_session(session, |session| {
            let path = params
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let value = params
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if session.record(&path).is_none() {
                return Err((-32011, format!("unknown option path: {path}")));
            }
            session.staged.insert(path, value);
            Ok(json!({ "staged": true }))
        }),
        "control.reset" => with_session(session, |session| {
            let path = params.get("path").and_then(Value::as_str).unwrap_or_default();
            session.staged.remove(path);
            Ok(json!({ "staged": false }))
        }),
        "edits.preview" => with_session(session, |session| {
            let edits = session
                .pending_edits()
                .map_err(|error| (-32012, error))?;
            Ok(json!({
                "edits": edits.iter().map(|edit| json!({
                    "file": edit.file,
                    "start": edit.start,
                    "end": edit.end,
                    "before": std::fs::read_to_string(&edit.file).ok()
                        .and_then(|text| text.get(edit.start..edit.end).map(str::to_string)),
                    "after": edit.value,
                })).collect::<Vec<_>>()
            }))
        }),
        "edits.apply" => with_session(session, |session| {
            let edits = session
                .pending_edits()
                .map_err(|error| (-32012, error))?;
            if edits.is_empty() {
                return Ok(json!({ "files": 0 }));
            }
            match edit::apply(&edits, &session.root) {
                Ok(files) => {
                    session.staged.clear();
                    session.scanned = scan::scan_root(&session.root)
                        .map_err(|error| (-32013, error))?;
                    Ok(json!({ "files": files }))
                }
                Err(error) => Err((-32012, error)),
            }
        }),
        other => Err((-32601, format!("unknown method: {other}"))),
    };

    match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => rpc_error_with_id(id, code, &message),
    }
}

fn with_session(
    session: &mut Option<Session>,
    f: impl FnOnce(&mut Session) -> Result<Value, (i32, String)>,
) -> Result<Value, (i32, String)> {
    let session = session.as_mut().ok_or((-32000, "no open session".to_string()))?;
    f(session)
}

fn record_json(record: &nix_settings_gui::model::Record) -> Value {
    json!({
        "path": record.path(),
        "file": record.file,
        "value": record.value,
        "label": record.spec.label,
        "description": record.spec.description,
        "widget": record.spec.widget.map(|widget| format!("{widget:?}").to_lowercase()),
        "choices": record.spec.choices,
        "advanced": record.spec.advanced,
        "hidden": record.spec.hidden,
        "readonly": record.spec.readonly,
    })
}

fn rpc_error(id: Value, code: i32, message: &str) -> Value {
    rpc_error_with_id(id, code, message)
}

fn rpc_error_with_id(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn default_token_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(base.join("nix-settings-backend").join("token"))
}

fn load_or_create_token(path: &std::path::Path) -> Result<String, String> {    if let Ok(token) = std::fs::read_to_string(path) {
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let mut bytes = [0u8; 32];
    let mut source = std::fs::File::open("/dev/urandom")
        .map_err(|error| format!("cannot read random bytes: {error}"))?;
    source
        .read_exact(&mut bytes)
        .map_err(|error| format!("cannot read random bytes: {error}"))?;
    let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::write(path, &token)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(token)
}
