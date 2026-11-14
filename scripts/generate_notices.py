#!/usr/bin/env python3
"""Administrative license inventory. This script is not a Prumo runtime module."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TARGETS = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
NOTICE = ROOT / "THIRD_PARTY_NOTICES.md"
INVENTORY = ROOT / "docs/componentes/dependencias-pru001.json"
UPSTREAM = ROOT / "docs/componentes/upstream-notices"
# These inspected source distributions carry additional per-file attributions.
# Keep their legal comment blocks as well as their root licenses. The lock and
# package checksum identify the exact source of every preserved block.
SOURCE_COMMENT_NOTICES = {
    "approx", "geo", "heapless", "itertools", "lazy_static", "matrixmultiply",
    "minimal-lexical", "num-complex", "num-integer", "num-rational",
    "proc-macro-crate", "proc-macro-error", "rand", "rand_pcg", "rawpointer",
    "robust", "serde_json", "spin", "stable_deref_trait", "wasmparser", "wide",
}


def cargo(*args):
    return subprocess.check_output(["cargo", *args], cwd=ROOT, text=True)


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


def generate():
    metadata = json.loads(cargo("metadata", "--locked", "--format-version", "1"))
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    checksums = {(p["name"], p["version"]): p.get("checksum") for p in lock["package"]}
    targets = {}
    for target in TARGETS:
        tree = cargo("tree", "--workspace", "--locked", "--target", target,
                     "--edges", "normal,build", "--prefix", "none", "--format", "{p}")
        for line in tree.splitlines():
            match = re.match(r"([^ ]+) v([^ ]+)", line)
            if match:
                targets.setdefault((match[1], match[2]), []).append(target)
    index = json.loads((UPSTREAM / "index.json").read_text())
    texts = {}
    records = []
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if not package.get("source"):
            continue
        key = (package["name"], package["version"])
        base = Path(package["manifest_path"]).parent
        notice_ids = []

        def add(text, origin):
            original_text_sha256 = sha(text)
            text = "\n".join(line.rstrip(" \t") for line in text.strip().splitlines()) + "\n"
            digest = sha(text)
            record = texts.setdefault(digest, {"text": text, "origins": [], "source_text_sha256": {}})
            if origin not in record["origins"]:
                record["origins"].append(origin)
            record["source_text_sha256"][origin] = original_text_sha256
            if digest not in notice_ids:
                notice_ids.append(digest)

        if key in targets:
            for path in sorted(base.rglob("*")):
                if not path.is_file():
                    continue
                is_notice = bool(re.search(r"license|licence|copying|copyright|notice", path.name, re.I))
                is_font_notice = path.parent.name == "fonts" and path.suffix == ".txt"
                if (is_notice and path.suffix.lower() not in [".rs", ".js", ".html", ".json", ".svg", ".png"]) or is_font_notice:
                    add(path.read_text(encoding="utf-8"), f"crate:{key[0]}@{key[1]}/{path.relative_to(base).as_posix()}")
            repository = package.get("repository") or ""
            for source in index:
                selected_packages = source.get("packages")
                applies = (f"{key[0]}@{key[1]}" in selected_packages) if selected_packages else source["repo"] in repository
                if applies:
                    add((ROOT / source["local"]).read_text(), source["url"])
            if package["name"] in SOURCE_COMMENT_NOTICES:
                for path in sorted(base.rglob("*.rs")):
                    contents = path.read_text(encoding="utf-8")
                    comments = re.finditer(r"/\*[\s\S]*?\*/|^[ \t]*//[^\n]*(?:\n[ \t]*//[^\n]*)*", contents, re.M)
                    for match in comments:
                        block = match.group()
                        if re.search(r"copyright|SPDX-License-Identifier|licensed under|Boost Software License|permission to use, copy", block, re.I):
                            line = contents.count("\n", 0, match.start()) + 1
                            add(block, f"crate:{key[0]}@{key[1]}/{path.relative_to(base).as_posix()}:{line} (legal comment)")
            if package["name"] in ["gl_generator", "khronos_api"]:
                # These .crate archives carry license headers instead of root license files.
                apache = next(source for source in index if source["repo"] == "emilk/egui" and source["path"] == "LICENSE-APACHE")
                add((ROOT / apache["local"]).read_text(), apache["url"])
                for path in sorted(base.rglob("*")):
                    if not path.is_file() or path.suffix not in [".rs", ".xml", ".idl"]:
                        continue
                    text = path.read_text(encoding="utf-8")
                    source = f"crate:{key[0]}@{key[1]}/{path.relative_to(base).as_posix()}"
                    if path.suffix in [".rs", ".idl"]:
                        lines = []
                        for line in text.splitlines():
                            if line.startswith("//") or not line.strip():
                                lines.append(line)
                            else:
                                break
                        header = "\n".join(lines)
                        if re.search(r"copyright", header, re.I):
                            add(header, source + " (license header)")
                    else:
                        for a, b in re.findall(r"<!--(.*?)-->|<comment>(.*?)</comment>", text, re.S):
                            if re.search(r"copyright|permission is hereby", a + b, re.I):
                                add(a + b, source + " (license header)")
                if package["name"] == "khronos_api":
                    add((UPSTREAM / "google-angle/LICENSE").read_text(), "https://api.github.com/repos/google/angle/contents/LICENSE (retrieved 2026-11-14)")
            if not notice_ids:
                raise RuntimeError(f"Missing license text for distributed dependency {key}")
        records.append({
            "name": key[0], "version": key[1], "source": package["source"],
            "checksum": checksums[key], "repository": package.get("repository"),
            "license_expression": package.get("license"), "authors": package.get("authors", []),
            "native_build_or_runtime_targets": sorted(set(targets.get(key, []))),
            "license_text_sha256": sorted(notice_ids),
        })
    inventory = {
        "schema_version": 1,
        "scope": "Full Cargo.lock metadata; notices cover normal/build graph of distributed native targets, not only directly declared crates.",
        "lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "native_targets": TARGETS,
        "packages": records,
        "license_text_normalization": "Displayed text strips trailing spaces/tabs and outer blank lines. source_text_sha256 retains the UTF-8 text received from each source before presentation normalization; preserved upstream files remain unchanged.",
        "license_texts": [{"sha256": digest, "origins": sorted(record["origins"]), "source_text_sha256": dict(sorted(record["source_text_sha256"].items()))} for digest, record in sorted(texts.items())],
    }
    lines = ["# Prumo: third-party notices", "",
             "This file preserves license texts and attributions for the normal/build dependency graph of the Windows and Linux development binaries. Build tools are included conservatively; a listed component is not necessarily linked into every executable. The complete locked inventory also identifies test and other-platform dependencies separately.", "",
             "The Prumo product license is not defined by these third-party licenses. No dependency has been patched in this release. The original texts below govern the respective components; SPDX metadata alone is not a replacement for their conditions.", "",
             "Cargo.lock SHA-256: `" + inventory["lock_sha256"] + "`", "",
             "## Component inventory", "", "| Package | Declared license | License text sections |", "|---|---|---|"]
    for record in records:
        if record["native_build_or_runtime_targets"]:
            refs = ", ".join("[" + digest[:12] + "](#license-" + digest[:12] + ")" for digest in record["license_text_sha256"])
            name = record["name"] + " " + record["version"]
            lines.append(f"| [{name}](https://crates.io/crates/{record['name']}/{record['version']}) | {record['license_expression']} | {refs} |")
    lines += ["", "## Preserved license texts", "",
              "Font notices include the OFL, Ubuntu Font Licence, Bitstream Vera clauses, and copyright/permission notices bundled with the actual font files. SDK notices retain both Apache and MIT transition terms. Khronos registry and ANGLE notices are included with generated graphics bindings."]
    for digest, record in sorted(texts.items()):
        lines += ["", "### License " + digest[:12], "", "Sources:", ""]
        lines.extend("- " + origin for origin in sorted(record["origins"]))
        lines += ["", "````text", record["text"].rstrip(), "````"]
    return "\n".join(lines) + "\n", json.dumps(inventory, ensure_ascii=False, indent=2) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    notices, inventory = generate()
    for path, content in [(NOTICE, notices), (INVENTORY, inventory)]:
        if args.check:
            if not path.exists() or path.read_text() != content:
                raise SystemExit(f"Outdated notices: {path.relative_to(ROOT)}; run python3 scripts/generate_notices.py")
        else:
            path.write_text(content, encoding="utf-8", newline="\n")
    print("Third-party notices and locked dependency inventory verified." if args.check else "Third-party notices and locked dependency inventory generated.")


if __name__ == "__main__":
    main()
