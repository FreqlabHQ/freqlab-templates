//! Freqlab's Starter plugin templates and the code that turns one into a project, shared by
//! Freqlab Studio and Freqlab Express so both create exactly the same projects.
//!
//! Templates live in `templates/<framework>/<kind>-<ui>/`, compiled into the crate. A file's
//! trailing `.template` is dropped when it's written.
//!
//! Freqlab licensing is optional in a new project. In a template, a line holding
//! `freqlab:licensing-begin` starts licensing code and `freqlab:licensing-end` ends it, in whatever
//! comment the file uses; `freqlab:licensing-else` between them starts what's written instead when
//! licensing is off. The marker lines never reach a project. Files that exist only for licensing
//! (`freqlab_response`, `freqlab_dsp`, `LicenseUI.h`) are left out without it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

pub mod cmake_formats;

include!(concat!(env!("OUT_DIR"), "/files.rs"));

/// One file of a Starter: its path inside the Starter (no `.template`) and its text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFile {
    pub filename: String,
    pub content: String,
}

const BEGIN: &str = "freqlab:licensing-begin";
const ELSE: &str = "freqlab:licensing-else";
const END: &str = "freqlab:licensing-end";

/// A file that's there only for Freqlab licensing
fn licensing_only(filename: &str) -> bool {
    let name = filename.rsplit('/').next().unwrap_or(filename);
    name.starts_with("freqlab_response.") || name.starts_with("freqlab_dsp.") || name == "LicenseUI.h"
}

/// `text` with its licensing blocks kept or dropped (see the crate docs), or why its markers
/// don't pair up
pub fn with_licensing(text: &str, licensing: bool) -> Result<String, String> {
    #[derive(PartialEq)]
    enum In {
        Outside,
        Licensing,
        Instead,
    }
    let mut at = In::Outside;
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let line_no = index + 1;
        if line.contains(BEGIN) {
            if at != In::Outside {
                return Err(format!("line {line_no}: licensing-begin inside another block"));
            }
            at = In::Licensing;
        } else if line.contains(ELSE) {
            if at != In::Licensing {
                return Err(format!("line {line_no}: licensing-else outside a block"));
            }
            at = In::Instead;
        } else if line.contains(END) {
            if at == In::Outside {
                return Err(format!("line {line_no}: licensing-end with no block open"));
            }
            at = In::Outside;
        } else if at == In::Outside || (at == In::Licensing) == licensing {
            out.push_str(line);
        }
    }
    if at != In::Outside {
        return Err("a licensing block is never closed".to_string());
    }
    Ok(out)
}

/// The frameworks that have Starters, sorted
pub fn frameworks() -> Vec<&'static str> {
    let mut names: Vec<_> = FILES.iter().filter_map(|(key, _)| key.split('/').next()).collect();
    names.dedup();
    names
}

/// A framework's Starters as `<kind>-<ui>` (e.g. `effect-webview`), sorted
pub fn starters(framework: &str) -> Vec<&'static str> {
    let mut names: Vec<_> = FILES
        .iter()
        .filter_map(|(key, _)| {
            let mut parts = key.splitn(3, '/');
            (parts.next() == Some(framework)).then(|| parts.next()).flatten()
        })
        .collect();
    names.dedup();
    names
}

/// Every file of the `<kind>-<ui>` Starter for `framework`, with or without Freqlab licensing, or
/// `None` if there isn't one
pub fn starter(framework: &str, kind: &str, ui: &str, licensing: bool) -> Option<Vec<TemplateFile>> {
    let prefix = format!("{framework}/{kind}-{ui}/");
    let files: Vec<_> = FILES
        .iter()
        .filter_map(|(key, content)| {
            let rel = key.strip_prefix(&prefix)?;
            let filename = rel.strip_suffix(".template").unwrap_or(rel).to_string();
            (licensing || !licensing_only(&filename)).then(|| TemplateFile {
                content: with_licensing(content, licensing).unwrap_or_else(|why| panic!("{key}: {why}")),
                filename,
            })
        })
        .collect();
    (!files.is_empty()).then_some(files)
}

/// One Starter file's text with licensing (placeholders still in): `name` is its path inside the
/// Starter, with or without `.template`
pub fn starter_file(framework: &str, variant: &str, name: &str) -> Option<String> {
    let key = format!("{framework}/{variant}/{}", name.strip_suffix(".template").unwrap_or(name));
    let (_, content) = FILES.iter().find(|(k, _)| k.strip_suffix(".template").unwrap_or(k) == key)?;
    Some(with_licensing(content, true).unwrap_or_else(|why| panic!("{key}: {why}")))
}

/// Fills placeholders (`{{key}}` or `{key}`) in one pass: a value is never expanded again, and
/// unknown keys stay as written
pub fn apply_placeholders(content: &str, placeholders: &HashMap<String, String>) -> String {
    static PLACEHOLDER: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\{\{(\w+)\}\}|\{(\w+)\}").expect("valid pattern"));
    PLACEHOLDER
        .replace_all(content, |caps: &regex::Captures| {
            let key = caps.get(1).or_else(|| caps.get(2)).map_or("", |m| m.as_str());
            placeholders.get(key).cloned().unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

/// Where a Starter file goes in the new project, relative to its folder. `pascal_name` prefixes
/// iPlug2's plists and XIBs.
pub fn output_path(filename: &str, pascal_name: &str) -> PathBuf {
    if filename.contains('/') || filename.contains('\\') {
        // (a file with its own folders keeps them, e.g. resources/web/ui.html)
        PathBuf::from(filename)
    } else if matches!(filename, "Cargo.toml" | "CMakeLists.txt" | "config.h" | "resource.h") {
        // (build files, and iPlug2's config.h and resource.h, at the root)
        PathBuf::from(filename)
    } else if filename.starts_with("main.rc_mac") {
        PathBuf::from("resources").join(filename)
    } else if filename.ends_with(".plist") || filename.ends_with(".xib") {
        PathBuf::from("resources").join(format!("{pascal_name}-{filename}"))
    } else if filename.ends_with(".rs") || filename.ends_with(".cpp") || filename.ends_with(".h") {
        PathBuf::from("src").join(filename)
    } else {
        PathBuf::from(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_framework_has_its_six_starters() {
        assert_eq!(frameworks(), ["iplug2", "juce-8", "juce-9", "nih-plug"]);
        for framework in frameworks() {
            let names = starters(framework);
            assert_eq!(names.len(), 6, "{framework}: {names:?}");
            for name in names {
                let (kind, ui) = name.split_once('-').unwrap();
                for licensing in [true, false] {
                    let files = starter(framework, kind, ui, licensing).unwrap();
                    assert!(files.iter().all(|f| !f.filename.ends_with(".template") && !f.content.is_empty()), "{framework}/{name}");
                    assert!(files.iter().any(|f| f.filename == "Cargo.toml" || f.filename == "CMakeLists.txt"), "{framework}/{name}");
                    assert!(files.iter().all(|f| ![BEGIN, ELSE, END].iter().any(|m| f.content.contains(m))), "{framework}/{name}: a marker left");
                }
            }
        }
        assert!(starter("juce-9", "effect", "nope", true).is_none());
        assert!(starters("nope").is_empty());
    }

    /// Every word that only licensing code uses: none may be left in a project made without it
    const LICENSING_WORDS: &[&str] = &[
        "freqlab_licensing", "freqlab-licensing", "licensing::", "freqlab_should_run_dsp", "freqlab_dsp",
        "freqlab_response", "@freqlab:", "LicenseUI", "licenseState", "license_state", "LicenseStatus",
        "LicenseBanner", "LicenseModal", "refreshAsync", "FREQLAB_LICENSING",
    ];

    #[test]
    fn a_starter_made_without_licensing_has_none_of_it() {
        for framework in frameworks() {
            for name in starters(framework) {
                let (kind, ui) = name.split_once('-').unwrap();
                let off = starter(framework, kind, ui, false).unwrap();
                let on = starter(framework, kind, ui, true).unwrap();
                assert!(off.iter().all(|f| !licensing_only(&f.filename)), "{framework}/{name}");
                assert!(on.iter().any(|f| licensing_only(&f.filename)), "{framework}/{name} has its licensing files");
                for file in &off {
                    for (n, line) in file.content.lines().enumerate() {
                        let left: Vec<&&str> = LICENSING_WORDS.iter().filter(|w| line.contains(**w)).collect();
                        assert!(left.is_empty(), "{framework}/{name}/{}:{}: {left:?} in `{}`", file.filename, n + 1, line.trim());
                        // (a Cargo package's `license = ` is its own licence, not Freqlab's)
                        let lower = line.to_lowercase();
                        assert!(!lower.contains("licens") || lower.trim_start().starts_with("license ="),
                            "{framework}/{name}/{}:{}: `{}`", file.filename, n + 1, line.trim());
                    }
                }
            }
        }
    }

    #[test]
    fn licensing_blocks_are_kept_dropped_or_swapped() {
        let text = "a\n// freqlab:licensing-begin\nlicensed\n// freqlab:licensing-else\nplain\n// freqlab:licensing-end\nb\n# freqlab:licensing-begin\nonly\n# freqlab:licensing-end\nc";
        assert_eq!(with_licensing(text, true).unwrap(), "a\nlicensed\nb\nonly\nc");
        assert_eq!(with_licensing(text, false).unwrap(), "a\nplain\nb\nc");
        assert!(with_licensing("// freqlab:licensing-begin\nx\n", true).is_err());
        assert!(with_licensing("x\n// freqlab:licensing-end\n", true).is_err());
        assert!(with_licensing("// freqlab:licensing-else\n", true).is_err());
        assert!(with_licensing("// freqlab:licensing-begin\n// freqlab:licensing-begin\n", true).is_err());
        assert_eq!(with_licensing("no markers\r\nat all", false).unwrap(), "no markers\r\nat all");
    }

    #[test]
    fn a_starter_file_is_found_with_or_without_its_suffix() {
        let a = starter_file("iplug2", "effect-igraphics", "CMakeLists.txt.template").unwrap();
        assert_eq!(Some(a), starter_file("iplug2", "effect-igraphics", "CMakeLists.txt"));
        assert!(starter_file("iplug2", "effect-igraphics", "nope.txt").is_none());
    }

    #[test]
    fn a_placeholder_value_is_never_expanded_again() {
        for _ in 0..40 {
            let p: HashMap<String, String> =
                [("description", "About {{pascal_name}} and {x}"), ("pascal_name", "Gain")].map(|(k, v)| (k.to_string(), v.to_string())).into();
            assert_eq!(
                apply_placeholders("{{pascal_name}}: {{description}} {unknown} {{other}} {pascal_name}", &p),
                "Gain: About {{pascal_name}} and {x} {unknown} {{other}} Gain"
            );
        }
    }

    #[test]
    fn files_go_where_the_apps_always_put_them() {
        let at = |f: &str| output_path(f, "Gain").to_string_lossy().replace('\\', "/");
        assert_eq!(at("resources/web/ui.html"), "resources/web/ui.html");
        assert_eq!(at("Cargo.toml"), "Cargo.toml");
        assert_eq!(at("CMakeLists.txt"), "CMakeLists.txt");
        assert_eq!(at("config.h"), "config.h");
        assert_eq!(at("resource.h"), "resource.h");
        assert_eq!(at("main.rc_mac"), "resources/main.rc_mac");
        assert_eq!(at("Info.plist"), "resources/Gain-Info.plist");
        assert_eq!(at("main.xib"), "resources/Gain-main.xib");
        assert_eq!(at("lib.rs"), "src/lib.rs");
        assert_eq!(at("PluginProcessor.cpp"), "src/PluginProcessor.cpp");
        assert_eq!(at("PluginProcessor.h"), "src/PluginProcessor.h");
        assert_eq!(at("README.md"), "README.md");
    }
}
