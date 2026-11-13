"""Measure illustrative v11/v12 syntax; these are not Conductor files."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path(__file__).resolve().parent


def run(*args):
    return json.loads(subprocess.check_output(args, cwd=ROOT, text=True))


def main():
    # Use the current binary for both: all before syntax was allowed in v11,
    # and its scoring tiers are unchanged. The after uses the v12 tiers.
    binary = str(ROOT / "target/debug/subset")
    results = {"kind": "illustrative isolated examples, not Conductor source",
               "configuration_version": 12, "examples": {}}
    for name in ("before", "after"):
        source = str((EVIDENCE / f"{name}.rs").relative_to(ROOT))
        results["examples"][name] = {
            "score": run(binary, "score", source, "--by", "fn", "--json"),
            "measure": run(binary, "measure", source, "--json"),
        }
    (EVIDENCE / "measurements.json").write_text(
        json.dumps(results, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
