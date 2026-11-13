#!/usr/bin/env bash
# Read-only reference extraction; all generated files stay in this project.
set -euo pipefail
evidence=$(cd "$(dirname "$0")" && pwd)
project=$(cd "$evidence/../../.." && pwd)
reference=${CONDUCTOR_REFERENCE:-/home/jim/projects/apps/conductor}
revision=79568ce8a55b266fe1cf96b17dd5b2bc2dfa0c8c
scratch="$project/target/issue5-review/conductor"
mkdir -p "$scratch"
git -C "$reference" archive "$revision" | tar -x -C "$scratch"
python3 "$evidence/reproduce.py"
# Optional additional validation, requiring cargo and network/cached crates:
# python3 "$evidence/verify_grant.py"
