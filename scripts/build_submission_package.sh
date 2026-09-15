#!/usr/bin/env bash
# Build the single LaTeX archive requested as the Main Manuscript upload.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
source_dir="$repo_root/paper1"
stage_dir="$repo_root/dist/main-latex"
archive="$repo_root/dist/COMSI-2026-04-0112_main-latex.zip"

command -v zip >/dev/null
command -v unzip >/dev/null

rm -rf "$stage_dir"
mkdir -p "$stage_dir"

files=(
  main.tex
  abstract.tex
  section1_introduction.tex
  section2_related_work.tex
  section3_type_system.tex
  section4_theorem.tex
  section5_benchmarks.tex
  section5_table1_construction.tex
  section5_tables_generated.tex
  section6_discussion.tex
  section7_conclusion.tex
  refs.bib
)

for file in "${files[@]}"; do
  test -s "$source_dir/$file"
  cp "$source_dir/$file" "$stage_dir/$file"
done

rm -f "$archive"
(
  cd "$stage_dir"
  zip -q "$archive" "${files[@]}"
)
unzip -t "$archive" >/dev/null

printf 'BUILD-SUBMISSION-PACKAGE: OK — %s\n' "$archive"
