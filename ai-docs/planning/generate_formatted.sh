#!/usr/bin/env bash
set -euo pipefail

# Generate ai-docs/planning/original-chat-formatted.md from
# ai-docs/original-chat-raw.md by removing a leading/trailing
# fenced ```markdown block if present while preserving inner code blocks.

DST="$(cd "$(dirname "$0")" && pwd)/original-chat-formatted.md"

# Candidate source locations (ordered)
CANDIDATES=(
  "$(cd "$(dirname "$0")" && pwd)/original-chat-raw.md"
  "$(cd "$(dirname "$0")/.." && pwd)/original-chat-raw.md"
  "$(cd "$(dirname "$0")/.." && pwd)/original-chat.md"
)

SRC=""
for c in "${CANDIDATES[@]}"; do
  if [ -f "$c" ]; then
    SRC="$c"
    break
  fi
done

if [ -z "$SRC" ]; then
  echo "Source not found in candidates: ${CANDIDATES[*]}" >&2
  exit 1
fi

if [ ! -f "$SRC" ]; then
  echo "Source not found: $SRC" >&2
  exit 1
fi

# Use Perl to safely remove an initial ```...\n and a trailing ``` block if present.
perl -0777 -pe 's/\A```(?:\w+)?\n//s; s/\n```+\s*\z//s' "$SRC" > "$DST"

echo "Wrote $DST"
