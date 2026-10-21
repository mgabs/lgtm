use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Python,
    Ts,
    Tsx,
    Swift,
    Go,
}

/// JS/JSX parse with the TSX grammar, which is a superset.
pub fn lang_of(path: &Path) -> Option<Lang> {
    match path.extension()?.to_str()? {
        "rs" => Some(Lang::Rust),
        "py" | "pyi" => Some(Lang::Python),
        "ts" | "mts" | "cts" => Some(Lang::Ts),
        "swift" => Some(Lang::Swift),
        "tsx" | "jsx" | "js" | "mjs" | "cjs" => Some(Lang::Tsx),
        "go" => Some(Lang::Go),
        _ => None,
    }
}

fn grammar(lang: Lang) -> Language {
    match lang {
        Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
        Lang::Python => tree_sitter_python::LANGUAGE.into(),
        Lang::Ts => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Lang::Swift => tree_sitter_swift::LANGUAGE.into(),
        Lang::Go => tree_sitter_go::LANGUAGE.into(),
    }
}

/// 0-based rows where `query` captures `@hit`. Broken code still parses
/// (tree-sitter recovers), so partial agent output is fine.
pub fn hits(lang: Lang, src: &str, query: &str) -> Result<Vec<usize>> {
    hits_within(lang, src, query, &[])
}

pub fn sexp(lang: Lang, src: &str) -> Result<String> {
    let mut parser = Parser::new();
    parser.set_language(&grammar(lang))?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("parse failed"))?;
    Ok(tree.root_node().to_sexp())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstBlock {
    pub start_row: usize,
    pub end_row: usize,
    pub kind: String,
}

const FOLD_BLOCK_KINDS: &[&str] = &[
    "function_item",
    "impl_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "mod_item",
    "function_definition",
    "class_definition",
    "with_statement",
    "try_statement",
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "class_declaration",
    "interface_declaration",
    "statement_block",
    "object_type",
    "code_block",
    "method_declaration",
    "type_declaration",
    "block",
    "compound_statement",
    "if_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "match_expression",
    "switch_statement",
];

/// Extracts multi-line foldable AST blocks (functions, classes, blocks, control flows)
/// sorted by start_row ascending and longest span first.
pub fn fold_blocks(lang: Lang, src: &str) -> Result<Vec<AstBlock>> {
    let mut parser = Parser::new();
    parser.set_language(&grammar(lang))?;
    let tree = parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("parse failed"))?;
    let mut blocks = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(n) = stack.pop() {
        let s_row = n.start_position().row;
        let e_row = n.end_position().row;
        if e_row > s_row && FOLD_BLOCK_KINDS.contains(&n.kind()) {
            blocks.push(AstBlock {
                start_row: s_row,
                end_row: e_row,
                kind: n.kind().to_string(),
            });
        }
        let mut cur = n.walk();
        for child in n.children(&mut cur) {
            stack.push(child);
        }
    }
    blocks.sort_by(|a, b| {
        a.start_row
            .cmp(&b.start_row)
            .then_with(|| b.end_row.cmp(&a.end_row))
    });
    blocks.dedup_by(|a, b| a.start_row == b.start_row && a.end_row == b.end_row);
    Ok(blocks)
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

const BRANCH_KINDS: [&str; 20] = [
    "if_statement",
    "if_expression",
    "guard_statement",
    "for_statement",
    "for_in_statement",
    "for_expression",
    "while_statement",
    "while_expression",
    "loop_expression",
    "do_statement",
    "repeat_while_statement",
    "switch_statement",
    "catch_clause",
    "catch_block",
    "ternary_expression",
    "conditional_expression",
    "switch_case",
    "match_arm",
    "else_clause",
    "except_clause",
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

const IDENT_KINDS: [&str; 9] = [
    "identifier",
    "property_identifier",
    "type_identifier",
    "shorthand_property_identifier",
    "simple_identifier",
    "field_identifier",
    "scoped_identifier",
    "primitive_type",
    "dotted_name",
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
        assert_eq!(lang_of(Path::new("a.rs")), Some(Lang::Rust));
        assert_eq!(lang_of(Path::new("a.py")), Some(Lang::Python));
        assert_eq!(lang_of(Path::new("a.pyi")), Some(Lang::Python));
        assert_eq!(lang_of(Path::new("a.go")), Some(Lang::Go));
        assert_eq!(lang_of(Path::new("a.rb")), None);
    }

    #[test]
    fn go_identifiers_skip_strings_and_comments() {
        let src = "// comment_word\nvar userName = \"string_word\"\nfunc CalculateTotal() {\n    return userName\n}\n";
        let ids = identifiers(Lang::Go, src).unwrap();
        let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"userName"));
        assert!(names.contains(&"CalculateTotal"));
        assert!(!names
            .iter()
            .any(|n| n.contains("comment_word") || n.contains("string_word")));
    }

    #[test]
    fn rust_identifiers_skip_strings_and_comments() {
        let src = "// commentDirective\nlet comment_input = \"directive string\";\nActiveView::CommentInput;\n";
        let ids = identifiers(Lang::Rust, src).unwrap();
        let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"comment_input"));
        assert!(names.contains(&"CommentInput"));
        assert!(names.contains(&"ActiveView"));
        assert!(!names
            .iter()
            .any(|n| n.contains("directive") || n.contains("commentDirective")));
    }

    #[test]
    fn python_identifiers_skip_strings_and_comments() {
        let src = "# comment_word\nuser_name = \"docstring_word\"\ndef calculate_total():\n    return user_name\n";
        let ids = identifiers(Lang::Python, src).unwrap();
        let names: Vec<_> = ids.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"user_name"));
        assert!(names.contains(&"calculate_total"));
        assert!(!names
            .iter()
            .any(|n| n.contains("comment_word") || n.contains("docstring_word")));
    }

    #[test]
    fn fold_blocks_extracts_multiline_ranges() {
        let rust_src = "fn compute(a: i32) -> i32 {\n    let b = a * 2;\n    if b > 5 {\n        b + 1\n    } else {\n        b - 1\n    }\n}\n";
        let blocks = fold_blocks(Lang::Rust, rust_src).unwrap();
        assert!(!blocks.is_empty());
        assert_eq!(blocks[0].start_row, 0);
        assert_eq!(blocks[0].end_row, 7);

        let py_src = "def process(items):\n    for item in items:\n        print(item)\n";
        let py_blocks = fold_blocks(Lang::Python, py_src).unwrap();
        assert!(!py_blocks.is_empty());
        assert_eq!(py_blocks[0].start_row, 0);
        assert_eq!(py_blocks[0].end_row, 2);

        let go_src = "func Handler(w ResponseWriter, r *Request) {\n    if r.Method == \"POST\" {\n        doPost()\n    }\n}\n";
        let go_blocks = fold_blocks(Lang::Go, go_src).unwrap();
        assert!(!go_blocks.is_empty());
        assert_eq!(go_blocks[0].start_row, 0);
        assert_eq!(go_blocks[0].end_row, 4);
    }
}
