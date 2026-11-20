//! Freqlab's Starter plugin templates and the code that turns one into a project, shared by
//! Freqlab Studio and Freqlab Express so both create exactly the same projects.
//!
//! Templates live in `templates/<framework>/<kind>-<ui>/`, compiled into the crate. A file's
//! trailing `.template` is dropped when it's written.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

pub mod cmake_formats;

include!(concat!(env!("OUT_DIR"), "/files.rs"));

/// One file of a Starter: its path inside the Starter (no `.template`) and its text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFile {
    pub filename: String,
    pub content: &'static str,
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

/// Every file of the `<kind>-<ui>` Starter for `framework`, or `None` if there isn't one
pub fn starter(framework: &str, kind: &str, ui: &str) -> Option<Vec<TemplateFile>> {
    let prefix = format!("{framework}/{kind}-{ui}/");
    let files: Vec<_> = FILES
        .iter()
        .filter_map(|(key, content)| {
            let rel = key.strip_prefix(&prefix)?;
            Some(TemplateFile { filename: rel.strip_suffix(".template").unwrap_or(rel).to_string(), content })
        })
        .collect();
    (!files.is_empty()).then_some(files)
}

/// One Starter file's text as written in the template (placeholders and all): `name` is its
/// path inside the Starter, with or without `.template`
pub fn starter_file(framework: &str, variant: &str, name: &str) -> Option<&'static str> {
    let key = format!("{framework}/{variant}/{}", name.strip_suffix(".template").unwrap_or(name));
    FILES
        .iter()
        .find(|(k, _)| k.strip_suffix(".template").unwrap_or(k) == key)
        .map(|(_, content)| *content)
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
                let files = starter(framework, kind, ui).unwrap();
                assert!(files.iter().all(|f| !f.filename.ends_with(".template") && !f.content.is_empty()), "{framework}/{name}");
                assert!(files.iter().any(|f| f.filename == "Cargo.toml" || f.filename == "CMakeLists.txt"), "{framework}/{name}");
            }
        }
        assert!(starter("juce-9", "effect", "nope").is_none());
        assert!(starters("nope").is_empty());
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
