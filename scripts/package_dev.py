#!/usr/bin/env python3
"""Empacotamento administrativo. Não integra o runtime Rust do produto."""
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def main():
    suffix = '.exe' if os.name == 'nt' else ''
    files = {name + suffix: ROOT / 'target' / 'debug' / (name + suffix)
             for name in ('prumo-desktop', 'prumo-mcp')}
    files['THIRD_PARTY_NOTICES.md'] = ROOT / 'THIRD_PARTY_NOTICES.md'
    files['DESENVOLVIMENTO.md'] = ROOT / 'docs' / 'DESENVOLVIMENTO.md'
    for path in files.values():
        if not path.is_file():
            raise SystemExit('Arquivo obrigatório ausente: ' + str(path))
    target = 'windows-x86_64' if os.name == 'nt' else 'linux-x86_64'
    destination = ROOT / 'dist' / ('prumo-development-' + target + '.zip')
    destination.parent.mkdir(exist_ok=True)
    manifest = {
        'stage': 'development', 'target': target,
        'compiler': subprocess.check_output(['rustc', '--version', '--verbose'], text=True),
        'platform': platform.platform(),
        'source': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
        'files': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name,path in files.items()},
    }
    instructions = (
        '# Prumo: versão de desenvolvimento\n\n'
        'Extraia todos os arquivos para uma pasta. Abra prumo-desktop para criar ou abrir um projeto local. '
        'Não é necessário editar configuração. No Linux, marque os binários como executáveis se o extrator não conservar a permissão.\n\n'
        'O servidor MCP usa: prumo-mcp --project seu-arquivo.prumo\n'
        'A leitura é o padrão. Acrescente --allow-write apenas quando quiser autorizar alterações.\n\n'
        'Cálculo, dimensionamento e emissão técnica ainda não estão disponíveis. '
        'Leia THIRD_PARTY_NOTICES.md para os direitos dos componentes incorporados.\n'
    )
    with zipfile.ZipFile(destination,'w',zipfile.ZIP_DEFLATED) as package:
        for name,path in files.items(): package.write(path,name)
        package.writestr('COMECAR.md',instructions)
        package.writestr('manifest.json',json.dumps(manifest,ensure_ascii=False,indent=2))
    with tempfile.TemporaryDirectory() as directory:
        with zipfile.ZipFile(destination) as package: package.extractall(directory)
        for name in ('prumo-desktop','prumo-mcp'):
            executable = Path(directory) / (name + suffix)
            executable.chmod(0o755)
            subprocess.run([str(executable),'--help'],check=True,timeout=15,stdout=subprocess.DEVNULL)
        for name,digest in manifest['files'].items():
            if hashlib.sha256((Path(directory)/name).read_bytes()).hexdigest()!=digest:
                raise SystemExit('Integridade divergente: '+name)
    print(destination.name + ': extração, hashes e inicialização dos dois executáveis aprovadas.')

if __name__ == '__main__': main()
