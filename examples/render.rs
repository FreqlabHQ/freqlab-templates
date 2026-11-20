//! Writes one Starter as a project with fixed example values, the way the apps write a new one, so
//! it can be built: `cargo run --example render -- <framework> <kind>-<ui> <out_dir>`.
//! The apps fill the same keys from the new project's name, vendor and fresh random ids.
//! `render --frameworks` prints the frameworks as a JSON array (CI's build matrix), and
//! `render --starters <framework>` prints that framework's Starters, one a line.

use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--frameworks") => {
            let names: Vec<String> = freqlab_templates::frameworks().iter().map(|f| format!("{f:?}")).collect();
            println!("[{}]", names.join(","));
            return;
        }
        Some("--starters") => {
            for starter in freqlab_templates::starters(args.get(1).map_or("", String::as_str)) {
                println!("{starter}");
            }
            return;
        }
        _ => {}
    }
    let [framework, variant, out] = args.as_slice() else {
        eprintln!("Usage: render <framework> <kind>-<ui> <out_dir>");
        std::process::exit(2);
    };
    let (kind, ui) = variant.split_once('-').unwrap_or_else(|| panic!("'{variant}' isn't <kind>-<ui>"));
    let files = freqlab_templates::starter(framework, kind, ui).unwrap_or_else(|| panic!("no {framework} Starter '{variant}'"));

    let values: HashMap<String, String> = [
        ("snake_name", "ci_plug"),
        ("pascal_name", "CiPlug"),
        ("struct_name", "CiPlug"),
        ("class_name", "CiPlugPlugin"),
        ("description", "A Starter built by CI"),
        ("vst3_id", "Qw3rTy7uIoP1aS2d"),
        ("clap_id", "com.freqlabci.ci_plug.k2j3h4"),
        ("plugin_code", "Kq7z"),
        ("vendor_name", "Freqlab CI"),
        ("vendor_id", "freqlabci"),
        ("mfr_id", "FQCI"),
        ("bundle_vendor", "freqlabci"),
        ("vendor_url", "https://example.com"),
        ("vendor_email", "ci@example.com"),
        ("plug_width", "400"),
        ("plug_height", "300"),
        ("year", "2026"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    let out = PathBuf::from(out);
    std::fs::create_dir_all(out.join("src")).unwrap();
    for file in files {
        let path = out.join(freqlab_templates::output_path(&file.filename, &values["pascal_name"]));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, freqlab_templates::apply_placeholders(file.content, &values)).unwrap();
    }
    // The formats a new project gets by default in the apps (VST3 and CLAP; JUCE's CLAP comes from
    // clap-juce-extensions, not its FORMATS list), written into FORMATS as the apps do
    let cmake = out.join("CMakeLists.txt");
    if let Ok(text) = std::fs::read_to_string(&cmake) {
        let names: Vec<String> = if framework.starts_with("juce") { vec!["VST3".into()] } else { vec!["VST3".into(), "CLAP".into()] };
        let rewritten = freqlab_templates::cmake_formats::rewrite(&text, &names).expect("one FORMATS list to edit");
        std::fs::write(&cmake, rewritten).unwrap();
    }
    println!("Wrote {framework}/{variant} to {}", out.display());
}
