use anyhow::{anyhow, Result};
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ts,
    Tsx,
}

/// JS/JSX parse with the TSX grammar, which is a superset.
pub fn lang_of(path: &Path) -> Option<Lang> {
    match path.extension()?.to_str()? {
        "ts" | "mts" | "cts" => Some(Lang::Ts),
        "tsx" | "jsx" | "js" | "mjs" | "cjs" => Some(Lang::Tsx),
        _ => None,
    }
}

fn grammar(lang: Lang) -> Language {
    match lang {
        Lang::Ts => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
    }
}

/// 0-based rows where `query` captures `@hit`. Broken code still parses
/// (tree-sitter recovers), so partial agent output is fine.
pub fn hits(lang: Lang, src: &str, query: &str) -> Result<Vec<usize>> {
    hits_within(lang, src, query, &[])
}

const FUNCTION_KINDS: [&str; 5] = [
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "generator_function_declaration",
];

/// Like `hits`, but when `inside` is non-empty a hit only counts if an
/// ancestor of that kind exists before the enclosing function boundary.
pub fn hits_within(lang: Lang, src: &str, query: &str, inside: &[&str]) -> Result<Vec<usize>> {
    let language = grammar(lang);
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("parse failed"))?;
    let q = Query::new(&language, query)?;
    let idx = q
        .capture_index_for_name("hit")
        .ok_or_else(|| anyhow!("query needs a @hit capture"))?;
    let mut cursor = QueryCursor::new();
    let mut rows = Vec::new();
    let mut it = cursor.matches(&q, tree.root_node(), src.as_bytes());
    while let Some(m) = it.next() {
        for c in m.captures.iter().filter(|c| c.index == idx) {
            if inside.is_empty() || within(c.node, inside) {
                rows.push(c.node.start_position().row);
            }
        }
    }
    rows.sort_unstable();
    rows.dedup();
    Ok(rows)
}

fn within(node: tree_sitter::Node, kinds: &[&str]) -> bool {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if kinds.contains(&n.kind()) {
            return true;
        }
        if FUNCTION_KINDS.contains(&n.kind()) {
            return false;
        }
        cur = n.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q: &str =
        r#"(jsx_attribute (property_identifier) @n (#eq? @n "dangerouslySetInnerHTML")) @hit"#;

    #[test]
    fn finds_jsx_attribute_rows() {
        let src = "const a = 1;\nconst x = <div\n  dangerouslySetInnerHTML={{ __html: h }} />;\n";
        assert_eq!(hits(Lang::Tsx, src, Q).unwrap(), vec![2]);
        assert!(hits(Lang::Tsx, "const x = <div className=\"a\" />;", Q)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn broken_code_still_parses_and_bad_query_errors() {
        let src = "}\nconst x = <p dangerouslySetInnerHTML={x} />;\n";
        assert_eq!(hits(Lang::Tsx, src, Q).unwrap(), vec![1]);
        assert!(hits(Lang::Tsx, "x", "(nope) @hit").is_err());
        assert!(hits(Lang::Tsx, "x", "(identifier) @other").is_err());
    }

    #[test]
    fn within_stops_at_function_boundary() {
        let q = r#"(call_expression function: (identifier) @hit (#match? @hit "^use[A-Z]"))"#;
        let src = "function A() {\n  if (x) {\n    useA();\n  }\n  useB();\n  if (y) {\n    const f = () => useC();\n  }\n}\n";
        assert_eq!(
            hits_within(Lang::Tsx, src, q, &["if_statement"]).unwrap(),
            vec![2]
        );
        assert_eq!(hits(Lang::Tsx, src, q).unwrap(), vec![2, 4, 6]);
    }

    #[test]
    fn maps_extensions() {
        assert_eq!(lang_of(Path::new("a/b.tsx")), Some(Lang::Tsx));
        assert_eq!(lang_of(Path::new("a.js")), Some(Lang::Tsx));
        assert_eq!(lang_of(Path::new("a.ts")), Some(Lang::Ts));
        assert_eq!(lang_of(Path::new("a.rs")), None);
    }
}
