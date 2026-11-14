#!/usr/bin/env python3
"""Confere o planejamento, sem atestar código Rust implementado ou cálculos."""
from pathlib import Path
import argparse
import hashlib
import json
import re
import sys
from urllib.parse import unquote, urlsplit

from render_plan import (TECHNOLOGY_POLICY_ID, TECHNOLOGY_REQUIREMENT_ID,
                         read_json, rendered_outputs, source_index, technology_policy,
                         text_list)

ROOT = Path(__file__).resolve().parents[1]


def object_list(data, field, context):
    values = data.get(field)
    if not isinstance(values, list) or any(not isinstance(value, dict) for value in values):
        raise ValueError(f'{context}: {field} deve ser uma lista de objetos JSON.')
    return values


def text_field(data, field, context):
    value = data.get(field)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f'{context}: {field} deve ser um texto não vazio.')
    return value


def validate(allow_unpublished=False):
    errors = []
    data = read_json('planning/backlog.json')
    issues = object_list(data, 'issues', 'planning/backlog.json')
    policy = technology_policy()
    source_ids = set(source_index())
    missing_policy_sources = set(policy['source_ids']) - source_ids
    if missing_policy_sources:
        errors.append('Política Rust com fontes inexistentes: ' + ', '.join(sorted(missing_policy_sources)) + '.')
    keys = {text_field(issue, 'key', f'Issue na posição {n}') for n, issue in enumerate(issues, 1)}
    if len(issues) != 250 or type(data.get('issue_count')) is not int or data['issue_count'] != 250 or len(keys) != 250:
        errors.append('A fila inicial precisa conter 250 issues únicas.')
    if data.get('release_issue') != 'PRU-250':
        errors.append('A liberação comercial deve pertencer à PRU-250.')
    for n, issue in enumerate(issues, 1):
        key = f'PRU-{n:03}'
        if type(issue.get('sequence')) is not int or issue['sequence'] != n or issue['key'] != key:
            errors.append(f'Sequência inconsistente em {key}.')
        expected = [] if n == 1 else [f'PRU-{n-1:03}']
        if issue.get('depends_on') != expected:
            errors.append(f'Dependência incorreta em {key}.')
        if issue.get('implementation_language') != 'Rust':
            errors.append(f'{key}: implementation_language deve ser Rust, conforme {TECHNOLOGY_REQUIREMENT_ID}.')
        if issue.get('technology_policy') != TECHNOLOGY_POLICY_ID:
            errors.append(f'{key}: technology_policy deve ser {TECHNOLOGY_POLICY_ID}.')
        for field in ('title', 'goal', 'phase', 'phase_name'):
            text_field(issue, field, key)
        for field in ('scope', 'acceptance', 'validation'):
            values = issue.get(field)
            if not isinstance(values, list) or len(values) < 3 or any(not isinstance(value, str) or not value.strip() for value in values):
                errors.append(f'{key} incompleta em {field}.')
        for field in ('norms', 'reuse_candidates'):
            values = issue.get(field)
            if not isinstance(values, list) or any(not isinstance(value, str) or not value.strip() for value in values):
                errors.append(f'{key} com {field} fora do formato de lista de textos.')
        issue_sources = text_list(issue.get('source_ids'), f'{key}: source_ids', allow_empty=False)
        if set(issue_sources) - source_ids:
            errors.append(f'{key} com fonte inválida: {set(issue_sources) - source_ids}.')
        if len(issue['title']) > 230:
            errors.append(f'Título excessivo em {key}.')
        if not allow_unpublished:
            if type(issue.get('github_number')) is not int or issue['github_number'] != n or issue.get('github_url') != f'https://github.com/tjsasakifln/prumo/issues/{n}':
                errors.append(f'{key} sem correspondência publicada válida.')
    phase_numbers = []
    for phase in object_list(data, 'phases', 'planning/backlog.json'):
        phase_id = text_field(phase, 'id', 'Fase')
        text_field(phase, 'name', f'Fase {phase_id}')
        if any(type(phase.get(field)) is not int for field in ('first', 'last')):
            raise ValueError(f'Fase {phase_id}: first e last devem ser inteiros.')
        if not 1 <= phase['first'] <= phase['last'] <= 250:
            errors.append(f'Fase {phase_id} tem intervalo inválido.')
            continue
        phase_numbers.extend(range(phase['first'], phase['last'] + 1))
        for issue in issues[phase['first']-1:phase['last']]:
            if issue['phase'] != phase['id'] or issue['phase_name'] != phase['name']:
                errors.append(f'Fase inconsistente: {issue["key"]}.')
    if phase_numbers != list(range(1, 251)):
        errors.append('Fases não cobrem exatamente a fila.')
    coverage = read_json('planning/coverage.json')
    capabilities = object_list(coverage, 'capabilities', 'planning/coverage.json')
    if len(capabilities) < 72:
        errors.append('Cobertura inferior ao inventário ampliado de 72 famílias.')
    capability_ids = [text_field(c, 'capability_id', 'Capacidade') for c in capabilities]
    if len(set(capability_ids)) != len(capability_ids):
        errors.append('Capacidade duplicada.')
    required_capability_ids = {f'PAR-{n:02}' for n in range(1, 37)} | {f'MEP-{n:02}' for n in range(1, 37)}
    missing_capability_ids = required_capability_ids - set(capability_ids)
    if missing_capability_ids:
        errors.append('Capacidades obrigatórias ausentes: ' + ', '.join(sorted(missing_capability_ids)) + '.')
    for cap in capabilities:
        cid = cap['capability_id']
        for field in ('implementation_issues', 'qualification_issues'):
            issue_map = text_list(cap.get(field), f'{cid}: {field}', allow_empty=False)
            if set(issue_map) - keys:
                errors.append(f'{cid} com mapa inválido em {field}.')
        if cap.get('release_gate') != 'PRU-250' or cap.get('required') is not True:
            errors.append(f'{cid} perdeu condição de lançamento.')
        capability_sources = text_list(cap.get('source_ids'), f'{cid}: source_ids', allow_empty=False)
        if set(capability_sources) - source_ids:
            errors.append(f'{cid} usa fonte ausente ou inexistente.')
    requirements = read_json('planning/requirements.json')
    requirement_records = object_list(requirements, 'requirements', 'planning/requirements.json')
    if len(requirement_records) != 15:
        errors.append('O plano precisa conter 15 requisitos permanentes.')
    requirement_ids = [text_field(req, 'id', 'Requisito permanente') for req in requirement_records]
    if len(set(requirement_ids)) != len(requirement_ids):
        errors.append('Requisito permanente duplicado.')
    required_requirement_ids = {f'REQ-{n:02}' for n in range(1, 16)}
    if set(requirement_ids) != required_requirement_ids:
        errors.append('Os requisitos permanentes devem preservar REQ-01 a REQ-15.')
    for req in requirement_records:
        issue_map = text_list(req.get('issues'), f'{req["id"]}: issues', allow_empty=False)
        if set(issue_map) - keys:
            errors.append(f'{req["id"]} sem cobertura válida.')
        if req.get('required') is not True:
            errors.append(f'{req["id"]} perdeu condição obrigatória.')
        if req['id'] == TECHNOLOGY_REQUIREMENT_ID and (len(issue_map) != 250 or set(issue_map) != {f'PRU-{n:03}' for n in range(1, 251)}):
            errors.append(f'{TECHNOLOGY_REQUIREMENT_ID} deve abranger todas as 250 issues, uma vez cada.')
    for filename in ('README.md', 'LEIAME.md'):
        presentation = (ROOT / filename).read_text(encoding='utf-8')
        if re.search(r'\b(?:altoqi|qi[\s_-]*builder|builder|eberick|cype(?:cad|[\s_-]*3d)?|tqs)\b', unquote(presentation), re.IGNORECASE):
            errors.append('Nome de software de referência na apresentação: ' + filename)
    try:
        expected_outputs = rendered_outputs()
    except (KeyError, TypeError, ValueError, OSError, UnicodeError) as error:
        errors.append('Dados não renderizáveis: ' + str(error))
        expected_outputs = {}
    for relative, expected in expected_outputs.items():
        path = ROOT / relative
        if not path.exists() or path.read_text(encoding='utf-8') != expected:
            errors.append('Renderização desatualizada: ' + relative)
    local_issues = sorted((ROOT / 'planning/issues').glob('*.md'))
    if len(local_issues) != 250:
        errors.append('Quantidade de textos de issues diferente de 250.')
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
            manifest = read_json('planning/github-issues.json')
            records = object_list(manifest, 'issues', 'planning/github-issues.json')
            if len(records) != 250:
                errors.append('Registro de publicação incompleto.')
            if manifest.get('verified_count') != len(issues):
                errors.append('Contagem de publicação conferida divergente.')
            for n, record in enumerate(records, 1):
                if n > len(issues):
                    errors.append(f'Registro publicado sem issue no plano: {n}.')
                    continue
                if type(record.get('number')) is not int or record['number'] != n or record.get('key') != f'PRU-{n:03}':
                    errors.append(f'Identidade publicada incorreta: {n}.')
                if record.get('api_readback_verified') is not True or record.get('url') != f'https://github.com/tjsasakifln/prumo/issues/{n}':
                    errors.append(f'Publicação sem conferência ou URL válida: {n}.')
                content = expected_outputs.get(f'planning/issues/{n:03}.md', '')
                digest = hashlib.sha256(content.encode()).hexdigest()
                if record.get('body_sha256') != digest:
                    errors.append(f'Texto publicado fora de sincronia: {n}.')
                expected_title = f"[{issues[n-1]['key']}] {issues[n-1]['title']}"
                if record.get('title') != expected_title:
                    errors.append(f'Título publicado fora de sincronia: {n}.')
    return errors, {'issues': len(issues), 'capabilities': len(capabilities), 'requirements': len(requirement_records), 'sources': len(source_ids), 'technology_policy': TECHNOLOGY_POLICY_ID, 'first_party_language': 'Rust'}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--allow-unpublished', action='store_true')
    args = parser.parse_args()
    try:
        errors, stats = validate(args.allow_unpublished)
    except (KeyError, TypeError, ValueError, OSError, UnicodeError) as error:
        print('ERRO: dados do planejamento inválidos: ' + str(error), file=sys.stderr)
        raise SystemExit(1) from None
    if errors:
        for error in errors:
            print('ERRO: ' + error, file=sys.stderr)
        raise SystemExit(1)
    print(json.dumps({'status': 'ok', 'scope': 'planning_integrity_only', **stats}, ensure_ascii=False))


if __name__ == '__main__':
    main()
