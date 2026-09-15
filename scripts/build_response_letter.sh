#!/usr/bin/env bash
# Build the concise editorial response for COMSI-2026-04-0112.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
source_file="$repo_root/docs/RESPONSE-LETTER.md"
output_file="$repo_root/docs/COMSI-2026-04-0112_summary-of-changes.pdf"

command -v pandoc >/dev/null
command -v pdflatex >/dev/null

pandoc "$source_file" \
  --from=markdown \
  --pdf-engine=pdflatex \
  --variable=documentclass:article \
  --variable=classoption:11pt \
  --variable=geometry:margin=0.85in \
  --variable=colorlinks:false \
  --output="$output_file"

test -s "$output_file"
printf 'BUILD-RESPONSE-LETTER: OK — %s\n' "$output_file"
