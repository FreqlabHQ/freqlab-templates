#!/usr/bin/env bash
# bump-licensing-sdk.sh <new-tag>
#
# Bump the freqlab-licensing SDK tag pinned in every Starter: the nih-plug Cargo.toml templates and
# the freqlab_licensing FetchContent block in the iPlug2 / JUCE CMakeLists templates. A pinned tag
# makes users' caches refetch when it changes (a moving branch goes stale silently).
#
# Only the freqlab_licensing block is touched in CMake: CLAP, JUCE, iPlug2 and VST3 SDK GIT_TAGs are
# left alone. Tag the SDK release in freqlab-licensing BEFORE running this, or new projects can't
# fetch it. The script doesn't commit.
#
# Usage:
#   ./scripts/bump-licensing-sdk.sh sdk-v0.2.0
#   ./scripts/bump-licensing-sdk.sh v0.2.0          # sdk- prefix added

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <new-tag>   e.g. $0 sdk-v0.2.0   or   $0 v0.2.0" >&2
  exit 1
fi
NEW_TAG="$1"
case "$NEW_TAG" in
  sdk-v*) ;;
  v*)     NEW_TAG="sdk-$NEW_TAG" ;;
  *)      echo "Refusing: tag must start with 'v' or 'sdk-v' (got '$NEW_TAG')" >&2; exit 1 ;;
esac
if [[ ! "$NEW_TAG" =~ ^sdk-v[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
  echo "Refusing: '$NEW_TAG' is not 'sdk-vMAJOR.MINOR.PATCH' form" >&2
  exit 1
fi
for tool in perl grep find; do
  command -v "$tool" >/dev/null 2>&1 || { echo "Refusing: $tool is required but not on PATH." >&2; exit 1; }
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)/templates"
export NEW_TAG
cargo_n=0
cmake_n=0

# Rust: freqlab-licensing = { git = "...", branch|tag = "..." } becomes tag = "$NEW_TAG"
while IFS= read -r f; do
  perl -i -pe '
    BEGIN { $t = $ENV{NEW_TAG} }
    s{(freqlab-licensing\s*=\s*\{\s*git\s*=\s*"[^"]+"\s*,\s*)(?:branch|tag)\s*=\s*"[^"]+"}
     {"${1}tag = \"$t\""}e;
  ' "$f"
  if grep -q "freqlab-licensing.*tag = \"$NEW_TAG\"" "$f"; then
    cargo_n=$((cargo_n + 1))
  else
    echo "  WARN: no freqlab-licensing pin found after edit in $f" >&2
  fi
done < <(find "$ROOT" -type f -name 'Cargo.toml.template' -exec grep -l 'freqlab-licensing\s*=\s*{' {} +)

# CMake: the 3-line block (freqlab_licensing, GIT_REPOSITORY, GIT_TAG): matching all three means
# no other GIT_TAG can be hit by accident
while IFS= read -r f; do
  perl -i -0pe '
    BEGIN { $t = $ENV{NEW_TAG} }
    s{(\n\s+freqlab_licensing\n\s+GIT_REPOSITORY\s+\S+\n\s+GIT_TAG\s+)\S+}
     {"${1}$t"}e;
  ' "$f"
  if perl -0ne 'exit(/\n\s+freqlab_licensing\n\s+GIT_REPOSITORY\s+\S+\n\s+GIT_TAG\s+\Q$ENV{NEW_TAG}\E\b/ ? 0 : 1)' "$f"; then
    cmake_n=$((cmake_n + 1))
  else
    echo "  WARN: freqlab_licensing block not at the expected tag after edit in $f" >&2
  fi
done < <(find "$ROOT" -type f -name 'CMakeLists.txt.template' -exec grep -lE '^\s+freqlab_licensing\s*$' {} +)

echo "Bumped freqlab-licensing to $NEW_TAG: $cargo_n Cargo.toml + $cmake_n CMakeLists.txt templates."
echo "Review with: git -C \"$(dirname "$ROOT")\" diff -- templates/"
echo "Only freqlab_licensing / freqlab-licensing lines should change."
