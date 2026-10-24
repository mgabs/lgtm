use anyhow::Result;
use lgtm_core::git::GitEngine;
use lgtm_core::model::DiffSnapshot;
use lgtm_core::watcher::{RepoWatcher, WatcherMessage};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct McpServer {
    git_engine: GitEngine,
    repo_path: PathBuf,
    subscriptions: Arc<Mutex<HashSet<String>>>,
}

impl McpServer {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        let path = repo_path.as_ref().to_path_buf();
        Self {
            git_engine: GitEngine::new(&path),
            repo_path: path,
            subscriptions: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Helper to classify which resource URIs are affected by a set of changed paths.
    pub fn affected_resources(paths: &[PathBuf]) -> Vec<&'static str> {
        let mut affected = HashSet::new();
        // Almost any code or directory change touches diff and findings
        if !paths.is_empty() {
            affected.insert("lgtm://diff");
            affected.insert("lgtm://findings");
        }
        for p in paths {
            let s = p.to_string_lossy();
            if s.contains("comments.jsonl") || s.contains("directives") {
                affected.insert("lgtm://directives");
            }
            if s.contains(".lgtm") || s.contains("state.json") || s.contains("reviews") {
                affected.insert("lgtm://review-state");
                affected.insert("lgtm://directives");
            }
        }
        let mut res: Vec<&'static str> = affected.into_iter().collect();
        res.sort();
        res
    }

    pub fn run_stdio(&self) -> Result<()> {
        let stdin = io::stdin();
        let stdout = Arc::new(Mutex::new(io::stdout()));

        // Start background watcher if repo_path exists and is a directory
        let (watcher_tx, watcher_rx) = crossbeam_channel::unbounded();
        let _watcher = if self.repo_path.is_dir() {
            RepoWatcher::spawn(&self.repo_path, watcher_tx, 300).ok()
        } else {
            None
        };

        // Background notifier thread: listens for watcher file changes, debounces & coalesces URIs,
        // checks active subscriptions, and writes notifications under stdout lock.
        let subs_clone = Arc::clone(&self.subscriptions);
        let stdout_clone = Arc::clone(&stdout);
        let _notifier_handle = std::thread::spawn(move || {
            while let Ok(msg) = watcher_rx.recv() {
                let paths = match msg {
                    WatcherMessage::FilesChanged(paths) => paths,
                    WatcherMessage::Error(_) => continue,
                };
                let affected = Self::affected_resources(&paths);
                let subs = match subs_clone.lock() {
                    Ok(guard) => guard.clone(),
                    Err(_) => continue,
                };

                for uri in affected {
                    if subs.contains(uri) {
                        let notification = json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/resources/updated",
                            "params": {
                                "uri": uri
                            }
                        });
                        if let Ok(mut out) = stdout_clone.lock() {
                            let _ = writeln!(out, "{}", notification); // lgtm:ignore
                            let _ = out.flush(); // lgtm:ignore
                        }
                    }
                }
            }
        });

        for line in stdin.lock().lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let request: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(err) => {
                    let err_response = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32700,
                            "message": format!("Parse error: {}", err)
                        }
                    });
                    if let Ok(mut out) = stdout.lock() {
                        writeln!(out, "{}", err_response)?;
                        out.flush()?;
                    }
                    continue;
                }
            };

            let id = request.get("id").cloned();
            let method = request
                .get("method")
                .and_then(|m| m.as_str())
                .unwrap_or_default();
            let params = request.get("params").cloned().unwrap_or(json!({}));

            // If it's a notification without id (e.g. notifications/initialized), don't send a response
            if id.is_none() && method.starts_with("notifications/") {
                continue;
            }

            let response = self.handle_request(method, &params, id.clone());
            if let Some(resp) = response {
                if let Ok(mut out) = stdout.lock() {
                    writeln!(out, "{}", resp)?;
                    out.flush()?;
                }
            }
        }

        Ok(())
    }

    fn handle_request(&self, method: &str, params: &Value, id: Option<Value>) -> Option<Value> {
        let id_val = id.unwrap_or(Value::Null);

        match method {
            "initialize" => Some(json!({
                "jsonrpc": "2.0",
                "id": id_val,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                        "resources": {
                            "subscribe": true,
                            "listChanged": true
                        }
                    },
                    "serverInfo": {
                        "name": "lgtm",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            })),
            "ping" => Some(json!({
                "jsonrpc": "2.0",
                "id": id_val,
                "result": {}
            })),
            "tools/list" => Some(json!({
                "jsonrpc": "2.0",
                "id": id_val,
                "result": {
                    "tools": [
                        {
                            "name": "lgtm_diff",
                            "description": "Get current repository diff snapshot with file-by-file changes and additions/deletions statistics.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "rev_spec": {
                                        "type": "string",
                                        "description": "Optional revision range (e.g. 'main..feat', 'HEAD~1..HEAD'). If omitted, diffs working tree."
                                    }
                                }
                            }
                        },
                        {
                            "name": "lgtm_security_scan",
                            "description": "Run AST and regex static analysis guardrails for OWASP security vulnerabilities, hardcoded secrets, cognitive complexity spikes, test neutering, and typo hallucinations.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "strict": {
                                        "type": "boolean",
                                        "description": "Flag only Critical and High severity alerts if true."
                                    }
                                }
                            }
                        },
                        {
                            "name": "lgtm_list_directives",
                            "description": "List all human review directives and comments anchored across the repository.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "lgtm_add_directive",
                            "description": "Anchor an inline review directive/comment to a specific file and line for human or agent steering.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file": { "type": "string", "description": "Relative file path" },
                                    "line": { "type": "integer", "description": "1-based line number" },
                                    "content": { "type": "string", "description": "Review directive text" }
                                },
                                "required": ["file", "line", "content"]
                            }
                        },
                        {
                            "name": "lgtm_directive_status",
                            "description": "List directives with id, resolved flag, observed outcome (untouched/changed/gone/orphaned), agent claim and a one-line verdict, plus per-turn counts.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "lgtm_review_state",
                            "description": "Summarize the human review: reviewed hunk marks for the working tree and directive outcome counts.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "lgtm_scan_changes",
                            "description": "Security/quality findings that are new since the previous call (by fingerprint). The first call reports everything.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "lgtm_resolve_directive",
                            "description": "Record that you handled a directive. This only stores a claim; the human decides whether it is resolved, and lgtm checks the claim against the diff.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "integer", "description": "Directive id from lgtm_directive_status" },
                                    "note": { "type": "string", "description": "What you changed" }
                                },
                                "required": ["id", "note"]
                            }
                        }
                    ]
                }
            })),
            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let tool_result = self.execute_tool(tool_name, &args);

                match tool_result {
                    Ok(text) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": {
                            "content": [
                                {
                                    "type": "text",
                                    "text": text
                                }
                            ]
                        }
                    })),
                    Err(err) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "error": {
                            "code": -32603,
                            "message": err.to_string()
                        }
                    })),
                }
            }
            "resources/list" => Some(json!({
                "jsonrpc": "2.0",
                "id": id_val,
                "result": {
                    "resources": [
                        {
                            "uri": "lgtm://directives",
                            "name": "Review Directives",
                            "description": "Active and claimed human review directives with line anchors and verdicts.",
                            "mimeType": "application/json"
                        },
                        {
                            "uri": "lgtm://diff",
                            "name": "Working Tree Diff",
                            "description": "Current repository git diff snapshot.",
                            "mimeType": "text/plain"
                        },
                        {
                            "uri": "lgtm://findings",
                            "name": "Security & Quality Findings",
                            "description": "Static analysis alerts and test fraud detections across working tree.",
                            "mimeType": "application/json"
                        },
                        {
                            "uri": "lgtm://review-state",
                            "name": "Review Store State",
                            "description": "Reviewed hunks, progress percentage, and persistent marks.",
                            "mimeType": "application/json"
                        }
                    ]
                }
            })),
            "resources/read" => {
                let uri = params
                    .get("uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or_default();
                match self.read_resource(uri) {
                    Ok((mime_type, text)) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": {
                            "contents": [
                                {
                                    "uri": uri,
                                    "mimeType": mime_type,
                                    "text": text
                                }
                            ]
                        }
                    })),
                    Err(e) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "error": {
                            "code": -32602,
                            "message": format!("Error reading resource {}: {}", uri, e)
                        }
                    })),
                }
            }
            "resources/subscribe" => {
                if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                    if let Ok(mut subs) = self.subscriptions.lock() {
                        subs.insert(uri.to_string());
                    }
                }
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id_val,
                    "result": {}
                }))
            }
            "resources/unsubscribe" => {
                if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                    if let Ok(mut subs) = self.subscriptions.lock() {
                        subs.remove(uri);
                    }
                }
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id_val,
                    "result": {}
                }))
            }
            _ => Some(json!({
                "jsonrpc": "2.0",
                "id": id_val,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            })),
        }
    }

    fn execute_tool(&self, name: &str, args: &Value) -> Result<String> {
        match name {
            "lgtm_diff" => {
                let snapshot = if let Some(rev) = args.get("rev_spec").and_then(|r| r.as_str()) {
                    self.git_engine.compute_revision_diff(rev)?
                } else {
                    self.git_engine.compute_diff_snapshot()?
                };
                self.format_diff_summary(&snapshot)
            }
            "lgtm_security_scan" => {
                let alerts = self.active_findings()?;
                let strict = args
                    .get("strict")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false);

                let filtered: Vec<_> = if strict {
                    alerts
                        .into_iter()
                        .filter(|a| {
                            matches!(
                                a.severity,
                                lgtm_core::guard::Severity::Critical
                                    | lgtm_core::guard::Severity::High
                            )
                        })
                        .collect()
                } else {
                    alerts
                };

                let report = serde_json::to_string_pretty(&filtered)?;
                Ok(report)
            }
            "lgtm_list_directives" => {
                let comments_file = self.git_engine.repo_path.join(".lgtm/comments.jsonl");
                if !comments_file.exists() {
                    return Ok("No active review directives found.".to_string());
                }
                let content = std::fs::read_to_string(&comments_file)?;
                Ok(content)
            }
            "lgtm_add_directive" => {
                let file = args
                    .get("file")
                    .and_then(|f| f.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'file' parameter"))?;
                let line = args
                    .get("line")
                    .and_then(|l| l.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'line' parameter"))?
                    as u32;
                let content = args
                    .get("content")
                    .and_then(|c| c.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;

                let comment =
                    lgtm_core::comment::CommentAnchor {
                        id: lgtm_core::comment::new_id(),
                        file_path: file.to_string(),
                        target_line_no: line,
                        target_code: String::new(),
                        context_before: Vec::new(),
                        context_after: Vec::new(),
                        comment_text: content.to_string(),
                        replies: Vec::new(),
                        created_at_turn: self.turn(),
                        is_resolved: false,
                        pr_number: None,
                        remote_id: None,
                        orphaned: false,
                        track: lgtm_core::outcome::Tracking {
                            hunk_hash: self.git_engine.compute_diff_snapshot().ok().and_then(|s| {
                                lgtm_core::review::hunk_hash_at(&s.files, file, line)
                            }),
                            ..Default::default()
                        },
                    };

                let comment_store =
                    lgtm_core::comment::CommentStore::new(&self.git_engine.repo_path);
                let mut comments = comment_store.load_comments();
                comments.push(comment);
                comment_store.save_comments(&comments)?;

                Ok(format!(
                    "Successfully anchored directive to {}:{}",
                    file, line
                ))
            }
            "lgtm_directive_status" => self.directive_status(),
            "lgtm_review_state" => self.review_state(),
            "lgtm_scan_changes" => self.scan_changes(),
            "lgtm_resolve_directive" => {
                let id = args
                    .get("id")
                    .and_then(|i| i.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'id' parameter"))?
                    as usize;
                let note = args
                    .get("note")
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'note' parameter"))?;
                self.claim_directive(id, note)
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    fn store(&self) -> lgtm_core::comment::CommentStore {
        lgtm_core::comment::CommentStore::new(&self.git_engine.repo_path)
    }

    fn turn(&self) -> u64 {
        lgtm_core::turn::TurnManager::new(&self.git_engine.repo_path)
            .load_state()
            .current_turn
    }

    /// Directive ids are 1-based positions in `.lgtm/comments.jsonl`.
    fn directive_status(&self) -> Result<String> {
        let comments = self.store().load_comments();
        let items: Vec<Value> = comments
            .iter()
            .enumerate()
            .map(|(i, c)| {
                json!({
                    "id": i + 1,
                    "file": c.file_path,
                    "line": c.target_line_no,
                    "text": c.comment_text,
                    "resolved": c.is_resolved,
                    "outcome": c.track.outcome,
                    "claim": c.track.claim,
                    "verdict": lgtm_core::outcome::verdict(c),
                })
            })
            .collect();
        Ok(serde_json::to_string_pretty(&json!({
            "turn": self.turn(),
            "summary": lgtm_core::outcome::Summary::of(&comments).line(),
            "directives": items,
        }))?)
    }

    fn review_state(&self) -> Result<String> {
        let comments = self.store().load_comments();
        let marks =
            lgtm_core::review::ReviewStore::open_marks_only(&self.git_engine.repo_path, "working")?
                .load()?
                .hunks
                .len();
        Ok(serde_json::to_string_pretty(&json!({
            "turn": self.turn(),
            "reviewed_hunks": marks,
            "open_directives": comments.iter().filter(|c| !c.is_resolved).count(),
            "resolved_directives": comments.iter().filter(|c| c.is_resolved).count(),
            "outcomes": lgtm_core::outcome::Summary::of(&comments).line(),
        }))?)
    }

    fn active_findings(&self) -> Result<Vec<lgtm_core::guard::GuardAlert>> {
        let snapshot = self.git_engine.compute_diff_snapshot()?;
        let user_cfg = lgtm_core::config::user_config_path(false);
        let cfg = lgtm_core::config::LgtmConfig::load(&self.repo_path, user_cfg.as_deref())?;
        let findings = lgtm_core::findings::run_in(&self.repo_path, &snapshot.files, &cfg)?;
        Ok(findings.active)
    }

    fn scan_changes(&self) -> Result<String> {
        let alerts = self.active_findings()?;
        let path = self.git_engine.repo_path.join(".lgtm/last_scan.json");
        let seen: std::collections::HashSet<String> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let new: Vec<_> = alerts
            .iter()
            .filter(|a| !seen.contains(&a.fingerprint))
            .collect();
        let now: std::collections::BTreeSet<&str> =
            alerts.iter().map(|a| a.fingerprint.as_str()).collect();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_string(&now)?)?;
        Ok(serde_json::to_string_pretty(&new)?)
    }

    /// Writes only a claim; never resolves, edits code or touches git.
    fn claim_directive(&self, id: usize, note: &str) -> Result<String> {
        let store = self.store();
        let mut comments = store.load_comments();
        let turn = self.turn();
        let c = id
            .checked_sub(1)
            .and_then(|i| comments.get_mut(i))
            .ok_or_else(|| anyhow::anyhow!("No directive with id {}", id))?;
        lgtm_core::outcome::claim(c, note, turn);
        store.save_comments(&comments)?;
        Ok(format!(
            "Claim recorded for directive {}. It stays open until the reviewer resolves it.",
            id
        ))
    }

    fn format_diff_summary(&self, snapshot: &DiffSnapshot) -> Result<String> {
        let mut out = String::new();
        out.push_str(&format!(
            "Diff Snapshot: {} changed files (+{} / -{})\n\n",
            snapshot.files.len(),
            snapshot.total_additions,
            snapshot.total_deletions
        ));

        for f in &snapshot.files {
            out.push_str(&format!(
                "File: {} [{}] (+{} -{})\n",
                f.display_name(),
                f.change_type.badge(),
                f.additions,
                f.deletions
            ));
            for h in &f.hunks {
                out.push_str(&format!("  {}\n", h.header));
                for l in &h.lines {
                    let prefix = match l.line_type {
                        lgtm_core::model::LineType::Addition => "+",
                        lgtm_core::model::LineType::Deletion => "-",
                        lgtm_core::model::LineType::Context => " ",
                        lgtm_core::model::LineType::HunkHeader => "@",
                    };
                    out.push_str(&format!("  {} {}\n", prefix, l.content));
                }
            }
            out.push('\n');
        }

        Ok(out)
    }

    fn read_resource(&self, uri: &str) -> Result<(String, String)> {
        match uri {
            "lgtm://directives" => {
                let text = self.directive_status()?;
                Ok(("application/json".to_string(), text))
            }
            "lgtm://diff" => {
                let snap = self.git_engine.compute_diff_snapshot()?;
                let text = self.format_diff_summary(&snap)?;
                Ok(("text/plain".to_string(), text))
            }
            "lgtm://findings" => {
                let active = self.active_findings()?;
                let text = serde_json::to_string_pretty(&active)?;
                Ok(("application/json".to_string(), text))
            }
            "lgtm://review-state" => {
                let text = self.review_state()?;
                Ok(("application/json".to_string(), text))
            }
            _ => Err(anyhow::anyhow!("Unknown resource URI: {}", uri)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let server = McpServer::new(".");
        let resp = server
            .handle_request("initialize", &json!({}), Some(json!(1)))
            .unwrap();
        assert_eq!(resp["result"]["serverInfo"]["name"], "lgtm");
    }

    #[test]
    fn test_mcp_tools_list() {
        let server = McpServer::new(".");
        let resp = server
            .handle_request("tools/list", &json!({}), Some(json!(2)))
            .unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "lgtm_diff"));
        assert!(tools.iter().any(|t| t["name"] == "lgtm_security_scan"));
        for n in [
            "lgtm_directive_status",
            "lgtm_review_state",
            "lgtm_scan_changes",
            "lgtm_resolve_directive",
        ] {
            assert!(tools.iter().any(|t| t["name"] == n), "{n}");
        }
    }

    fn call(s: &McpServer, name: &str, args: Value) -> Value {
        s.handle_request(
            "tools/call",
            &json!({"name": name, "arguments": args}),
            Some(json!(1)),
        )
        .unwrap()
    }

    fn text(v: &Value) -> Value {
        serde_json::from_str(v["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    fn repo() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "t@t"],
            vec!["config", "user.name", "t"],
        ] {
            std::process::Command::new("git")
                .args(&args)
                .current_dir(d.path())
                .output()
                .unwrap();
        }
        d
    }

    #[test]
    fn test_claims_are_recorded_but_never_resolve() {
        let d = repo();
        let s = McpServer::new(d.path());
        let r = call(
            &s,
            "lgtm_add_directive",
            json!({"file": "a.rs", "line": 1, "content": "fix"}),
        );
        assert!(r["result"].is_object());
        let r = call(
            &s,
            "lgtm_resolve_directive",
            json!({"id": 1, "note": "done"}),
        );
        assert!(r["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("stays open"));
        let st = text(&call(&s, "lgtm_directive_status", json!({})));
        assert_eq!(st["directives"][0]["id"], 1);
        assert_eq!(st["directives"][0]["resolved"], false);
        assert_eq!(st["directives"][0]["claim"]["note"], "done");
        assert_eq!(st["directives"][0]["verdict"], "claimed");
        for bad in [
            json!({"id": 9, "note": "x"}),
            json!({"id": 0, "note": "x"}),
            json!({"id": 1}),
            json!({"note": "x"}),
        ] {
            assert!(call(&s, "lgtm_resolve_directive", bad)["error"].is_object());
        }
    }

    #[test]
    fn test_review_state_and_scan_changes() {
        let d = repo();
        std::fs::write(
            d.path().join("a.py"),
            "import requests\nrequests.get(u, verify=False)\n",
        )
        .unwrap();
        std::process::Command::new("git")
            .args(["add", "-N", "a.py"])
            .current_dir(d.path())
            .output()
            .unwrap();
        let s = McpServer::new(d.path());
        let rs = text(&call(&s, "lgtm_review_state", json!({})));
        assert_eq!(
            (
                rs["reviewed_hunks"].as_u64(),
                rs["open_directives"].as_u64()
            ),
            (Some(0), Some(0))
        );
        let first = text(&call(&s, "lgtm_scan_changes", json!({})));
        assert!(!first.as_array().unwrap().is_empty());
        let second = text(&call(&s, "lgtm_scan_changes", json!({})));
        assert!(second.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_mcp_resources_and_subscriptions() {
        let d = repo();
        let s = McpServer::new(d.path());

        // resources/list
        let list_resp = s
            .handle_request("resources/list", &json!({}), Some(json!(10)))
            .unwrap();
        let resources = list_resp["result"]["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 4);
        let uris: Vec<_> = resources
            .iter()
            .map(|r| r["uri"].as_str().unwrap())
            .collect();
        assert!(uris.contains(&"lgtm://directives"));
        assert!(uris.contains(&"lgtm://diff"));
        assert!(uris.contains(&"lgtm://findings"));
        assert!(uris.contains(&"lgtm://review-state"));

        // resources/subscribe
        let sub_resp = s
            .handle_request(
                "resources/subscribe",
                &json!({ "uri": "lgtm://directives" }),
                Some(json!(11)),
            )
            .unwrap();
        assert!(sub_resp["result"].is_object());
        assert!(s
            .subscriptions
            .lock()
            .unwrap()
            .contains("lgtm://directives"));

        // resources/read
        let read_resp = s
            .handle_request(
                "resources/read",
                &json!({ "uri": "lgtm://directives" }),
                Some(json!(12)),
            )
            .unwrap();
        let contents = read_resp["result"]["contents"].as_array().unwrap();
        assert_eq!(contents[0]["uri"], "lgtm://directives");
        assert_eq!(contents[0]["mimeType"], "application/json");

        // resources/unsubscribe
        let unsub_resp = s
            .handle_request(
                "resources/unsubscribe",
                &json!({ "uri": "lgtm://directives" }),
                Some(json!(13)),
            )
            .unwrap();
        assert!(unsub_resp["result"].is_object());
        assert!(!s
            .subscriptions
            .lock()
            .unwrap()
            .contains("lgtm://directives"));
    }

    #[test]
    fn test_affected_resources_classification() {
        let code_path = PathBuf::from("src/main.rs");
        let aff1 = McpServer::affected_resources(&[code_path]);
        assert!(aff1.contains(&"lgtm://diff"));
        assert!(aff1.contains(&"lgtm://findings"));

        let comments_path = PathBuf::from(".lgtm/comments.jsonl");
        let aff2 = McpServer::affected_resources(&[comments_path]);
        assert!(aff2.contains(&"lgtm://directives"));
        assert!(aff2.contains(&"lgtm://review-state"));
    }

    #[test]
    fn test_mcp_findings_respect_config_and_baseline() {
        let d = repo();
        let line = format!(
            "import requests\nrequests.get(u, {})\n",
            ["verify=", "False"].concat()
        );
        std::fs::write(d.path().join("a.py"), &line).unwrap();
        std::fs::write(d.path().join("ignored.py"), &line).unwrap();
        std::fs::write(
            d.path().join(".lgtm.toml"),
            "[ignore]\npaths = [\"ignored.py\"]\n",
        )
        .unwrap();
        std::process::Command::new("git")
            .args(["add", "-N", "a.py", "ignored.py"])
            .current_dir(d.path())
            .output()
            .unwrap();

        let s = McpServer::new(d.path());
        let scan = text(&call(&s, "lgtm_security_scan", json!({})));
        let arr = scan.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["file_path"], "a.py");

        // Baseline the finding on a.py and verify MCP tools and resource suppress it
        let cfg = lgtm_core::config::LgtmConfig::load(d.path(), None).unwrap();
        let (_, unbaselined) =
            lgtm_core::scan::FullRepoScanner::scan_unbaselined(d.path(), &cfg).unwrap();
        let bl_path = lgtm_core::baseline::path_for(d.path(), &cfg);
        let mut bl = lgtm_core::baseline::Baseline::load(&bl_path).unwrap();
        bl.update(&unbaselined, false, false);
        bl.save(&bl_path).unwrap();

        let scan_after = text(&call(&s, "lgtm_security_scan", json!({})));
        assert!(scan_after.as_array().unwrap().is_empty());

        let changes_after = text(&call(&s, "lgtm_scan_changes", json!({})));
        assert!(changes_after.as_array().unwrap().is_empty());

        let res = s
            .handle_request(
                "resources/read",
                &json!({ "uri": "lgtm://findings" }),
                Some(json!(14)),
            )
            .unwrap();
        let res_findings: Value =
            serde_json::from_str(res["result"]["contents"][0]["text"].as_str().unwrap()).unwrap();
        assert!(res_findings.as_array().unwrap().is_empty());
    }
}
