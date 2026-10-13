use anyhow::{anyhow, Result};
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ts,
    Tsx,
    Swift,
}

/// JS/JSX parse with the TSX grammar, which is a superset.
pub fn lang_of(path: &Path) -> Option<Lang> {
    match path.extension()?.to_str()? {
        "ts" | "mts" | "cts" => Some(Lang::Ts),
        "swift" => Some(Lang::Swift),
        "tsx" | "jsx" | "js" | "mjs" | "cjs" => Some(Lang::Tsx),
        _ => None,
    }
}

fn grammar(lang: Lang) -> Language {
    match lang {
        Lang::Ts => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Lang::Swift => tree_sitter_swift::LANGUAGE.into(),
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

const BRANCH_KINDS: [&str; 14] = [
    "if_statement",
    "guard_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "repeat_while_statement",
    "switch_statement",
    "catch_clause",
    "catch_block",
    "ternary_expression",
    "conditional_expression",
    "switch_case",
    "else_clause",
];

/// Cognitive complexity of branches that start on a row `counted` accepts:
/// each branch costs 1 plus its nesting depth; `else if` adds no depth.
pub fn cognitive(lang: Lang, src: &str, counted: impl Fn(usize) -> bool) -> Result<usize> {
    let mut parser = Parser::new();
    parser.set_language(&grammar(lang))?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("parse failed"))?;
    Ok(walk(tree.root_node(), 0, &counted))
}

fn walk(node: tree_sitter::Node, depth: usize, counted: &impl Fn(usize) -> bool) -> usize {
    let kind = node.kind();
    // case/else arms only deepen nothing; the switch / if above already paid
    let scores = BRANCH_KINDS.contains(&kind) && kind != "switch_case" && kind != "else_clause";
    let else_if =
        kind == "if_statement" && node.parent().is_some_and(|p| p.kind() == "else_clause");
    let mut total = 0;
    if scores && counted(node.start_position().row) {
        total += if else_if { 1 } else { 1 + depth };
    }
    let child_depth = if scores && !else_if { depth + 1 } else { depth };
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        total += walk(c, child_depth, counted);
    }
    total
}

const IDENT_KINDS: [&str; 5] = [
    "identifier",
    "property_identifier",
    "type_identifier",
    "shorthand_property_identifier",
    "simple_identifier",
];

/// Identifiers (not strings or comments) with the row each starts on.
pub fn identifiers(lang: Lang, src: &str) -> Result<Vec<(String, usize)>> {
    let mut parser = Parser::new();
    parser.set_language(&grammar(lang))?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("parse failed"))?;
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        if IDENT_KINDS.contains(&n.kind()) {
            out.push((src[n.byte_range()].to_string(), n.start_position().row));
        }
        let mut cur = n.walk();
        stack.extend(n.children(&mut cur));
    }
    Ok(out)
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
    fn cognitive_counts_nesting_and_skips_else_if_depth() {
        let src = "function f() {\n  if (a) {\n    for (;;) {\n      if (b) {}\n    }\n  } else if (c) {}\n}\n";
        // if=1, for=1+1, inner if=1+2, else-if=1
        assert_eq!(cognitive(Lang::Ts, src, |_| true).unwrap(), 1 + 2 + 3 + 1);
        assert_eq!(cognitive(Lang::Ts, src, |r| r == 3).unwrap(), 3);
        let jsx = "const x = <div style={{ a: 1 }}>{items.map((i) => <b key={i}>{i}</b>)}</div>;";
        assert_eq!(cognitive(Lang::Tsx, jsx, |_| true).unwrap(), 0);
        let sw = "func f() {\n  guard a else { return }\n  if b { while c {} }\n}\n";
        assert_eq!(cognitive(Lang::Swift, sw, |_| true).unwrap(), 1 + 1 + 2);
    }

    #[test]
    fn identifiers_skip_strings_and_comments() {
        let src = "// commentWord\nconst userName = \"stringWord\";\nuserName.fooBar();\n";
        let ids = identifiers(Lang::Ts, src).unwrap();
        let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"userName") && names.contains(&"fooBar"));
        assert!(!names.iter().any(|n| n.ends_with("Word")));
        assert_eq!(
            ids.iter()
                .filter(|(n, r)| n == "userName" && *r == 2)
                .count(),
            1
        );
    }

    #[test]
    fn maps_extensions() {
        assert_eq!(lang_of(Path::new("a/b.tsx")), Some(Lang::Tsx));
        assert_eq!(lang_of(Path::new("a.js")), Some(Lang::Tsx));
        assert_eq!(lang_of(Path::new("a.ts")), Some(Lang::Ts));
        assert_eq!(lang_of(Path::new("a.swift")), Some(Lang::Swift));
        assert_eq!(lang_of(Path::new("a.rs")), None);
    }
}
