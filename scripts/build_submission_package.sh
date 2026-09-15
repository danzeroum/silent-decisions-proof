#!/usr/bin/env bash
# Build the single LaTeX source archive requested as the Main Manuscript
# upload (R1: includes the five figure PDFs and their versioned TikZ sources).
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
source_dir="$repo_root/paper1"
stage_dir="$repo_root/release/main-latex"
out_dir="$repo_root/release"
archive="$out_dir/COMSI-2026-04-0112_source.zip"

command -v zip >/dev/null
command -v unzip >/dev/null

rm -rf "$stage_dir"
mkdir -p "$stage_dir/figures" "$stage_dir/figure-src"

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

# Figures: compiled PDFs (needed to compile the archive) + versioned sources
for figpdf in "$source_dir"/figures/fig*.pdf; do
  test -s "$figpdf"
  cp "$figpdf" "$stage_dir/figures/$(basename "$figpdf")"
done
for figsrc in "$source_dir"/figure-src/fig*.tex; do
  test -s "$figsrc"
  cp "$figsrc" "$stage_dir/figure-src/$(basename "$figsrc")"
done

rm -f "$archive"
(
  cd "$stage_dir"
  zip -rq "$archive" "${files[@]}" figures figure-src
)
unzip -t "$archive" >/dev/null

printf 'BUILD-SUBMISSION-PACKAGE: OK — %s\n' "$archive"
