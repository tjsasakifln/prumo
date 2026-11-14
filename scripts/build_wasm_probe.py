#!/usr/bin/env python3
"""Administrative regeneration of a byte array from the reviewed Rust fixture."""
import argparse
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'crates/prumo-components/fixtures/wasm_boundary.rs'
OUTPUT = ROOT / 'crates/prumo-components/src/wasm_bytes.rs'

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as directory:
        target = Path(directory) / 'probe.wasm'
        subprocess.run([
            'rustc', str(SOURCE), '--crate-type', 'cdylib', '--edition', '2024',
            '--target', 'wasm32-unknown-unknown', '-C', 'opt-level=3',
            '-C', 'panic=abort', '-C', 'strip=symbols', '-C', 'codegen-units=1',
            '-C', 'metadata=prumo-probe-v1', '-C', 'link-arg=-zstack-size=16384',
            '-C', 'link-arg=--initial-memory=65536',
            '-C', 'link-arg=--max-memory=131072',
            '--remap-path-prefix', str(ROOT) + '=prumo', '-o', str(target)
        ], check=True)
        data = target.read_bytes()
    lines = ['// Generated from fixtures/wasm_boundary.rs by scripts/build_wasm_probe.py.',
             '// Rust 1.99.1, wasm32-unknown-unknown. Do not edit the byte array.',
             'pub const RUST_BOUNDARY_MODULE: &[u8] = &[']
    for start in range(0, len(data), 20):
        lines.append('    ' + ', '.join(str(byte) for byte in data[start:start+20]) + ',')
    lines.append('];')
    content = '\n'.join(lines) + '\n'
    if args.check:
        if OUTPUT.read_text() != content:
            raise SystemExit('Wasm fixture differs from the pinned Rust build.')
        print(f'Wasm fixture verified: {len(data)} bytes.')
    else:
        OUTPUT.write_text(content)
        print(f'Wasm fixture generated: {len(data)} bytes.')

if __name__ == '__main__':
    main()
