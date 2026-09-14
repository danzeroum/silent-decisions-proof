#!/usr/bin/env python3
"""
Documented, versioned word-count method for the IEEE Computer submission
(OS-11, COMSI-2026-04-0112, closes A4/H2's methodology gap).

The manuscript was bounced once by the desk (13-Apr-2026) for exceeding
the venue's word limits, and the audit separately found the body word
count undocumented ("método de contagem não documentado", H3). This
script is that method: resolve every `\\input{...}` in main.tex, convert
the result LaTeX -> plain text with pandoc (so `\\texttt{...}`, math, and
markup are stripped rather than counted as extra words the way a raw
`wc -w` over the .tex source would), and count words.

Usage:
  python3 scripts/count_words.py            # whole manuscript body
  python3 scripts/count_words.py --abstract # abstract.tex alone (<=150 words, OS-11 gate)

Requires `pandoc` (not vendored; install with the system package manager).
"""
import argparse
import re
import subprocess
import sys
from pathlib import Path

PAPER1 = Path(__file__).resolve().parent.parent / "paper1"


def resolve_inputs(tex: str) -> str:
    def resolve(m: re.Match) -> str:
        name = m.group(1)
        if not name.endswith(".tex"):
            name += ".tex"
        fpath = PAPER1 / name
        if not fpath.exists():
            print(f"WARNING: \\input target missing: {fpath}", file=sys.stderr)
            return ""
        return fpath.read_text()

    resolved = re.sub(r"\\input\{([^}]+)\}", resolve, tex)
    # Pandoc's LaTeX reader treats \begin{abstract}...\end{abstract} as a
    # METADATA field (like \title/\author) and the `plain` writer silently
    # drops it — verified by a 0-word count with the environment intact.
    # Strip the markers so the abstract's prose counts as ordinary body
    # text, here exactly as in the standalone --abstract path below.
    return resolved.replace("\\begin{abstract}", "").replace(
        "\\end{abstract}", ""
    )


def pandoc_word_count(tex_source: str) -> int:
    result = subprocess.run(
        ["pandoc", "-f", "latex", "-t", "plain"],
        input=tex_source.encode("utf-8"),
        capture_output=True,
        check=True,
    )
    return len(result.stdout.split())


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--abstract", action="store_true", help="count abstract.tex alone"
    )
    args = parser.parse_args()

    if args.abstract:
        # abstract.tex is a bare `abstract` environment. Pandoc's LaTeX
        # reader treats \begin{abstract}...\end{abstract} as a METADATA
        # field (like \title/\author), not body content, so the `plain`
        # writer silently omits it (observed: 0 words) regardless of
        # document wrapping. Strip the environment markers and feed the
        # inner prose as ordinary body text instead.
        body = (PAPER1 / "abstract.tex").read_text()
        inner = body.replace("\\begin{abstract}", "").replace(
            "\\end{abstract}", ""
        )
        source = (
            "\\documentclass{article}\n\\begin{document}\n"
            + inner
            + "\n\\end{document}\n"
        )
        label = "abstract.tex"
    else:
        source = resolve_inputs((PAPER1 / "main.tex").read_text())
        label = "main.tex (\\input resolved, bibliography excluded)"

    n = pandoc_word_count(source)
    print(f"{label}: {n} words")


if __name__ == "__main__":
    main()
