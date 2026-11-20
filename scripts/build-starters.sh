#!/usr/bin/env bash
# build-starters.sh <framework> <work_dir> [both|on|off]
#
# Writes every Starter of one framework as a project (as the apps write a new one) and builds each in
# turn. Each CMake dependency is downloaded once and reused by the later Starters
# (FETCHCONTENT_SOURCE_DIR_<NAME>), and nih-plug Starters share one Cargo target folder. The third
# argument picks Freqlab licensing on, off, or both (the default: each Starter built twice).

set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: $0 <framework> <work_dir> [both|on|off]" >&2
  exit 2
fi
FRAMEWORK="$1"
WORK="$2"
case "${3:-both}" in
  both) MODES="on off" ;;
  on|off) MODES="$3" ;;
  *) echo "Licensing must be both, on or off (got '$3')" >&2; exit 2 ;;
esac
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
for mode in $MODES; do
for starter in $STARTERS; do
  name="$starter (licensing $mode)"
  dir="$WORK/$mode/$starter"
  rm -rf "$dir"
  if [[ "$mode" == "on" ]]; then "$RENDER" "$FRAMEWORK" "$starter" "$dir"; else "$RENDER" "$FRAMEWORK" "$starter" "$dir" --no-licensing; fi
  echo "::group::$FRAMEWORK $name"
  if [[ -f "$dir/Cargo.toml" ]]; then
    if (cd "$dir" && CARGO_TARGET_DIR="$WORK/cargo-target" cargo build --release); then ok=1; else ok=0; fi
  else
    if (cd "$dir" && cmake -B build -S . -DCMAKE_BUILD_TYPE=Release ${shared_sources[@]+"${shared_sources[@]}"} && cmake --build build --config Release --parallel); then ok=1; else ok=0; fi
    # (this build's downloads, for the rest of this framework's Starters)
    if [[ $ok -eq 1 && -d "$dir/build/_deps" ]]; then
      for src in "$dir"/build/_deps/*-src; do
        [[ -d "$src" ]] || continue
        # (CMake's variable is the dependency's name upper-cased, hyphens kept)
        dep="$(basename "$src" -src | tr '[:lower:]' '[:upper:]')"
        case " ${shared_sources[*]-} " in *" -DFETCHCONTENT_SOURCE_DIR_${dep}="*) continue ;; esac
        shared_sources+=("-DFETCHCONTENT_SOURCE_DIR_${dep}=$src")
      done
    fi
  fi
  echo "::endgroup::"
  if [[ $ok -eq 1 ]]; then echo "Built $FRAMEWORK $name"; else echo "FAILED: $FRAMEWORK $name"; failed+=("$name"); fi
done
done

if [[ ${#failed[@]} -gt 0 ]]; then
  printf "Starters that didn't build (%s):\n" "$FRAMEWORK" >&2
  printf "  %s\n" "${failed[@]}" >&2
  exit 1
fi
echo "All $FRAMEWORK Starters built."
