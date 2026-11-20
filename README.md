# Freqlab Starter templates

The Starter plugin templates that [Freqlab Studio](https://creator.freqlab.co/studio) and [Freqlab Express](https://creator.freqlab.co/express) create new projects from, and the small Rust crate that turns one into a project. Both apps use this crate, so they make exactly the same projects.

## What's here

- `templates/<framework>/<kind>-<ui>/`: one folder per Starter (JUCE 8, JUCE 9, iPlug2 and NIH-plug; effect or instrument; each UI). A file's trailing `.template` is dropped when it's written, and `{{placeholders}}` are filled with the new project's name, vendor and ids.
- `src/lib.rs`: the crate. It compiles every template in, and gives the apps the Starter list, a Starter's files, placeholder filling and where each file goes in the new project.
- `examples/render.rs`: writes one Starter as a project with example values, for building it by hand or in CI.
- `scripts/build-starters.sh <framework> <dir>`: writes and builds every Starter of a framework, reusing the framework's download between them. CI runs it only when started by hand (Actions, CI, Run workflow); pushes run just the crate's tests.
- `scripts/bump-licensing-sdk.sh`: bumps the freqlab-licensing SDK tag pinned in every Starter.

## Try a Starter

```bash
cargo run --example render -- juce-9 effect-webview /tmp/my-plug
cd /tmp/my-plug && cmake -B build -S . -DCMAKE_BUILD_TYPE=Release && cmake --build build --config Release
```

NIH-plug Starters build with `cargo build --release`.

## How the apps use it

Each app depends on a tagged release in `src-tauri/Cargo.toml`:

```toml
freqlab-templates = { git = "https://github.com/FreqlabHQ/freqlab-templates", tag = "v0.1.0" }
```

The templates are compiled into the app: users never download anything, and the app works offline. To change a template, change it here, run the Starter builds for bigger changes (Actions, CI, Run workflow: macOS and Windows), tag a release, then bump the tag in both apps.

To work on templates and an app together, point the app at your checkout without editing its `Cargo.toml`, in the app's `src-tauri/.cargo/config.toml` (don't commit it):

```toml
[patch."https://github.com/FreqlabHQ/freqlab-templates"]
freqlab-templates = { path = "../../freqlab-templates" }
```

## License

The templates are under the MIT No Attribution license (MIT-0): projects made from them don't need to keep any notice. See [LICENSE](LICENSE).
