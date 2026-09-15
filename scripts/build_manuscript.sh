#!/usr/bin/env bash
# build_manuscript.sh — compile paper1 to a submission-ready PDF and enforce
# the typographic gates (COMSI-2026-04-0112 Round 3; R1 revision 2026-09-15).
#
# Until this round, NO round of this manuscript had ever been compiled: every
# check was structural (balanced environments, \ref/\label pairs, \cite keys).
# The first compile surfaced three tables overflowing IEEEtran's column
# measure by up to 99.7 pt — invisible to every structural check, obvious to
# any reader of the PDF. This script makes the compile a gate, not a hope.
#
# Engine: uses pdflatex when available (CI), otherwise tectonic (local R1
# environment). Tectonic runs bibtex and reruns automatically and writes the
# same TeX log files the gates below parse.
#
# Exits non-zero on: any LaTeX error, any undefined reference or citation,
# any overfull hbox, or a BibTeX warning.
#
# Usage:  bash scripts/build_manuscript.sh
# Output: paper1/main.pdf
set -uo pipefail

cd "$(dirname "$0")/../paper1" || exit 1

# ── Figures first (R1: figure sources are now versioned in figure-src/) ──
for figsrc in figure-src/fig*.tex; do
  figpdf="figures/$(basename "${figsrc%.tex}").pdf"
  if [ ! -s "$figpdf" ] || [ "$figsrc" -nt "$figpdf" ]; then
    if command -v tectonic >/dev/null; then
      tectonic --keep-logs "$figsrc" >/dev/null 2>&1 || {
        echo "BUILD-MANUSCRIPT: FAIL — figure $figsrc did not compile" >&2
        exit 1; }
      mv "figure-src/$(basename "${figsrc%.tex}").pdf" "$figpdf"
    elif command -v pdflatex >/dev/null; then
      (cd figure-src && pdflatex -interaction=nonstopmode "$(basename "$figsrc")" >/dev/null 2>&1) || {
        echo "BUILD-MANUSCRIPT: FAIL — figure $figsrc did not compile" >&2
        exit 1; }
      mv "figure-src/$(basename "${figsrc%.tex}").pdf" "$figpdf"
    else
      echo "BUILD-MANUSCRIPT: FAIL — no LaTeX engine for figures" >&2; exit 1
    fi
  fi
done

rm -f main.aux main.bbl main.blg main.log main.out main.pdf

if command -v pdflatex >/dev/null; then
  run() { timeout 400 pdflatex -interaction=nonstopmode main.tex >/dev/null 2>&1; }
  run
  timeout 300 bibtex main >/dev/null 2>&1
  run
  run
else
  command -v tectonic >/dev/null || { echo "no LaTeX engine" >&2; exit 1; }
  timeout 500 tectonic --keep-logs --keep-intermediates main.tex >/dev/null 2>&1
fi

fail=0
report() { printf '%-34s %s\n' "$1" "$2"; }

if [ ! -f main.pdf ]; then
    report "PDF produced" "NO — compile failed"; exit 1
fi
pages=$(grep -oE 'Output written on main\.pdf \([0-9]+ pages|Output written on main\.xdv \([0-9]+ pages' main.log | grep -oE '[0-9]+' | head -1)
report "PDF produced" "yes (${pages} pages)"

n=$(grep -c '^!' main.log || true);            report "LaTeX errors" "$n"; [ "$n" -eq 0 ] || fail=1
n=$(grep -cE 'Reference .* undefined|Citation .* undefined|There were undefined' main.log || true)
report "undefined refs/citations" "$n"; [ "$n" -eq 0 ] || fail=1
n=$(grep -c 'Overfull \\hbox' main.log || true); report "overfull hboxes" "$n"; [ "$n" -eq 0 ] || fail=1
n=$(grep -c '^\\bibitem' main.bbl || true);    report "bibliography entries" "$n"
if [ -s main.blg ]; then
  n=$(grep -i 'warning' main.blg | grep -vc 'warning\$' || true)
  report "BibTeX warnings" "$n"; [ "$n" -eq 0 ] || fail=1
fi
n=$(grep -c 'includegraphics' section*.tex 2>/dev/null || true)
report "figure includes" "$n"

if [ "$fail" -eq 0 ]; then
    echo "BUILD-MANUSCRIPT: OK — main.pdf is submission-ready."
else
    echo "BUILD-MANUSCRIPT: FAIL — fix the rows above before submitting." >&2
fi
exit "$fail"
