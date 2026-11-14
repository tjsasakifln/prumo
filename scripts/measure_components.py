#!/usr/bin/env python3
"""Medição administrativa reproduzível da PRU-003, fora do runtime do produto."""
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time

from package_dev import EXPECTED_COMPONENTS, file_identity, native_target, validate_report

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / 'target' / 'qualification-clean-release'
OUTPUT = ROOT / 'docs/evidencias/PRU-003-metricas.json'


def sha256(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def source_snapshot():
    sources = [ROOT / name for name in (
        'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
        'scripts/measure_components.py', 'scripts/package_dev.py',
        'THIRD_PARTY_NOTICES.md', 'docs/DESENVOLVIMENTO.md')]
    for directory in ('apps', 'crates'):
        sources.extend(path for path in (ROOT / directory).rglob('*')
                       if path.is_file())
    for name in ('.cargo/config', '.cargo/config.toml'):
        if (ROOT / name).is_file():
            sources.append(ROOT / name)
    return {path.relative_to(ROOT).as_posix(): sha256(path) for path in sorted(sources)}


def main():
    target_name = native_target()
    # Refuse an existing directory instead of calling an incremental build clean.
    TARGET.mkdir(parents=True, exist_ok=False)
    source_hashes = source_snapshot()
    source_fingerprint = hashlib.sha256(json.dumps(
        source_hashes, sort_keys=True, separators=(',', ':')).encode()).hexdigest()
    command = ['cargo', 'build', '--workspace', '--release', '--locked', '-j', '2']
    environment = dict(os.environ, CARGO_TARGET_DIR=str(TARGET))
    started = time.perf_counter_ns()
    with (TARGET / 'build.log').open('w') as log:
        subprocess.run(command, cwd=ROOT, env=environment, check=True, stdout=log, stderr=log)
    build_ns = time.perf_counter_ns() - started
    suffix = '.exe' if os.name == 'nt' else ''
    binaries = {name: TARGET / 'release' / (name + suffix)
                for name in ('prumo-desktop', 'prumo-mcp', 'prumo-component-worker')}
    binary_identities = {name: file_identity(path) for name, path in binaries.items()}
    if source_snapshot() != source_hashes:
        raise ValueError('Fontes alteradas durante a compilação; medição recusada.')
    reports = []
    request_ids = set()
    for _ in range(5):
        started = time.perf_counter_ns()
        report = json.loads(subprocess.check_output(
            [str(binaries['prumo-component-worker']), '--qualification-report'],
            cwd=ROOT, timeout=30))
        process_ns = time.perf_counter_ns() - started
        validate_report(report, binary_identities['prumo-component-worker'])
        if report['request_id'] in request_ids:
            raise ValueError('A medição repetiu uma requisição em vez de obter nova execução.')
        request_ids.add(report['request_id'])
        reports.append({'process_elapsed_ns': process_ns, 'report': report})
    subprocess.run([sys.executable, str(ROOT / 'scripts/package_dev.py'), '--profile', 'release',
                    '--target-dir', str(TARGET)], cwd=ROOT, env=environment, check=True)
    if {name: file_identity(path) for name, path in binaries.items()} != binary_identities:
        raise ValueError('Binários alterados durante a execução ou o empacotamento; medição recusada.')
    if source_snapshot() != source_hashes:
        raise ValueError('Fontes alteradas durante a medição; evidência recusada.')
    archive = ROOT / 'dist' / ('prumo-development-' + target_name + '-release.zip')
    record = {
        'issue': 'PRU-003', 'measured_at_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
        'profile': 'release', 'engineering_release': False,
        'compiler': subprocess.check_output(['rustc', '--version', '--verbose'], cwd=ROOT, text=True),
        'platform': platform.platform(), 'machine': platform.machine(),
        'cpu_count': os.cpu_count(), 'source_fingerprint_sha256': source_fingerprint,
        'source_files_sha256': source_hashes,
        'clean_build': {'command': command, 'target_initially_empty': True,
                        'dependency_download_cache_reused': True, 'elapsed_ns': build_ns,
                        'environment_overrides': {key: environment[key] for key in (
                            'CARGO_BUILD_TARGET', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
                            'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_INCREMENTAL',
                            'RUSTUP_TOOLCHAIN') if key in environment}},
        'binaries': binary_identities,
        'validation': {'protocol_version': 1, 'suite_id': 'PRU-003.1',
                       'expected_components': EXPECTED_COMPONENTS,
                       'source_snapshot_unchanged': True, 'binary_snapshot_unchanged': True,
                       'numeric_checks_consistent': True},
        'distribution': {'file': archive.name, 'bytes': archive.stat().st_size,
                         'sha256': sha256(archive), 'extracted_three_binaries_started': True,
                         'extracted_component_suite_passed': True},
        'samples': reports,
        'interpretation': [
            'Cinco execuções locais; não constituem comparação entre fornecedores.',
            'Tempos dos componentes incluem bateria, verificações e preparação dos relatórios.',
            'Tempo do processo inclui inicialização, hash do executável e serialização.',
            'Build usa target vazio e cache de downloads existente, com dois jobs.',
            'Extração e execução de --help e do worker usam o mesmo sistema e suas bibliotecas nativas; não simulam outra máquina.',
            'O tamanho é do produto integrado; não é rateado artificialmente por biblioteca.',
            'Perfil release otimiza os executáveis, sem liberar cálculo de engenharia.'
        ],
    }
    OUTPUT.write_text(json.dumps(record, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'output': str(OUTPUT.relative_to(ROOT)), 'build_seconds': build_ns / 1e9,
                      'package_bytes': archive.stat().st_size, 'samples_passed': len(reports)}))


if __name__ == '__main__':
    main()
