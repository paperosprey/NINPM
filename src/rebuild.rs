use anyhow::{bail, Result};

pub const TARGET: &str = "environment.systemPackages = with pkgs; [";

#[derive(Debug, Clone, Copy)]
pub struct ItemSpan {
    pub start: usize,
    pub end: usize,
}

pub fn find_bracket_range(content: &str) -> Result<(usize, usize)> {
    let target_pos = content
        .find(TARGET)
        .ok_or_else(|| anyhow::anyhow!("could not find `{TARGET}` in the config file"))?;
    let open_idx = content[target_pos..]
        .find('[')
        .map(|i| target_pos + i)
        .ok_or_else(|| anyhow::anyhow!("found systemPackages but no opening `[`"))?;

    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    for (i, c) in content[open_idx..].char_indices() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '#' => in_comment = true,
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((open_idx, open_idx + i));
                }
                if depth < 0 {
                    bail!("unbalanced `]` while scanning systemPackages list");
                }
            }
            _ => {}
        }
    }

    bail!("systemPackages list was never closed (missing `]`)")
}

pub fn parse_items(items_text: &str) -> Vec<ItemSpan> {
    let mut items = Vec::new();
    let mut depth = 0i32;
    let mut item_start: Option<usize> = None;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    for (idx, c) in items_text.char_indices() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            if item_start.is_none() {
                item_start = Some(idx);
            }
            continue;
        }
        match c {
            '#' => {
                in_comment = true;
                continue;
            }
            '"' => {
                in_string = true;
                if item_start.is_none() {
                    item_start = Some(idx);
                }
                continue;
            }
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => depth -= 1,
            _ => {}
        }

        if depth == 0 && c.is_whitespace() {
            if let Some(start) = item_start.take() {
                items.push(ItemSpan { start, end: idx });
            }
        } else if item_start.is_none() {
            item_start = Some(idx);
        }
    }

    if let Some(start) = item_start {
        items.push(ItemSpan {
            start,
            end: items_text.len(),
        });
    }

    let mut merged: Vec<ItemSpan> = Vec::new();
    for span in items {
        let starts_with_paren = items_text[span.start..span.end]
            .trim_start()
            .starts_with('(');
        if starts_with_paren {
            if let Some(last) = merged.last_mut() {
                last.end = span.end;
                continue;
            }
        }
        merged.push(span);
    }

    merged
}

pub fn canonical_path(raw_item: &str) -> &str {
    let trimmed = raw_item.trim();
    let after_prefix = trimmed.strip_prefix("pkgs.").unwrap_or(trimmed);
    let end = after_prefix
        .find(|c: char| c == '(' || c == '{' || c.is_whitespace())
        .unwrap_or(after_prefix.len());
    after_prefix[..end].trim_end_matches(';')
}

pub fn first_segment(canonical: &str) -> &str {
    let end = canonical.find('.').unwrap_or(canonical.len());
    &canonical[..end]
}

pub fn item_matches(raw_item: &str, target: &str) -> bool {
    let canon = canonical_path(raw_item);
    canon == target || first_segment(canon) == target
}

pub fn flatten_for_display(raw_item: &str) -> String {
    raw_item.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_path_removes_pkgs_prefix() {
        assert_eq!(canonical_path("pkgs.neovim"), "neovim");
        assert_eq!(canonical_path("  pkgs.vim  "), "vim");
    }

    #[test]
    fn canonical_path_handles_complex_expressions() {
        assert_eq!(
            canonical_path("pkgs.python3.withPackages (ps: [ ps.numpy ])"),
            "python3.withPackages"
        );
    }

    #[test]
    fn canonical_path_strips_trailing_semicolon() {
        assert_eq!(canonical_path("pkgs.htop;"), "htop");
    }

    #[test]
    fn canonical_path_works_without_pkgs_prefix() {
        assert_eq!(canonical_path("htop"), "htop");
        assert_eq!(canonical_path("custom.package"), "custom.package");
    }

    #[test]
    fn first_segment_extracts_leading_path() {
        assert_eq!(first_segment("python3.withPackages"), "python3");
        assert_eq!(first_segment("neovim"), "neovim");
        assert_eq!(first_segment("a.b.c.d"), "a");
    }

    #[test]
    fn item_matches_exact_canonical_path() {
        assert!(item_matches("pkgs.neovim", "neovim"));
        assert!(!item_matches("pkgs.neovim", "vim"));
    }

    #[test]
    fn item_matches_by_first_segment() {
        assert!(item_matches("pkgs.python3.withPackages (ps: [ ps.numpy ])", "python3"));
        assert!(item_matches("pkgs.python3", "python3"));
    }

    #[test]
    fn item_matches_complex_expressions() {
        assert!(item_matches(
            "pkgs.python3.withPackages (ps: [ ps.numpy ps.pandas ])",
            "python3.withPackages"
        ));
    }

    #[test]
    fn parse_items_single_item() {
        let text = "    pkgs.neovim";
        let items = parse_items(text);
        assert_eq!(items.len(), 1);
        assert_eq!(&text[items[0].start..items[0].end], "pkgs.neovim");
    }

    #[test]
    fn parse_items_multiple_items() {
        let text = "    pkgs.neovim\n    htop\n    lazygit";
        let items = parse_items(text);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn parse_items_with_complex_expressions() {
        let text = r#"    pkgs.neovim
    pkgs.python3.withPackages (ps: [
      ps.numpy
      ps.pandas
    ])
    htop"#;
        let items = parse_items(text);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn parse_items_respects_comments() {
        let text = "    pkgs.neovim # this is a comment\n    htop";
        let items = parse_items(text);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn find_bracket_range_simple() {
        let content = r#"
environment.systemPackages = with pkgs; [
    pkgs.neovim
    htop
];
"#;
        let (open, close) = find_bracket_range(content).unwrap();
        assert!(open < close);
        assert_eq!(&content[open..open + 1], "[");
        assert_eq!(&content[close..=close], "]");
        assert!(content[open + 1..close].contains("neovim"));
    }

    #[test]
    fn find_bracket_range_with_nested_brackets() {
        let content = r#"
environment.systemPackages = with pkgs; [
    pkgs.python3.withPackages (ps: [
        ps.numpy
    ])
];
"#;
        let (open, close) = find_bracket_range(content).unwrap();
        assert!(open < close);
        assert_eq!(&content[open..open + 1], "[");
        assert_eq!(&content[close..=close], "]");
    }

    #[test]
    fn flatten_for_display_removes_extra_whitespace() {
        assert_eq!(
            flatten_for_display("pkgs.python3.withPackages  (ps:  [ ])"),
            "pkgs.python3.withPackages (ps: [ ])"
        );
    }

    #[test]
    fn find_bracket_range_error_missing_target() {
        let content = "some random config";
        assert!(find_bracket_range(content).is_err());
    }

    #[test]
    fn find_bracket_range_error_unbalanced_brackets() {
        let content = r#"
environment.systemPackages = with pkgs; [
    pkgs.neovim
    [ nested [ brackets
"#;
        assert!(find_bracket_range(content).is_err());
    }
}
