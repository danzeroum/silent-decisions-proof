#!/usr/bin/env bash
# build_manuscript.sh — compile paper1 to a submission-ready PDF and enforce
# the typographic gates (COMSI-2026-04-0112 Round 3).
#
# Until this round, NO round of this manuscript had ever been compiled: every
# check was structural (balanced environments, \ref/\label pairs, \cite keys).
# The first compile surfaced three tables overflowing IEEEtran's column
# measure by up to 99.7 pt — invisible to every structural check, obvious to
# any reader of the PDF. This script makes the compile a gate, not a hope.
#
# Exits non-zero on: any LaTeX error, any undefined reference or citation,
# any overfull hbox, or a BibTeX warning.
#
# Usage:  bash scripts/build_manuscript.sh
# Output: paper1/main.pdf
set -uo pipefail

cd "$(dirname "$0")/../paper1" || exit 1
rm -f main.aux main.bbl main.blg main.log main.out main.pdf

run() { timeout 400 pdflatex -interaction=nonstopmode main.tex >/dev/null 2>&1; }
run
timeout 300 bibtex main >/dev/null 2>&1
run
timeout 400 pdflatex -interaction=nonstopmode main.tex > /dev/null 2>&1

fail=0
report() { printf '%-34s %s\n' "$1" "$2"; }

if [ ! -f main.pdf ]; then
    report "PDF produced" "NO — compile failed"; exit 1
fi
pages=$(grep -oE 'Output written on main\.pdf \([0-9]+ pages' main.log | grep -oE '[0-9]+' | head -1)
report "PDF produced" "yes (${pages} pages)"

n=$(grep -c '^!' main.log || true);            report "LaTeX errors" "$n"; [ "$n" -eq 0 ] || fail=1
n=$(grep -ic 'undefined' main.log || true);    report "undefined refs/citations" "$n"; [ "$n" -eq 0 ] || fail=1
n=$(grep -c 'Overfull \\hbox' main.log || true); report "overfull hboxes" "$n"; [ "$n" -eq 0 ] || fail=1
n=$(grep -c '^\\bibitem' main.bbl || true);    report "bibliography entries" "$n"
n=$(grep -ci 'warning' main.blg || true)
# bibtex's own tally line ("warning$ -- 0") always matches; count real ones only
n=$(grep -i 'warning' main.blg | grep -vc 'warning\$' || true)
report "BibTeX warnings" "$n"; [ "$n" -eq 0 ] || fail=1

if [ "$fail" -eq 0 ]; then
    echo "BUILD-MANUSCRIPT: OK — main.pdf is submission-ready."
else
    echo "BUILD-MANUSCRIPT: FAIL — fix the rows above before submitting." >&2
fi
exit "$fail"
