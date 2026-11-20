#!/usr/bin/env python3
"""Empacotamento administrativo do visualizador Rust/WASM; não integra o produto."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGET = "wasm32-unknown-unknown"
BINDGEN_VERSION = "0.2.130"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def output(*command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--target-dir", type=Path)
    parser.add_argument("--wasm-bindgen", default="wasm-bindgen")
    args = parser.parse_args()
    version = output(args.wasm_bindgen, "--version")
    if version != f"wasm-bindgen {BINDGEN_VERSION}":
        raise SystemExit(f"wasm-bindgen deve ser {BINDGEN_VERSION}; encontrado: {version}")
    if not args.skip_build:
        subprocess.run(
            ["cargo", "build", "-p", "prumo-web", "--target", TARGET, "--locked", "-j1"],
            cwd=ROOT, check=True,
        )
    target_dir = args.target_dir
    if target_dir is None:
        metadata = json.loads(output("cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"))
        target_dir = Path(metadata["target_directory"])
    wasm = target_dir / TARGET / "debug" / "prumo_web.wasm"
    if not wasm.is_file() or wasm.read_bytes()[:8] != b"\0asm\x01\0\0\0":
        raise SystemExit("Módulo WASM de desenvolvimento ausente ou inválido: " + str(wasm))
    destination = ROOT / "dist" / "prumo-web"
    destination.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [args.wasm_bindgen, str(wasm), "--target", "web", "--out-dir", str(destination / "pkg")],
        cwd=ROOT, check=True,
    )
    for source in (ROOT / "apps/prumo-web/index.html", ROOT / "THIRD_PARTY_NOTICES.md"):
        shutil.copy2(source, destination / source.name)
    (destination / "COMECAR.md").write_text(
        "# Prumo: visualizador web de desenvolvimento\n\n"
        "Sirva esta pasta por HTTP local, por exemplo com `python -m http.server 8080`, "
        "e abra `http://localhost:8080`. Não abra o HTML por `file://`, pois o navegador "
        "precisa carregar o módulo WASM pelo mesmo servidor. O servidor apenas fornece "
        "os arquivos estáticos. Nenhum projeto é enviado a ele.\n\n"
        "Na aplicação nativa, abra o projeto e use **Exportar projeto para vista web**. "
        "Selecione o arquivo `.view.json` no controle **Arquivo de vista web**. "
        "Também é possível colar o JSON integral obtido pela ferramenta MCP de leitura. "
        "A identidade e a revisão são preservadas. A navegação nesta aba não salva "
        "alterações no arquivo de origem.\n\n"
        "Use Tab e Enter para operar os controles textuais. A lista inclui objetos "
        "ocultos; selecionar e localizar revela a peça. Se a vista gráfica falhar, "
        "a lista e as propriedades permanecem disponíveis.\n\n"
        "Este pacote é de desenvolvimento. Não fornece cálculo, dimensionamento, "
        "emissão técnica ou persistência de autoria no navegador. Acessibilidade "
        "com leitor de tela humano e cada backend gráfico exigem sua própria "
        "validação. Leia THIRD_PARTY_NOTICES.md.\n",
        encoding="utf-8",
    )
    names = ["index.html", "THIRD_PARTY_NOTICES.md", "COMECAR.md"]
    names += sorted(str(p.relative_to(destination)).replace(os.sep, "/")
                    for p in (destination / "pkg").rglob("*") if p.is_file())
    if not {"pkg/prumo_web.js", "pkg/prumo_web_bg.wasm"}.issubset(names):
        raise SystemExit("Bindings gerados incompletos.")
    manifest = {
        "stage": "development", "target": TARGET, "profile": "debug",
        "source": output("git", "rev-parse", "HEAD"),
        "source_dirty": bool(output("git", "status", "--porcelain")),
        "compiler": output("rustc", "--version", "--verbose"),
        "cargo_lock_sha256": digest(ROOT / "Cargo.lock"),
        "wasm_bindgen": version, "input_wasm_sha256": digest(wasm),
        "browser_execution_verified_by_packager": False,
        "files": {name: digest(destination / name) for name in names},
    }
    (destination / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    archive = destination.parent / "prumo-web-development.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as package:
        for name in [*names, "manifest.json"]:
            package.write(destination / name, name)
    with tempfile.TemporaryDirectory() as temporary:
        with zipfile.ZipFile(archive) as package:
            package.extractall(temporary)
        extracted = Path(temporary)
        for name, expected in manifest["files"].items():
            if digest(extracted / name) != expected:
                raise SystemExit("Integridade divergente: " + name)
        if (extracted / "pkg/prumo_web_bg.wasm").read_bytes()[:8] != b"\0asm\x01\0\0\0":
            raise SystemExit("Cabeçalho WASM extraído inválido.")
    print(f"{archive.name}: bindings {BINDGEN_VERSION}, extração e hashes conferidos; execução no browser é um ensaio separado.")


if __name__ == "__main__":
    main()
