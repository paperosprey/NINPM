//! Parsing/rewriting of a Nix `environment.systemPackages = with pkgs; [ ... ]`
//! list. This is intentionally NOT a full Nix parser (that way lies madness --
//! see project name). It understands just enough Nix syntax to safely treat
//! each list entry as one unit, even when that entry spans multiple lines or
//! is itself a function application like:
//!
//!   pkgs.python3.withPackages (ps: [
//!     ps.numpy
//!   ])
//!
//! Known, documented limitations:
//!   - Nix's indented strings (`''...''`) are not specially handled; only
//!     double-quoted strings are. An indented string containing an unbalanced
//!     bracket character could confuse depth tracking. This is rare inside a
//!     package list and is a deliberate scope boundary for this tool.
//!   - The tool operates on text, not semantics: it cannot know whether an
//!     attribute path like `python3Packages.numpy` is "one package" from
//!     Nix's point of view. It matches on the text you gave it.

use anyhow::{bail, Result};

pub const TARGET: &str = "environment.systemPackages = with pkgs; [";

/// Byte range of one list item's raw source text, relative to the string it
/// was parsed from.
#[derive(Debug, Clone, Copy)]
pub struct ItemSpan {
    pub start: usize,
    pub end: usize,
}

/// Locates the systemPackages list and returns the absolute byte indices of
/// its opening `[` and matching closing `]`.
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

/// Splits the raw text between `[` and `]` into top-level items, skipping
/// over strings/comments so brackets inside them don't throw off depth
/// tracking, and merging a bare `(...)` continuation into the previous item
/// (the `withPackages (ps: [...])` pattern).
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

    // Merge pass: fold a token that starts with '(' into the previous item,
    // since in this context it's almost always a function argument
    // continuation, not a genuinely separate list entry.
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

/// The dotted attribute path of an item with the leading `pkgs.` dropped and
/// any trailing function-application / whitespace stripped, e.g.:
///   "pkgs.python3.withPackages (ps: [...])" -> "python3.withPackages"
///   "pkgs.vim"                              -> "vim"
///   "htop"                                  -> "htop"
pub fn canonical_path(raw_item: &str) -> &str {
    let trimmed = raw_item.trim();
    let after_prefix = trimmed.strip_prefix("pkgs.").unwrap_or(trimmed);
    let end = after_prefix
        .find(|c: char| c == '(' || c == '{' || c.is_whitespace())
        .unwrap_or(after_prefix.len());
    after_prefix[..end].trim_end_matches(';')
}

/// The first path segment, e.g. "python3.withPackages" -> "python3".
pub fn first_segment(canonical: &str) -> &str {
    let end = canonical.find('.').unwrap_or(canonical.len());
    &canonical[..end]
}

/// True if the user-supplied `target` (a bare name or a dotted path) refers
/// to this item, either by matching its full canonical path or just its
/// first segment (so both `explode python3Packages.numpy` and
/// `explode python3Packages` work as you'd expect).
pub fn item_matches(raw_item: &str, target: &str) -> bool {
    let canon = canonical_path(raw_item);
    canon == target || first_segment(canon) == target
}

pub fn flatten_for_display(raw_item: &str) -> String {
    raw_item.split_whitespace().collect::<Vec<_>>().join(" ")
}
