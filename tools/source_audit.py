from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXCLUDED_DIRECTORIES = {"target", "node_modules", ".git"}
FORBIDDEN_TOKENS = (
    "todo!()",
    "unimplemented!()",
    'panic!("not implemented")',
)
TEXT_EXTENSIONS = {
    ".css",
    ".html",
    ".json",
    ".md",
    ".ps1",
    ".py",
    ".rs",
    ".sh",
    ".toml",
    ".ts",
    ".tsx",
    ".wgsl",
    ".yaml",
    ".yml",
}


def is_included(path: Path) -> bool:
    if not path.is_file():
        return False
    if path.name == "SOURCE_MANIFEST.json":
        return False
    return not any(part in EXCLUDED_DIRECTORIES for part in path.parts)


def audit() -> tuple[list[dict[str, object]], list[str]]:
    issues: list[str] = []
    manifest: list[dict[str, object]] = []

    for path in sorted(ROOT.rglob("*")):
        if not is_included(path):
            continue

        data = path.read_bytes()
        relative_path = path.relative_to(ROOT).as_posix()
        manifest.append(
            {
                "path": relative_path,
                "bytes": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
            }
        )

        if path.suffix not in TEXT_EXTENSIONS:
            continue

        text = data.decode("utf-8", "replace")
        for token in FORBIDDEN_TOKENS:
            if token in text and path.name != "source_audit.py":
                issues.append(f"{relative_path}: forbidden token {token}")

        if path.suffix == ".rs" and not text.strip():
            issues.append(f"{relative_path}: empty Rust source")

        if path.suffix in {".rs", ".ts", ".tsx", ".py", ".wgsl"}:
            for line_number, line in enumerate(text.splitlines(), start=1):
                if len(line) > 240:
                    issues.append(
                        f"{relative_path}:{line_number}: source line exceeds 240 characters"
                    )

    return manifest, issues


def main() -> int:
    manifest, issues = audit()
    manifest_path = ROOT / "SOURCE_MANIFEST.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    total_bytes = sum(int(entry["bytes"]) for entry in manifest)
    print(f"audited {len(manifest)} files, {total_bytes} bytes")

    if issues:
        print("\n".join(issues))
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
