#!/usr/bin/env python3
r"""
count_words.py — the DOCUMENTED, versioned word-count method for the
manuscript (OS-11). IEEE Computer limits: abstract <= 150 words; body
4,000--6,000 words (H1/H3).

Method (deterministic, no external tools):
  1. Read the .tex file.
  2. Strip comments: everything from an unescaped % to end of line.
  3. For the abstract: extract the body of the abstract environment,
     drop the \begin/\end lines, replace inline math ($...$) and
     \multimap-style symbols with a single placeholder token each.
  4. For body sections: drop \begin{...}/\end{...} lines, \input lines,
     table/figure environments entirely (IEEE counts running prose),
     \caption text EXCLUDED (counted separately if requested), and
     remove LaTeX commands (\word-like tokens) before counting.
  5. Count remaining whitespace-separated tokens that contain at least
     one alphanumeric character.

The method is deliberately conservative and simple; the count it prints
is the number the response letter cites.
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PAPER = REPO / "paper1"

BODY_SECTIONS = [
    "section1_introduction.tex",
    "section2_related_work.tex",
    "section3_type_system.tex",
    "section4_theorem.tex",
    "section5_benchmarks.tex",
    "section6_discussion.tex",
    "section7_conclusion.tex",
]


def strip_comments(text: str) -> str:
    return re.sub(r"(?<!\\)%.*", "", text)


def strip_commands(text: str) -> str:
    # \command{arg} -> arg (one level), \command* -> drop, \command -> drop
    text = re.sub(r"\\[a-zA-Z]+\*?(\[[^\]]*\])?", " ", text)
    text = text.replace("{", " ").replace("}", " ")
    text = re.sub(r"\$[^$]*\$", " MATH ", text)
    return text


def count_tokens(text: str) -> int:
    return sum(
        1
        for tok in strip_commands(strip_comments(text)).split()
        if re.search(r"[A-Za-z0-9]", tok)
    )


def abstract_words() -> int:
    text = (PAPER / "abstract.tex").read_text()
    m = re.search(
        r"\\begin\{abstract\}(.*?)\\end\{abstract\}", strip_comments(text), re.DOTALL
    )
    return count_tokens(m.group(1)) if m else 0


def body_words() -> int:
    total = 0
    for name in BODY_SECTIONS:
        text = strip_comments((PAPER / name).read_text())
        # Drop environments entirely (tables, figures, listings, verbatim,
        # equations) — IEEE counts running prose.
        text = re.sub(
            r"\\begin\{(table\*?|figure\*?|lstlisting|verbatim|align\*?|equation\*?)\}.*?\\end\{\1\}",
            " ",
            text,
            flags=re.DOTALL,
        )
        text = re.sub(r"\\input\{[^}]*\}", " ", text)
        total += count_tokens(text)
    return total


def main() -> int:
    a = abstract_words()
    b = body_words()
    print(f"abstract words: {a} (limit 150)")
    print(f"body words:     {b} (limit 4000-6000, running prose; "
          "tables/figures/equations excluded by this method)")
    ok = a <= 150 and 4000 <= b <= 6000
    print("GATE:", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
