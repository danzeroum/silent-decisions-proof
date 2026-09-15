#!/usr/bin/env bash
# Build a reviewer-facing tracked-changes PDF against the archived April source.
#
# Baseline provenance:
#   commit 61194cc is the final repository snapshot before the April submission;
#   its text matches the retained 10-page historical PDF on the central claims,
#   legal citations, equations, benchmark figures, and section structure.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
baseline="${TRACKED_BASELINE:-61194cc}"
work_dir="$(mktemp -d)"
output="$repo_root/dist/COMSI-2026-04-0112_tracked-changes.pdf"
trap 'rm -rf "$work_dir"' EXIT

for tool in git tar latexpand latexdiff pdflatex bibtex; do
  command -v "$tool" >/dev/null
done

git -C "$repo_root" cat-file -e "$baseline^{commit}"
mkdir -p "$work_dir/baseline" "$work_dir/current" "$repo_root/dist"
git -C "$repo_root" archive "$baseline" paper1 | tar -x -C "$work_dir/baseline"
cp -a "$repo_root/paper1/." "$work_dir/current/"

(
  cd "$work_dir/baseline/paper1"
  latexpand main.tex > "$work_dir/baseline-flat.tex"
)
(
  cd "$work_dir/current"
  latexpand main.tex > "$work_dir/current-flat.tex"
)

latexdiff \
  --type=CFONT \
  --append-textcmd=href \
  "$work_dir/baseline-flat.tex" \
  "$work_dir/current-flat.tex" \
  > "$work_dir/tracked-changes.tex"

cp "$repo_root/paper1/refs.bib" "$work_dir/refs.bib"
(
  cd "$work_dir"
  timeout 400 pdflatex -interaction=nonstopmode tracked-changes.tex >/dev/null
  timeout 300 bibtex tracked-changes >/dev/null
  timeout 400 pdflatex -interaction=nonstopmode tracked-changes.tex >/dev/null
  timeout 400 pdflatex -interaction=nonstopmode tracked-changes.tex >/dev/null
)

test -s "$work_dir/tracked-changes.pdf"
cp "$work_dir/tracked-changes.pdf" "$output"
printf 'BUILD-TRACKED-CHANGES: OK — %s (baseline %s)\n' "$output" "$baseline"
