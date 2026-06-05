#!/usr/bin/env python3
"""Generate a signed release/promotion manifest.

The manifest is designed to be immutable and safely promotable across
environments. It intentionally captures *hashes* of referenced files
instead of embedding their full contents.
"""

from __future__ import annotations

import argparse
import datetime as dt
import glob
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Dict, List


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def file_entry(root: Path, path: Path) -> Dict[str, Any]:
    rel = path.relative_to(root).as_posix()
    st = path.stat()
    return {
        "path": rel,
        "bytes": st.st_size,
        "sha256": sha256_file(path),
    }


def collect_migrations(root: Path) -> List[Dict[str, Any]]:
    paths = sorted(
        Path(p)
        for p in glob.glob(str(root / "crates" / "*" / "migrations" / "*.sql"))
    )
    return [file_entry(root, p) for p in paths]


def collect_files(root: Path, rel_paths: List[str]) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    for rel in rel_paths:
        p = root / rel
        if not p.exists():
            continue
        out.append(file_entry(root, p))
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", required=True)
    ap.add_argument("--git-sha", required=True)
    ap.add_argument("--git-ref", required=True)
    ap.add_argument("--image-ref", required=True)
    ap.add_argument("--image-digest", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    root = Path(os.getcwd()).resolve()

    config_files = [
        "crates/convergio-server/README.md",
        "crates/convergio-server/src/main.rs",
        "deny.toml",
        ".cargo/audit.toml",
        ".cargo/config.toml",
        "rust-toolchain.toml",
    ]

    # Closest thing to a model-card snapshot in this repo today: the evaluation framework ADR.
    model_card_files = [
        "docs/adr/0020-model-evaluation-framework.md",
    ]

    migrations = collect_migrations(root)

    manifest: Dict[str, Any] = {
        "schema_version": "1",
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "git": {
            "sha": args.git_sha,
            "ref": args.git_ref,
        },
        "version": {
            "release_tag": args.tag,
            "ontology_version": args.tag,
        },
        "image": {
            "ref": args.image_ref,
            "digest": args.image_digest,
            "qualified": f"{args.image_ref}@{args.image_digest}",
        },
        "migration_plan": {
            "kind": "sqlx-migrations-snapshot",
            "migrations": migrations,
        },
        "snapshots": {
            "config": collect_files(root, config_files),
            "model_card": collect_files(root, model_card_files),
        },
    }

    out_path = Path(args.out)
    out_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
