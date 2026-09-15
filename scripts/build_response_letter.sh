#!/usr/bin/env bash
# Build the final author-response letter PDF for COMSI-2026-04-0112 (R1).
# Source: docs/RESPONSE-LETTER-FINAL.md (the single consolidated letter).
# Engine: pdflatex when available (CI), otherwise tectonic (local R1 env).
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
source_file="$repo_root/docs/RESPONSE-LETTER-FINAL.md"
out_dir="$repo_root/release"
output_file="$out_dir/COMSI-2026-04-0112_response_letter_final.pdf"

command -v pandoc >/dev/null
if command -v pdflatex >/dev/null; then
  engine="pdflatex"
else
  command -v tectonic >/dev/null || { echo "no PDF engine" >&2; exit 1; }
  engine="tectonic"
fi
mkdir -p "$out_dir"

pandoc "$source_file" \
  --from=markdown \
  --pdf-engine="$engine" \
  --variable=documentclass:article \
  --variable=classoption:11pt \
  --variable=geometry:margin=0.85in \
  --variable=colorlinks:false -H scripts/letter-header.tex \
  --output="$output_file"

test -s "$output_file"
printf 'BUILD-RESPONSE-LETTER: OK — %s (engine %s)\n' "$output_file" "$engine"
