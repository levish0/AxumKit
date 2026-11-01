"""Resolve the application image and reject tags that differ from its version."""

import argparse
from pathlib import Path
import re
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--tag")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    with (root / "Cargo.toml").open("rb") as manifest:
        version = tomllib.load(manifest)["workspace"]["package"]["version"]
    if not isinstance(version, str) or not re.fullmatch(
        r"[0-9][A-Za-z0-9_.-]{0,127}", version
    ):
        parser.error("workspace version must be a valid container tag")
    if args.tag is not None and args.tag != f"v{version}":
        parser.error(f"release tag must be exactly v{version}")
    repository = args.repository.lower()
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]*/[a-z0-9][a-z0-9_.-]*", repository):
        parser.error("repository must be an owner/name pair")
    print(f"version={version}")
    print(f"image=ghcr.io/{repository}")


if __name__ == "__main__":
    main()
