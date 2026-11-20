#!/usr/bin/env bash
# build-starters.sh <framework> <work_dir>
#
# Writes every Starter of one framework as a project (as the apps write a new one) and builds each in
# turn. Each CMake dependency is downloaded once and reused by the later Starters
# (FETCHCONTENT_SOURCE_DIR_<NAME>), and nih-plug Starters share one Cargo target folder.

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <framework> <work_dir>" >&2
  exit 2
fi
FRAMEWORK="$1"
WORK="$2"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$WORK"
WORK="$(cd "$WORK" && pwd)"
# (Git Bash on Windows: paths CMake and Cargo read must be Windows ones, C:/..., not /c/...)
if command -v cygpath >/dev/null 2>&1; then WORK="$(cygpath -m "$WORK")"; fi

cd "$REPO"
cargo build -q --example render
RENDER="$REPO/target/debug/examples/render"
STARTERS=$("$RENDER" --starters "$FRAMEWORK")
if [[ -z "$STARTERS" ]]; then
  echo "No Starters for '$FRAMEWORK'" >&2
  exit 1
fi

shared_sources=()
failed=()
for starter in $STARTERS; do
  dir="$WORK/$starter"
  rm -rf "$dir"
  "$RENDER" "$FRAMEWORK" "$starter" "$dir"
  echo "::group::$FRAMEWORK $starter"
  if [[ -f "$dir/Cargo.toml" ]]; then
    if (cd "$dir" && CARGO_TARGET_DIR="$WORK/cargo-target" cargo build --release); then ok=1; else ok=0; fi
  else
    if (cd "$dir" && cmake -B build -S . -DCMAKE_BUILD_TYPE=Release ${shared_sources[@]+"${shared_sources[@]}"} && cmake --build build --config Release --parallel); then ok=1; else ok=0; fi
    # (this build's downloads, for the rest of this framework's Starters)
    if [[ $ok -eq 1 && -d "$dir/build/_deps" ]]; then
      for src in "$dir"/build/_deps/*-src; do
        [[ -d "$src" ]] || continue
        # (CMake's variable is the dependency's name upper-cased, hyphens kept)
        name="$(basename "$src" -src | tr '[:lower:]' '[:upper:]')"
        case " ${shared_sources[*]-} " in *" -DFETCHCONTENT_SOURCE_DIR_${name}="*) continue ;; esac
        shared_sources+=("-DFETCHCONTENT_SOURCE_DIR_${name}=$src")
      done
    fi
  fi
  echo "::endgroup::"
  if [[ $ok -eq 1 ]]; then echo "Built $FRAMEWORK $starter"; else echo "FAILED: $FRAMEWORK $starter"; failed+=("$starter"); fi
done

if [[ ${#failed[@]} -gt 0 ]]; then
  echo "Starters that didn't build ($FRAMEWORK): ${failed[*]}" >&2
  exit 1
fi
echo "All $FRAMEWORK Starters built."
