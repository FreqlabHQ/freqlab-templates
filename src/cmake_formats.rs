//! Where a CMake plugin project lists its formats, and rewriting that list in place.
//! Covers three styles:
//!   1. one line:        `FORMATS VST3 AU Standalone`
//!   2. a block:         `FORMATS` with one format per line below it
//!   3. a variable:      `set(FORMATS Standalone AU VST3 AUv3)` (Pamplejuce), read
//!      by the plugin call as `FORMATS "${FORMATS}"`, a line that's left alone
//! Anything else (formats built from conditions, set in another file, listed in
//! more than one place, or a list broken up by blank or comment lines) isn't
//! edited: the project's own CMake decides.

/// Format names JUCE and iPlug2 accept in a FORMATS list
const NAMES: &[&str] = &["AAX", "AU", "AUv3", "AUV3", "VST3", "VST", "CLAP", "Standalone", "APP", "LV2"];

fn is_name(word: &str) -> bool {
    NAMES.contains(&word)
}

#[derive(Debug, Clone, PartialEq)]
enum Style {
    Line,
    Block { last: usize },
    Set,
}

/// The one place a CMakeLists.txt lists its formats
#[derive(Debug, Clone, PartialEq)]
pub struct FormatsSpot {
    /// Index of its first line
    pub line: usize,
    /// That line as written, for showing where the edit goes
    pub text: String,
    /// The formats it lists, as written
    pub names: Vec<String>,
    style: Style,
}

fn code(line: &str) -> &str {
    line.split('#').next().unwrap_or("")
}

/// A line that's only a format name (the list's last may close the call)
fn bare_name(line: &str) -> Option<&str> {
    let word = code(line).trim();
    let word = word.strip_suffix(')').unwrap_or(word).trim_end();
    is_name(word).then_some(word)
}

/// Whether the next line with code on it, from `from`, is a bare format name:
/// then the list goes on past where it seems to end, so it isn't edited
fn continues(lines: &[&str], from: usize) -> bool {
    lines[from.min(lines.len())..]
        .iter()
        .find(|l| !code(l).trim().is_empty())
        .is_some_and(|l| bare_name(l).is_some())
}

/// The words after `FORMATS` on a line, if they're all format names (a closing
/// `)` of the call may follow the last one)
fn names_after(rest: &str) -> Option<Vec<String>> {
    let rest = rest.trim();
    let rest = rest.strip_suffix(')').unwrap_or(rest).trim_end();
    let words: Vec<&str> = rest.split_whitespace().collect();
    (!words.is_empty() && words.iter().all(|w| is_name(w))).then(|| words.iter().map(|w| w.to_string()).collect())
}

/// Where a bracket comment opens in `text` (`#[[`, `#[=[`, `#[==[` ...): its
/// index, the opener's length and how many `=` it has
fn bracket_open(text: &str) -> Option<(usize, usize, usize)> {
    let mut from = 0;
    while let Some(at) = text[from..].find("#[").map(|i| from + i) {
        let equals = text[at + 2..].chars().take_while(|c| *c == '=').count();
        if text[at + 2 + equals..].starts_with('[') {
            return Some((at, 3 + equals, equals));
        }
        from = at + 2;
    }
    None
}

/// The file's lines with `#[[ ... ]]` bracket comments taken out (code before an
/// opening or after a closing one on the same line stays; a comment ends only at
/// the `]]`, `]=]` ... matching its opener), so a commented-out list or call is
/// never taken for a real one
pub fn without_bracket_comments(cmake: &str) -> Vec<String> {
    // The closing marker of the comment we're in, if any
    let mut closing: Option<String> = None;
    cmake
        .lines()
        .map(|line| {
            let mut kept = String::new();
            let mut rest = line;
            loop {
                if let Some(close) = &closing {
                    match rest.find(close.as_str()) {
                        Some(i) => {
                            rest = &rest[i + close.len()..];
                            closing = None;
                        }
                        None => break,
                    }
                } else {
                    match bracket_open(rest) {
                        Some((at, len, equals)) => {
                            kept.push_str(&rest[..at]);
                            closing = Some(format!("]{}]", "=".repeat(equals)));
                            rest = &rest[at + len..];
                        }
                        None => {
                            kept.push_str(rest);
                            break;
                        }
                    }
                }
            }
            kept
        })
        .collect()
}

/// The formats list, when the file has exactly one in a style that can be edited
pub fn find(cmake: &str) -> Option<FormatsSpot> {
    let code_lines = without_bracket_comments(cmake);
    let lines: Vec<&str> = code_lines.iter().map(String::as_str).collect();
    // A line touched by a bracket comment is never edited: the rewrite works on
    // the line as written
    let original: Vec<&str> = cmake.lines().collect();
    let plain = |k: usize| lines[k] == original[k];
    let mut found = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = code(lines[i]).trim();
        if line == "FORMATS" {
            let mut names = Vec::new();
            let mut j = i + 1;
            while let Some(word) = lines.get(j).and_then(|l| bare_name(l)) {
                // A comment on a format's line would be lost when the list is rewritten
                if lines[j].contains('#') {
                    return None;
                }
                names.push(word.to_string());
                j += 1;
            }
            if !names.is_empty() {
                if continues(&lines, j) || !(i..j).all(plain) {
                    return None;
                }
                found.push(FormatsSpot { line: i, text: lines[i].trim().to_string(), names, style: Style::Block { last: j - 1 } });
                i = j;
                continue;
            }
        } else if let Some(rest) = line.strip_prefix("FORMATS ").or_else(|| line.strip_prefix("FORMATS\t")) {
            if let Some(names) = names_after(rest) {
                if !plain(i) || (!rest.trim_end().ends_with(')') && continues(&lines, i + 1)) {
                    return None;
                }
                found.push(FormatsSpot { line: i, text: lines[i].trim().to_string(), names, style: Style::Line });
            }
        } else if let Some(inner) = line.strip_prefix("set(").map(str::trim_start) {
            if let Some(rest) = inner.strip_prefix("FORMATS ").or_else(|| inner.strip_prefix("FORMATS\t")) {
                if rest.trim_end().ends_with(')') {
                    if let Some(names) = names_after(rest) {
                        if !plain(i) {
                            return None;
                        }
                        found.push(FormatsSpot { line: i, text: lines[i].trim().to_string(), names, style: Style::Set });
                    }
                }
            }
        }
        i += 1;
    }
    // Listed in more than one place (per platform, say): not a single list to edit
    (found.len() == 1).then(|| found.remove(0))
}

/// Whether two lists name the same formats (order and case aside)
pub fn same_names(a: &[String], b: &[String]) -> bool {
    let norm = |v: &[String]| {
        let mut v: Vec<String> = v.iter().map(|s| s.to_ascii_lowercase()).collect();
        v.sort();
        v.dedup();
        v
    };
    norm(a) == norm(b)
}

/// `cmake` with its formats list replaced by `names`, keeping indentation, any
/// closing `)`, a comment after the list, and each line's ending. None when
/// there's no single list to edit or no names to write.
pub fn rewrite(cmake: &str, names: &[String]) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let spot = find(cmake)?;
    let pieces: Vec<&str> = cmake.split_inclusive('\n').collect();
    let body = |piece: &str| piece.trim_end_matches('\n').trim_end_matches('\r').len();
    let first = &pieces[spot.line][..body(pieces[spot.line])];
    let indent = &first[..first.len() - first.trim_start().len()];
    // A comment after the list, with the spacing before it
    let comment = if first.contains('#') { &first[code(first).trim_end().len()..] } else { "" };
    let list = names.join(" ");
    let last = match spot.style {
        Style::Block { last } => last,
        _ => spot.line,
    };
    let last_line = &pieces[last][..body(pieces[last])];
    let replacement = match spot.style {
        Style::Set => format!("{}set(FORMATS {}){}", indent, list, comment),
        _ => {
            let closes = code(last_line).trim_end().ends_with(')');
            format!("{}FORMATS {}{}{}", indent, list, if closes { ")" } else { "" }, comment)
        }
    };
    let ending = &pieces[last][body(pieces[last])..];
    let mut out = String::with_capacity(cmake.len());
    out.extend(pieces[..spot.line].iter().copied());
    out.push_str(&replacement);
    out.push_str(ending);
    out.extend(pieces[last + 1..].iter().copied());
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_one_line_list_is_found_and_rewritten() {
        let cmake = "juce_add_plugin(P\n    PLUGIN_CODE P001\n    FORMATS Standalone VST3 AU\n    PRODUCT_NAME \"P\")\n";
        let spot = find(cmake).unwrap();
        assert_eq!(spot.names, names(&["Standalone", "VST3", "AU"]));
        assert_eq!(spot.line, 2);
        let out = rewrite(cmake, &names(&["VST3", "AU"])).unwrap();
        assert_eq!(out, "juce_add_plugin(P\n    PLUGIN_CODE P001\n    FORMATS VST3 AU\n    PRODUCT_NAME \"P\")\n");
    }

    #[test]
    fn pamplejuce_edits_its_set_line_never_the_variable_reference() {
        let cmake = "set(FORMATS Standalone AU VST3 AUv3)\n\njuce_add_plugin(\"${PROJECT_NAME}\"\n    FORMATS \"${FORMATS}\"\n)\n";
        let spot = find(cmake).unwrap();
        assert_eq!(spot.text, "set(FORMATS Standalone AU VST3 AUv3)");
        let out = rewrite(cmake, &names(&["VST3", "AU"])).unwrap();
        assert!(out.starts_with("set(FORMATS VST3 AU)\n"), "{out}");
        assert!(out.contains("    FORMATS \"${FORMATS}\"\n"), "{out}");
    }

    #[test]
    fn a_block_list_becomes_one_line() {
        let cmake = "iplug_add_plugin(P\n  FORMATS\n    VST3\n    AU\n    APP\n  UI IGRAPHICS)\r\n";
        let cmake = cmake.replace('\n', "\r\n").replace("\r\r\n", "\r\n");
        let spot = find(&cmake).unwrap();
        assert_eq!(spot.names, names(&["VST3", "AU", "APP"]));
        let out = rewrite(&cmake, &names(&["VST3", "CLAP"])).unwrap();
        assert_eq!(out, "iplug_add_plugin(P\r\n  FORMATS VST3 CLAP\r\n  UI IGRAPHICS)\r\n");
    }

    #[test]
    fn a_list_closing_the_call_keeps_its_paren() {
        let out = rewrite("add(P\n  FORMATS VST3 AU)\n", &names(&["VST3"])).unwrap();
        assert_eq!(out, "add(P\n  FORMATS VST3)\n");
    }

    #[test]
    fn lists_that_cant_be_edited_safely_are_left_alone() {
        // Only a variable reference
        assert!(find("juce_add_plugin(P\n    FORMATS \"${FORMATS}\")\n").is_none());
        // Mixed with a variable
        assert!(find("    FORMATS VST3 ${EXTRA_FORMATS}\n").is_none());
        // Per platform: two lists
        assert!(find("if(APPLE)\n  set(FORMATS AU VST3)\nelse()\n  set(FORMATS VST3)\nendif()\n").is_none());
        // Commented out
        assert!(find("# FORMATS VST3 AU\n").is_none());
        // Nothing to write
        assert!(rewrite("    FORMATS VST3\n", &[]).is_none());
    }

    #[test]
    fn a_comment_after_the_list_and_mixed_line_endings_are_kept() {
        let cmake = "a\r\n    FORMATS AU VST3 Standalone  # Other valid formats are: AAX\nb\n";
        let out = rewrite(cmake, &names(&["VST3"])).unwrap();
        assert_eq!(out, "a\r\n    FORMATS VST3  # Other valid formats are: AAX\nb\n");
        let out = rewrite("set(FORMATS AU VST3) # mine\n", &names(&["AU"])).unwrap();
        assert_eq!(out, "set(FORMATS AU) # mine\n");
    }

    #[test]
    fn a_list_broken_up_by_blank_or_comment_lines_is_left_alone() {
        assert!(find("  FORMATS\n    VST3\n\n    AU\n").is_none());
        assert!(find("  FORMATS\n    VST3\n    # more\n    AU\n").is_none());
        assert!(find("  FORMATS VST3 AU\n    Standalone\n").is_none());
        // Comments on the formats' own lines
        assert!(find("  FORMATS\n    VST3 # win\n    AU\n").is_none());
        // A following line that isn't a format name doesn't count
        assert!(find("  FORMATS VST3 AU\n\n  PRODUCT_NAME \"P\"\n").is_some());
    }

    #[test]
    fn a_list_on_a_line_with_a_bracket_comment_is_never_edited() {
        assert!(find("#[[ old\nnote ]] FORMATS VST3 AU #[[ more\nstill comment ]]\n").is_none());
        assert!(find("  FORMATS\n  VST3\n  AU) #[[ trailing\ncomment ]]\n").is_none());
        assert!(find("#[[ a ]] FORMATS VST3 AU\n").is_none());
        assert!(find("FORMATS VST3 AU #[[ note ]]\n").is_none());
        assert!(find("set(FORMATS VST3) #[[ x ]]\n").is_none());
        // Not skipped either: the other platform's list isn't the only one
        assert!(find("if(APPLE)\n  set(FORMATS AU VST3) #[[mac]]\nelse()\n  set(FORMATS VST3)\nendif()\n").is_none());
    }

    #[test]
    fn a_bracket_comment_ends_only_at_its_own_marker() {
        let lines = without_bracket_comments("#[=[ a ]] still comment\nFORMATS VST3\n]=] after\n#[==[ x ]==] call(D)\n");
        assert_eq!(lines, vec!["", "", " after", " call(D)"]);
    }

    #[test]
    fn code_sharing_a_line_with_a_bracket_comment_still_counts() {
        let lines = without_bracket_comments("call(A) #[[ note\nhidden\nend ]] call(B)\n#[[ one ]] call(C)\n");
        assert_eq!(lines, vec!["call(A) ", "", " call(B)", " call(C)"]);
    }

    #[test]
    fn a_list_inside_a_bracket_comment_is_never_the_one() {
        let cmake = "#[[\n    FORMATS VST3 AU\n]]\nset(FORMATS Standalone VST3)\n";
        assert_eq!(find(cmake).unwrap().text, "set(FORMATS Standalone VST3)");
        let out = rewrite(cmake, &names(&["VST3"])).unwrap();
        assert_eq!(out, "#[[\n    FORMATS VST3 AU\n]]\nset(FORMATS VST3)\n");
    }

    #[test]
    fn formats_match_whatever_their_order_or_case() {
        assert!(same_names(&names(&["VST3", "AUv3"]), &names(&["AUV3", "VST3"])));
        assert!(!same_names(&names(&["VST3"]), &names(&["VST3", "AU"])));
    }
}
