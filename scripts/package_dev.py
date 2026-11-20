#!/usr/bin/env python3
"""Empacotamento administrativo. Não integra o runtime Rust do produto."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[1]

EXPECTED_COMPONENTS = {
    'bim-ruststep': '0.4.0',
    'geometry-nalgebra': '0.35.0',
    'solver-nalgebra': '0.35.0',
    'sections-geo': '0.33.1',
    'documents-svg': '0.18.0',
    'wasmi-boundary': '2.0.0',
}


def native_target():
    machine = platform.machine().lower()
    if machine not in ('x86_64', 'amd64'):
        raise ValueError('Arquitetura fora da matriz qualificada: ' + machine)
    if os.name == 'nt':
        return 'windows-x86_64'
    if sys.platform.startswith('linux'):
        return 'linux-x86_64'
    raise ValueError('Sistema fora da matriz qualificada: ' + sys.platform)


def file_identity(path):
    with path.open('rb') as source:
        digest = hashlib.file_digest(source, 'sha256').hexdigest()
    return {'bytes': path.stat().st_size, 'sha256': digest}


def validate_report(report, binary):
    """Acceptance of this fixed administrative suite, without assertions disabled by -O."""
    if (not isinstance(report, dict)
            or type(report.get('protocol_version')) is not int
            or report['protocol_version'] != 1
            or report.get('suite_id') != 'PRU-003.1'
            or report.get('engineering_release') is not False
            or report.get('basis') is not None
            or report.get('platform') != native_target()
            or report.get('executable_sha256') != binary['sha256']
            or type(report.get('executable_bytes')) is not int
            or report['executable_bytes'] != binary['bytes']):
        raise ValueError('Protocolo, suíte, plataforma ou identidade do executável divergente.')
    request_id = uuid.UUID(report.get('request_id', ''))
    if request_id.int == 0:
        raise ValueError('O relatório não identifica a requisição.')
    experiments = report.get('experiments')
    if not isinstance(experiments, list) or len(experiments) != len(EXPECTED_COMPONENTS):
        raise ValueError('O relatório não contém os seis componentes esperados.')
    identities = []
    for experiment in experiments:
        component = experiment.get('component', {})
        identities.append((component.get('id'), component.get('version')))
        if (type(experiment.get('iterations')) is not int or experiment['iterations'] <= 0
                or type(experiment.get('elapsed_ns')) is not int or experiment['elapsed_ns'] < 0):
            raise ValueError('Contagem ou tempo inválido no experimento.')
        checks = experiment.get('checks')
        if not isinstance(checks, list) or not checks:
            raise ValueError('Experimento sem verificações.')
        for check in checks:
            if check.get('passed') is not True:
                raise ValueError('A bateria contém verificação reprovada.')
            error, tolerance = check.get('absolute_error'), check.get('tolerance')
            if error is None and tolerance is None:
                continue
            if (type(error) not in (int, float) or type(tolerance) not in (int, float)
                    or not math.isfinite(error) or not math.isfinite(tolerance)
                    or error < 0 or tolerance < 0 or error > tolerance):
                raise ValueError('Erro e tolerância numéricos são inválidos ou incoerentes.')
    if sorted(identities) != sorted(EXPECTED_COMPONENTS.items()):
        raise ValueError('Identidade, versão ou unicidade dos componentes divergente.')
    if type(report.get('elapsed_ns')) is not int or report['elapsed_ns'] < 0:
        raise ValueError('Tempo total inválido no relatório.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', choices=('debug', 'release'), default='debug')
    parser.add_argument('--target-dir', type=Path, default=ROOT / 'target')
    args = parser.parse_args()
    suffix = '.exe' if os.name == 'nt' else ''
    files = {name + suffix: args.target_dir / args.profile / (name + suffix)
             for name in ('prumo-desktop', 'prumo-mcp', 'prumo-component-worker')}
    files['THIRD_PARTY_NOTICES.md'] = ROOT / 'THIRD_PARTY_NOTICES.md'
    files['DESENVOLVIMENTO.md'] = ROOT / 'docs' / 'DESENVOLVIMENTO.md'
    for path in files.values():
        if not path.is_file():
            raise SystemExit('Arquivo obrigatório ausente: ' + str(path))
    target = native_target()
    profile_suffix = '-release' if args.profile == 'release' else ''
    destination = ROOT / 'dist' / ('prumo-development-' + target + profile_suffix + '.zip')
    destination.parent.mkdir(exist_ok=True)
    manifest = {
        'stage': 'development', 'target': target, 'profile': args.profile,
        'compiler': subprocess.check_output(['rustc', '--version', '--verbose'], cwd=ROOT, text=True),
        'platform': platform.platform(),
        'source': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'source_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True).strip()),
        'files': {name: hashlib.sha256(path.read_bytes()).hexdigest() for name,path in files.items()},
    }
    instructions = (
        '# Prumo: versão de desenvolvimento\n\n'
        'Extraia todos os arquivos para uma pasta. Abra prumo-desktop para criar ou abrir um projeto local. '
        'Não é necessário editar configuração. No Linux, marque os binários como executáveis se o extrator não conservar a permissão.\n\n'
        'O servidor MCP usa: prumo-mcp --project seu-arquivo.prumo\n'
        'A leitura é o padrão. Acrescente --allow-write apenas quando quiser autorizar alterações.\n\n'
        'Cálculo do projeto, dimensionamento e emissão técnica ainda não estão disponíveis. '
        'Leia THIRD_PARTY_NOTICES.md para os direitos dos componentes incorporados.\n'
    )
    with zipfile.ZipFile(destination,'w',zipfile.ZIP_DEFLATED) as package:
        for name,path in files.items(): package.write(path,name)
        package.writestr('COMECAR.md',instructions)
        package.writestr('manifest.json',json.dumps(manifest,ensure_ascii=False,indent=2))
    with tempfile.TemporaryDirectory() as directory:
        with zipfile.ZipFile(destination) as package: package.extractall(directory)
        for name in ('prumo-desktop','prumo-mcp','prumo-component-worker'):
            executable = Path(directory) / (name + suffix)
            executable.chmod(0o755)
            subprocess.run([str(executable),'--help'],cwd=directory,check=True,timeout=15,stdout=subprocess.DEVNULL)
        for name,digest in manifest['files'].items():
            if hashlib.sha256((Path(directory)/name).read_bytes()).hexdigest()!=digest:
                raise SystemExit('Integridade divergente: '+name)
        worker = Path(directory) / ('prumo-component-worker' + suffix)
        report = json.loads(subprocess.check_output(
            [str(worker), '--qualification-report'], cwd=directory, timeout=30))
        validate_report(report, {'sha256': manifest['files'][worker.name],
                                 'bytes': worker.stat().st_size})
    print(destination.name + ': extração, hashes, três executáveis e ensaios de componentes aprovados.')

if __name__ == '__main__': main()
