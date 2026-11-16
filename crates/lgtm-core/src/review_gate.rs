/// Pure `## Review Gate` markdown checkbox helpers.
/// If `line_1based` in `content` is an unticked `- [ ]` item inside a `## Review Gate`
/// section, returns `Some(updated_content)` with the first `[ ]` on that line replaced by `[x]`.
/// Preserves LF and CRLF line endings. Returns `None` otherwise.
pub fn tick_review_gate_line(content: &str, line_1based: u32) -> Option<String> {
    let target_idx = line_1based.checked_sub(1)? as usize;
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    if target_idx >= lines.len() {
        return None;
    }

    let mut in_review_gate = false;
    for (idx, line) in lines.iter().enumerate() {
        let body = line.trim_end_matches(['\r', '\n']);
        let trimmed = body.trim();
        if trimmed == "## Review Gate" {
            in_review_gate = true;
        } else if trimmed.trim_start_matches('#').starts_with(' ') {
            in_review_gate = false;
        }

        if idx == target_idx {
            if !in_review_gate {
                return None;
            }
            let trimmed_start = body.trim_start();
            if trimmed_start == "- [ ]" || trimmed_start.starts_with("- [ ] ") {
                let mut out = String::with_capacity(content.len());
                for (i, part) in lines.iter().enumerate() {
                    if i == idx {
                        out.push_str(&part.replacen("[ ]", "[x]", 1));
                    } else {
                        out.push_str(part);
                    }
                }
                return Some(out);
            }
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_unticked_checkbox_inside_review_gate() {
        let doc =
            "# Proposal\n\n## Tasks\n- [ ] Task 1\n\n## Review Gate\n- [ ] **Gate 1**: Approve\n";
        let updated = tick_review_gate_line(doc, 7).expect("should tick gate line");
        assert_eq!(
            updated,
            "# Proposal\n\n## Tasks\n- [ ] Task 1\n\n## Review Gate\n- [x] **Gate 1**: Approve\n"
        );
    }

    #[test]
    fn ticks_indented_checkbox_and_heading_with_trailing_spaces() {
        let doc = "## Review Gate   \n  - [ ] Gate item\n";
        let updated = tick_review_gate_line(doc, 2).expect("should tick indented gate line");
        assert_eq!(updated, "## Review Gate   \n  - [x] Gate item\n");
    }

    #[test]
    fn returns_none_for_already_ticked_or_non_checkbox_line() {
        let doc = "## Review Gate\n- [x] Done\nSome text\n";
        assert_eq!(tick_review_gate_line(doc, 2), None);
        assert_eq!(tick_review_gate_line(doc, 3), None);
    }

    #[test]
    fn returns_none_outside_review_gate_and_resets_on_next_heading() {
        let doc = "## Tasks\n- [ ] Task 1\n## Review Gate\n#tag-not-a-heading\n- [ ] Gate\n## Notes\n- [ ] Note\n";
        assert_eq!(tick_review_gate_line(doc, 2), None);
        assert!(tick_review_gate_line(doc, 5).is_some());
        assert_eq!(tick_review_gate_line(doc, 7), None);
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let doc = "## Review Gate\r\n- [ ] Gate 1\r\n- [ ] Gate 2\r\n";
        let updated = tick_review_gate_line(doc, 2).expect("should tick CRLF gate line");
        assert_eq!(
            updated,
            "## Review Gate\r\n- [x] Gate 1\r\n- [ ] Gate 2\r\n"
        );
    }

    #[test]
    fn returns_none_for_zero_or_out_of_bounds_line() {
        let doc = "## Review Gate\n- [ ] Gate 1\n";
        assert_eq!(tick_review_gate_line(doc, 0), None);
        assert_eq!(tick_review_gate_line(doc, 3), None);
        assert_eq!(tick_review_gate_line(doc, 99), None);
    }

    #[test]
    fn ticks_bare_checkbox_without_trailing_text() {
        let doc = "## Review Gate\n- [ ]\n";
        let updated = tick_review_gate_line(doc, 2).expect("should tick bare checkbox");
        assert_eq!(updated, "## Review Gate\n- [x]\n");
    }
}
