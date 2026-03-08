#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 4 ]]; then
  echo "Usage: $0 <red_ai> <blue_ai> <artifact_json> <target_dir> [log_file]" >&2
  exit 1
fi

RED_AI="$1"
BLUE_AI="$2"
ARTIFACT_JSON="$3"
TARGET_DIR="$4"
LOG_FILE="${5:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GAME_DIR="$REPO_ROOT/game"

mkdir -p "$(dirname "$ARTIFACT_JSON")"
mkdir -p "$TARGET_DIR"

pushd "$GAME_DIR" >/dev/null

echo "Running: red=$RED_AI blue=$BLUE_AI target_dir=$TARGET_DIR"
if [[ -n "$LOG_FILE" ]]; then
  CARGO_TARGET_DIR="$TARGET_DIR" cargo run -- --headless --red "$RED_AI" --blue "$BLUE_AI" | tee "$LOG_FILE"
else
  CARGO_TARGET_DIR="$TARGET_DIR" cargo run -- --headless --red "$RED_AI" --blue "$BLUE_AI"
fi

cp game_log.json "$ARTIFACT_JSON"
echo "Saved artifact: $ARTIFACT_JSON"

popd >/dev/null
