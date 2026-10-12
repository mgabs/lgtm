use anyhow::Result;
use lgtm_core::git::GitEngine;
use lgtm_core::guard::SecurityAndTestGuard;
use lgtm_core::model::DiffSnapshot;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::Path;

pub struct McpServer {
    git_engine: GitEngine,
}

impl McpServer {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            git_engine: GitEngine::new(repo_path),
        }
    }

    pub fn run_stdio(&self) -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

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
                    writeln!(stdout, "{}", err_response)?;
                    stdout.flush()?;
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
                writeln!(stdout, "{}", resp)?;
                stdout.flush()?;
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
                        "tools": {}
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
                let snapshot = self.git_engine.compute_diff_snapshot()?;
                let alerts = SecurityAndTestGuard::scan_snapshot(&snapshot.files);
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

                let comment = lgtm_core::comment::CommentAnchor {
                    file_path: file.to_string(),
                    target_line_no: line,
                    target_code: String::new(),
                    context_before: Vec::new(),
                    context_after: Vec::new(),
                    comment_text: content.to_string(),
                    replies: Vec::new(),
                    created_at_turn: 1,
                    is_resolved: false,
                    pr_number: None,
                    remote_id: None,
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
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
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
    }
}
