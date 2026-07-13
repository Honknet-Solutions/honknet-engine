#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-/workspaces/space-station-15}"
SOURCE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ ! -d "$TARGET/.git" ]]; then
  echo "Target is not a git repository: $TARGET" >&2
  exit 1
fi

rsync -a --delete "$SOURCE/apps/" "$TARGET/apps/"
rsync -a --delete "$SOURCE/crates/" "$TARGET/crates/"
rsync -a --delete "$SOURCE/content/" "$TARGET/content/"
cp "$SOURCE/Cargo.toml" "$TARGET/Cargo.toml"
cp "$SOURCE/package.json" "$TARGET/package.json"
cp "$SOURCE/.gitignore" "$TARGET/.gitignore"
cp "$SOURCE/README.md" "$TARGET/README.md"

echo "Applied complete vertical-slice source to $TARGET"
