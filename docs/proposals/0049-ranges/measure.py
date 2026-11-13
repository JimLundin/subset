"""Measure isolated range examples, not Conductor source."""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path(__file__).resolve().parent
BINARY = str(ROOT / "target/debug/subset")


def run(*args):
    return json.loads(subprocess.check_output(args, cwd=ROOT, text=True))


def main():
    results = {"kind": "illustrative isolated examples, not Conductor source",
               "configuration_version": 13, "examples": {}}
    for name in ("before", "after"):
        source = EVIDENCE / f"{name}.rs"
        relative = str(source.relative_to(ROOT))
        scratch = ROOT / f"target/issue6-ranges/evidence-{name}"
        fixture = scratch / "crates/daemon/src/example.rs"
        fixture.parent.mkdir(parents=True, exist_ok=True)
        fixture.write_text(source.read_text(), encoding="utf-8")
        (scratch / "subset.toml").write_text((ROOT / "subset.toml").read_text(), encoding="utf-8")
        check = subprocess.run([BINARY, "check"], cwd=scratch, text=True, capture_output=True)
        results["examples"][name] = {
            "score": run(BINARY, "score", relative, "--by", "fn", "--json"),
            "measure": run(BINARY, "measure", relative, "--json"),
            "check": {"exit": check.returncode, "stdout": check.stdout, "stderr": check.stderr},
        }
        if check.returncode != 0:
            raise RuntimeError(f"{name}: {check.stdout}{check.stderr}")
    (EVIDENCE / "measurements.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
