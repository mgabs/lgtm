//! Safe command failures: inspect provider output in memory, retain only a category and codes.
use anyhow::Result;
use std::{fmt, path::Path, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    MissingRef,
    Authentication,
    AccessDenied,
    RepositoryUnavailable,
    Timeout,
    Network,
    Tls,
    Http,
    ToolUnavailable,
    Other,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingRef => "missing_ref",
            Self::Authentication => "authentication",
            Self::AccessDenied => "access_denied",
            Self::RepositoryUnavailable => "repository_unavailable",
            Self::Timeout => "timeout",
            Self::Network => "network",
            Self::Tls => "tls",
            Self::Http => "http",
            Self::ToolUnavailable => "tool_unavailable",
            Self::Other => "command_failed",
        }
    }
}
#[derive(Debug)]
pub struct Failure {
    pub kind: Kind,
    pub exit_code: Option<i32>,
    pub http_status: Option<u16>,
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let advice = match self.kind {
            Kind::MissingRef => "requested Git ref or commit is unavailable",
            Kind::Authentication => {
                "authentication failed; check your provider login and Git credentials"
            }
            Kind::AccessDenied => "access denied; check repository permissions",
            Kind::RepositoryUnavailable => {
                "repository is unavailable; check origin and repository access"
            }
            Kind::Timeout => "request timed out; check connectivity and retry",
            Kind::Network => "network connection failed; check connectivity and retry",
            Kind::Tls => "TLS verification failed; check your certificate configuration",
            Kind::Http => "provider HTTP request failed",
            Kind::ToolUnavailable => {
                "required command could not start; check installation and PATH"
            }
            Kind::Other => "provider command failed",
        };
        write!(f, "{advice}")?;
        if let Some(status) = self.http_status {
            write!(f, " (HTTP {status})")?;
        }
        if let Some(code) = self.exit_code {
            write!(f, " (exit {code})")?;
        }
        Ok(())
    }
}
impl std::error::Error for Failure {}

pub fn classify(program: &str, code: Option<i32>, stderr: &str) -> Failure {
    let text = stderr.to_ascii_lowercase();
    let http_status = [
        "returned error:",
        "http ",
        "http/",
        "http error ",
        "status code:",
        "statuscode\":",
    ]
    .iter()
    .find_map(|marker| {
        text.split_once(marker).and_then(|(_, tail)| {
            tail.split(|c: char| !c.is_ascii_digit())
                .filter_map(|s| s.parse::<u16>().ok())
                .find(|n| (400..600).contains(n))
        })
    });
    let kind = if text.contains("couldn't find remote ref")
        || text.contains("could not find remote ref")
        || text.contains("remote ref does not exist")
        || text.contains("not our ref")
        || text.contains("unadvertised object")
    {
        Kind::MissingRef
    } else if (program == "gh" && (code == Some(4) || text.contains("gh auth login")))
        || http_status == Some(401)
        || [
            "authentication failed",
            "permission denied (publickey",
            "could not read username",
            "terminal prompts disabled",
            "az login",
            "not logged",
            "tf400813",
        ]
        .iter()
        .any(|s| text.contains(s))
    {
        Kind::Authentication
    } else if http_status == Some(403) || text.contains("access denied") {
        Kind::AccessDenied
    } else if http_status == Some(404) || text.contains("repository not found") {
        Kind::RepositoryUnavailable
    } else if (program == "curl" && code == Some(28))
        || text.contains("timed out")
        || text.contains("timeout")
    {
        Kind::Timeout
    } else if (program == "curl" && matches!(code, Some(35 | 51 | 60)))
        || text.contains("certificate")
    {
        Kind::Tls
    } else if (program == "curl" && matches!(code, Some(5 | 6 | 7 | 52 | 56)))
        || [
            "could not resolve",
            "couldn't resolve",
            "connection refused",
            "network is unreachable",
            "connection reset",
        ]
        .iter()
        .any(|s| text.contains(s))
    {
        Kind::Network
    } else if http_status.is_some() {
        Kind::Http
    } else {
        Kind::Other
    };
    Failure {
        kind,
        exit_code: code,
        http_status,
    }
}

pub fn missing_ref(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<Failure>()
        .is_some_and(|e| e.kind == Kind::MissingRef)
}

pub fn is_ambiguous_write(error: &anyhow::Error) -> bool {
    if error
        .downcast_ref::<Failure>()
        .is_some_and(|e| e.kind == Kind::Timeout)
    {
        return true;
    }
    let text = format!("{error:#}").to_ascii_lowercase();
    text.contains("timed out")
        || text.contains("timeout")
        || text.contains("no receipt")
        || text.contains("no comment id")
        || text.contains("no thread id")
        || text.contains("no confirmation")
}

pub fn operation(program: &str, args: &[&str]) -> &'static str {
    match program {
        "git" if args.first() == Some(&"fetch") => {
            if args.iter().any(|a| a.contains("/merge:")) {
                "git.fetch_merge_ref"
            } else if args.iter().any(|a| {
                a.starts_with('+')
                    && a.split(':')
                        .next()
                        .is_some_and(|sha| git2::Oid::from_str(sha.trim_start_matches('+')).is_ok())
            }) {
                "git.fetch_commits"
            } else if args.iter().any(|a| a.contains("refs/pull/")) {
                "git.fetch_head_ref"
            } else {
                "git.fetch_branches"
            }
        }
        "git" => "command.git",
        "curl" => "command.curl",
        "az" => "command.az",
        "gh" => "command.gh",
        _ => "command.other",
    }
}

pub fn run(repo: &Path, program: &str, args: &[&str]) -> Result<String> {
    let operation = operation(program, args);
    crate::debug::measure(operation, || {
        let resolved = crate::sys::resolve_program(program);
        let mut command = Command::new(resolved.as_ref());
        if program == "curl" {
            command.args(["--connect-timeout", "10", "--max-time", "30"]);
        }
        let out = command
            .args(args)
            .current_dir(repo)
            .env("AZURE_CORE_COLLECT_TELEMETRY", "false")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .output();
        let failure = match out {
            Ok(out) if out.status.success() => {
                return Ok(String::from_utf8_lossy(&out.stdout).into_owned())
            }
            Ok(out) => classify(
                program,
                out.status.code(),
                &String::from_utf8_lossy(&out.stderr),
            ),
            Err(_) => Failure {
                kind: Kind::ToolUnavailable,
                exit_code: None,
                http_status: None,
            },
        };
        crate::debug::command_failure(
            operation,
            failure.kind.label(),
            failure.exit_code,
            failure.http_status,
        );
        Err(failure.into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn categories_and_safe_messages() {
        for (program, code, text, kind) in [
            (
                "git",
                128,
                "fatal: couldn't find remote ref refs/pull/1/merge",
                Kind::MissingRef,
            ),
            (
                "git",
                128,
                "Authentication failed for https://user:secret@host",
                Kind::Authentication,
            ),
            (
                "git",
                128,
                "Repository not found",
                Kind::RepositoryUnavailable,
            ),
            (
                "curl",
                22,
                "curl: (22) The requested URL returned error: 403",
                Kind::AccessDenied,
            ),
            (
                "curl",
                22,
                "curl: (22) The requested URL returned error: 401",
                Kind::Authentication,
            ),
            ("curl", 28, "secret body", Kind::Timeout),
            ("curl", 6, "secret host", Kind::Network),
            ("curl", 60, "secret cert", Kind::Tls),
        ] {
            let failure = classify(program, Some(code), text);
            assert_eq!(failure.kind, kind);
            assert!(!failure.to_string().contains("secret"));
            assert_eq!(missing_ref(&failure.into()), kind == Kind::MissingRef);
        }
        assert!(!missing_ref(&anyhow::anyhow!("unknown failure")));
    }
}
