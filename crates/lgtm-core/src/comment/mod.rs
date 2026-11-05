use serde::{Deserialize, Serialize};

// Re-export directive types for backward compatibility during migration
pub use crate::directive::{new_id, CommentAnchor, CommentStore, Directive, DirectiveStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum CommentSide {
    #[default]
    Right,
    Left,
}

impl CommentSide {
    pub fn as_github_str(&self) -> &'static str {
        match self {
            Self::Right => "RIGHT",
            Self::Left => "LEFT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment_side_serde() {
        let json = r#"{"side":"LEFT"}"#;
        #[derive(Deserialize)]
        struct S {
            side: CommentSide,
        }
        let s: S = serde_json::from_str(json).unwrap();
        assert_eq!(s.side, CommentSide::Left);
        assert_eq!(s.side.as_github_str(), "LEFT");
        assert_eq!(CommentSide::Right.as_github_str(), "RIGHT");
    }
}
