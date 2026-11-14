#!/usr/bin/env python3
"""Confere integridade do planejamento. Não verifica cálculos estruturais."""
from pathlib import Path
import argparse
import hashlib
import json
import re
import sys
from urllib.parse import unquote, urlsplit

from render_plan import rendered_outputs, source_index

ROOT = Path(__file__).resolve().parents[1]


def validate(allow_unpublished=False):
    errors = []
    data = json.loads((ROOT / 'planning/backlog.json').read_text(encoding='utf-8'))
    issues = data['issues']
    source_ids = set(source_index())
    keys = {issue['key'] for issue in issues}
    if len(issues) != 120 or data['issue_count'] != 120 or len(keys) != 120:
        errors.append('A fila inicial precisa conter 120 issues únicas.')
    for n, issue in enumerate(issues, 1):
        key = f'PRU-{n:03}'
        if issue['sequence'] != n or issue['key'] != key:
            errors.append(f'Sequência inconsistente em {key}.')
        expected = [] if n == 1 else [f'PRU-{n-1:03}']
        if issue['depends_on'] != expected:
            errors.append(f'Dependência incorreta em {key}.')
        for field in ('title', 'goal', 'phase', 'phase_name'):
            if not issue.get(field):
                errors.append(f'{key} sem {field}.')
        for field in ('scope', 'acceptance', 'validation'):
            if len(issue.get(field, [])) < 3:
                errors.append(f'{key} incompleta em {field}.')
        if not issue['source_ids'] or set(issue['source_ids']) - source_ids:
            errors.append(f'{key} com fonte ausente ou inválida: {set(issue["source_ids"]) - source_ids}.')
        if len(issue['title']) > 230:
            errors.append(f'Título excessivo em {key}.')
        if not allow_unpublished:
            if issue.get('github_number') != n or issue.get('github_url') != f'https://github.com/tjsasakifln/prumo/issues/{n}':
                errors.append(f'{key} sem correspondência publicada válida.')
    phase_numbers = []
    for phase in data['phases']:
        phase_numbers.extend(range(phase['first'], phase['last'] + 1))
        for issue in issues[phase['first']-1:phase['last']]:
            if issue['phase'] != phase['id'] or issue['phase_name'] != phase['name']:
                errors.append(f'Fase inconsistente: {issue["key"]}.')
    if phase_numbers != list(range(1, 121)):
        errors.append('Fases não cobrem exatamente a fila.')
    coverage = json.loads((ROOT / 'planning/coverage.json').read_text(encoding='utf-8'))
    capabilities = coverage['capabilities']
    if len(capabilities) < 36:
        errors.append('Cobertura inferior ao inventário inicial de 36 famílias.')
    capability_ids = [c['capability_id'] for c in capabilities]
    if len(set(capability_ids)) != len(capability_ids):
        errors.append('Capacidade duplicada.')
    required_capability_ids = {f'PAR-{n:02}' for n in range(1, 37)}
    missing_capability_ids = required_capability_ids - set(capability_ids)
    if missing_capability_ids:
        errors.append('Capacidades obrigatórias ausentes: ' + ', '.join(sorted(missing_capability_ids)) + '.')
    for cap in capabilities:
        cid = cap['capability_id']
        for field in ('implementation_issues', 'qualification_issues'):
            if not cap[field] or set(cap[field]) - keys:
                errors.append(f'{cid} com mapa inválido em {field}.')
        if cap['release_gate'] != 'PRU-120' or cap['required'] is not True:
            errors.append(f'{cid} perdeu condição de lançamento.')
        if not cap['source_ids'] or set(cap['source_ids']) - source_ids:
            errors.append(f'{cid} usa fonte ausente ou inexistente.')
    requirements = json.loads((ROOT / 'planning/requirements.json').read_text(encoding='utf-8'))
    if len(requirements['requirements']) != 12:
        errors.append('Requisitos permanentes incompletos.')
    requirement_ids = [req['id'] for req in requirements['requirements']]
    if len(set(requirement_ids)) != len(requirement_ids):
        errors.append('Requisito permanente duplicado.')
    required_requirement_ids = {f'REQ-{n:02}' for n in range(1, 13)}
    if set(requirement_ids) != required_requirement_ids:
        errors.append('Os requisitos permanentes devem preservar REQ-01 a REQ-12.')
    for req in requirements['requirements']:
        if not req['issues'] or set(req['issues']) - keys:
            errors.append(f'{req["id"]} sem cobertura válida.')
        if req.get('required') is not True:
            errors.append(f'{req["id"]} perdeu condição obrigatória.')
    expected_outputs = rendered_outputs()
    for relative, expected in expected_outputs.items():
        path = ROOT / relative
        if not path.exists() or path.read_text(encoding='utf-8') != expected:
            errors.append('Renderização desatualizada: ' + relative)
    local_issues = sorted((ROOT / 'planning/issues').glob('*.md'))
    if len(local_issues) != 120:
        errors.append('Quantidade de textos de issues diferente de 120.')
    for path in sorted(ROOT.rglob('*.md')):
        if '.git' in path.parts:
            continue
        text = path.read_text(encoding='utf-8')
        if '\u2014' in text:
            errors.append('Travessão não permitido: ' + str(path.relative_to(ROOT)))
        for match in re.finditer(r'\[[^\]]*\]\(([^)]+)\)', text):
            target = match.group(1).split(' "')[0]
            parsed = urlsplit(target)
            if parsed.scheme or target.startswith('#'):
                continue
            local = (path.parent / unquote(parsed.path)).resolve()
            if not local.exists():
                errors.append(f'Link local quebrado: {path.relative_to(ROOT)} -> {target}')
    manifest_path = ROOT / 'planning/github-issues.json'
    if not allow_unpublished:
        if not manifest_path.exists():
            errors.append('Registro de publicação ausente.')
        else:
            manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
            records = manifest['issues']
            if len(records) != 120:
                errors.append('Registro de publicação incompleto.')
            for n, record in enumerate(records, 1):
                if record['number'] != n or record['key'] != f'PRU-{n:03}':
                    errors.append(f'Identidade publicada incorreta: {n}.')
                content = expected_outputs.get(f'planning/issues/{n:03}.md', '')
                digest = hashlib.sha256(content.encode()).hexdigest()
                if record['body_sha256'] != digest:
                    errors.append(f'Texto publicado fora de sincronia: {n}.')
                expected_title = f"[{issues[n-1]['key']}] {issues[n-1]['title']}"
                if record['title'] != expected_title:
                    errors.append(f'Título publicado fora de sincronia: {n}.')
    return errors, {'issues': len(issues), 'capabilities': len(capabilities), 'requirements': len(requirements['requirements']), 'sources': len(source_ids)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--allow-unpublished', action='store_true')
    args = parser.parse_args()
    errors, stats = validate(args.allow_unpublished)
    if errors:
        for error in errors:
            print('ERRO: ' + error, file=sys.stderr)
        raise SystemExit(1)
    print(json.dumps({'status': 'ok', 'scope': 'planning_integrity_only', **stats}, ensure_ascii=False))


if __name__ == '__main__':
    main()
