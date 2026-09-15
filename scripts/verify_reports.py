#!/usr/bin/env python3
"""
verify_reports.py — reconcile every number in reports/tcb_summary.md against
its raw source (OS-05, closes F5). Wired into CI.

Method: re-run scripts/gen_tcb_summary.py (the single generator) and
byte-compare its output with the committed reports/tcb_summary.md. ANY
divergence — a hand-edited number, a stale raw capture, a dependency change
not reflected — exits 1 with a diff.

Additional independent cross-checks (do not rely on the generator's own
arithmetic):
  1. Every (crate, version, numbers) row quoted in the tcb_summary Top-10
     table must equal the corresponding CSV row.
  2. The blake3 total quoted in the prose must equal the CSV total.
  3. The audit JSON must be present, parseable, and consistent with the
     count quoted in the summary.
  4. The `rust-toolchain.toml` channel (when present) must be what the
     summary declares.
  5. Zero `std::mem::forget` in the fail-secure path (OS-06 state) must
     match the summary's claim.
"""

import csv
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
REPORTS = REPO_ROOT / "reports"
GEN = REPO_ROOT / "scripts" / "gen_tcb_summary.py"


def fail(msg: str) -> int:
    print(f"VERIFY-REPORTS: FAIL — {msg}", file=sys.stderr)
    return 1


def main() -> int:
    summary_path = REPORTS / "tcb_summary.md"
    if not summary_path.exists():
        return fail("reports/tcb_summary.md missing")
    committed = summary_path.read_text()

    # --- 0. Canonical regeneration diff -------------------------------------
    fresh = subprocess.run(
        [sys.executable, str(GEN)], capture_output=True, text=True
    )
    if fresh.returncode != 0:
        return fail(f"generator crashed: {fresh.stderr[-400:]}")
    regenerated = summary_path.read_text()
    if regenerated != committed:
        import difflib

        diff = "\n".join(
            list(
                difflib.unified_diff(
                    committed.splitlines(),
                    regenerated.splitlines(),
                    "committed",
                    "regenerated",
                    lineterm="",
                )
            )[:60]
        )
        return fail(
            "tcb_summary.md diverges from its data sources "
            "(hand-edited number or stale raw capture):\n" + diff
        )

    # --- 1. Top-10 rows vs CSV ----------------------------------------------
    csv_rows = {
        r["name"]: r
        for r in csv.DictReader(open(REPORTS / "cargo_geiger_unsafe_inventory.csv"))
    }
    committed = regenerated  # now the regenerated (canonical) text
    table_rows = re.findall(
        r"^\| ([a-zA-Z0-9_-]+) \| ([\w.-]+) \| (\d+) \| (\d+) \| (\d+) \| (\d+) \| (\d+) \|",
        committed,
        re.MULTILINE,
    )
    if not table_rows:
        return fail("Top-10 table not found in summary")
    for name, version, files, blocks, fns, impls, total in table_rows:
        src = csv_rows.get(name)
        if src is None:
            return fail(f"Top-10 row {name} not in CSV")
        expected = (
            src["version"],
            src["files"],
            src["unsafe_blocks"],
            src["unsafe_fns"],
            src["unsafe_impls"],
            src["total"],
        )
        got = (version, files, blocks, fns, impls, total)
        if got != expected:
            return fail(f"Top-10 row {name}: {got} != CSV {expected}")

    # --- 2. blake3 prose total ----------------------------------------------
    m = re.search(r"blake3` exact row: .*total (\d+)\*\*", committed)
    if not m:
        return fail("blake3 prose line not found")
    if int(m.group(1)) != int(csv_rows["blake3"]["total"]):
        return fail("blake3 total in prose != CSV")

    # --- 3. Audit JSON vs summary claim -------------------------------------
    result = json.loads((REPORTS / "cargo_audit_result.json").read_text())
    count = result["vulnerabilities"]["count"]
    m = re.search(r"vulnerabilities\.count = (\d+)", committed)
    if not m or int(m.group(1)) != count:
        return fail("audit vulnerability count mismatch vs JSON")

    # --- 4. Toolchain channel ------------------------------------------------
    tc_file = REPO_ROOT / "rust-toolchain.toml"
    if tc_file.exists():
        channel = tomllib.loads(tc_file.read_text())["toolchain"]["channel"]
        if channel not in committed:
            return fail(f"toolchain channel {channel} not declared in summary")

    # --- 5. mem::forget claim -------------------------------------------------
    lib_rs = (REPO_ROOT / "btv-core" / "src" / "lib.rs").read_text()
    forget_count = len(re.findall(r"std::mem::forget\(", lib_rs))
    m = re.search(r"contém (\d+) chamada\(s\) a `std::mem::forget`", committed)
    if not m or int(m.group(1)) != forget_count:
        return fail("mem::forget count mismatch vs source")

    # --- 6. README's inventory annotation ------------------------------------
    #
    # COMSI-2026-04-0112 Round 3: the README's file-tree comment claimed
    # "113 deps, 63 with unsafe" long after the workspace unification made
    # the real figures 232 and 139 — a documentation number that had drifted
    # from the evidence file it describes, which is precisely the defect
    # class this verifier exists to close. The tcb_summary check above never
    # saw it because the README is not the summary. It is now covered.
    readme = REPO_ROOT / "README.md"
    if readme.exists():
        geiger_rows = list(
            csv.DictReader((REPORTS / "cargo_geiger_unsafe_inventory.csv").open())
        )
        n_deps = len(geiger_rows)
        n_unsafe = sum(1 for r in geiger_rows if int(r["total"]) > 0)
        m = re.search(r"(\d+) deps, (\d+) with unsafe", readme.read_text())
        if not m:
            return fail("README no longer carries the geiger inventory annotation")
        if (int(m.group(1)), int(m.group(2))) != (n_deps, n_unsafe):
            return fail(
                f"README says {m.group(1)} deps / {m.group(2)} with unsafe; "
                f"cargo_geiger_unsafe_inventory.csv says {n_deps} / {n_unsafe}"
            )

    print(
        "VERIFY-REPORTS: OK — every figure in reports/tcb_summary.md and the "
        "README's inventory annotation re-derives from the committed raw evidence."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
