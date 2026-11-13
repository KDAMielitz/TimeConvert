#!/usr/bin/env python3
"""Validate Time Convert release tags and, for production, the Cargo version."""
import os
import re
import sys
import tomllib
from pathlib import Path

TAG = re.compile(r"v([0-9]+)\.([0-9]+)\.([0-9]+)\.([0-9]+)([ab]?)\Z")

def classify(tag, cargo_version):
    match = TAG.fullmatch(tag)
    if not match:
        raise ValueError("expected vMAJOR.MILESTONE.PRODUCTION.0 or vMAJOR.MILESTONE.PRODUCTION.TESTING[a|b]")
    major, milestone, production, testing, suffix = match.groups()
    version = f"{major}.{milestone}.{production}"
    if testing == "0" and not suffix:
        if version != cargo_version:
            raise ValueError(f"production tag version {version} does not match Cargo version {cargo_version}")
        return "production"
    if suffix and int(testing) > 0:
        return "testing"
    raise ValueError("production must end in .0; testing must use a nonzero revision followed by a or b")

def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: parse_tag.py TAG")
    with Path("Cargo.toml").open("rb") as f:
        version = tomllib.load(f)["package"]["version"]
    try:
        kind = classify(sys.argv[1], version)
    except ValueError as exc:
        raise SystemExit(f"Invalid release tag {sys.argv[1]!r}: {exc}") from exc
    print(kind)
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as out:
            print(f"kind={kind}", file=out)

if __name__ == "__main__":
    main()
