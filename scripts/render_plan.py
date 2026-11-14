#!/usr/bin/env python3
"""Renderiza os textos de issues e o roadmap a partir do registro do plano."""
from pathlib import Path
import argparse
import json

ROOT = Path(__file__).resolve().parents[1]
REPO_URL = 'https://github.com/tjsasakifln/prumo'
TECHNOLOGY_POLICY_ID = 'TEC-RUST-01'
TECHNOLOGY_REQUIREMENT_ID = 'REQ-15'


def read_json(relative, expected_type=dict):
    """Lê dados do plano com diagnóstico de arquivo e formato, sem traceback."""
    path = ROOT / relative
    try:
        data = json.loads(path.read_text(encoding='utf-8'))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError(f'Não foi possível ler {relative}: {error}') from error
    if not isinstance(data, expected_type):
        kind = 'um objeto JSON' if expected_type is dict else 'uma lista JSON'
        raise ValueError(f'{relative} deve conter {kind}.')
    return data


def text_list(values, context, allow_empty=True):
    if (not isinstance(values, list) or (not allow_empty and not values)
            or any(not isinstance(value, str) or not value.strip() for value in values)):
        raise ValueError(f'{context} deve ser uma lista de textos não vazios.')
    return values


def technology_policy():
    """Confere a decisão de planejamento; não examina código do produto."""
    policy = read_json('planning/technology.json')
    expected = {
        'policy_id': TECHNOLOGY_POLICY_ID,
        'requirement_id': TECHNOLOGY_REQUIREMENT_ID,
        'status': 'accepted_by_owner',
        'first_party_language': 'Rust',
        'ui_authored_in_rust': True,
        'cargo_workspace_required': True,
        'rust_toolchain_pinned_required': True,
        'release_issue': 'PRU-250',
        'planning_tools_may_use_other_languages': True,
    }
    for field, value in expected.items():
        if type(policy.get(field)) is not type(value) or policy[field] != value:
            raise ValueError(f'planning/technology.json: {field} deve ser {json.dumps(value, ensure_ascii=False)}.')
    external = policy.get('external_components')
    if not isinstance(external, dict):
        raise ValueError('planning/technology.json: external_components deve ser um objeto JSON.')
    for field, value in {'allowed': True, 'new_non_rust_business_logic_allowed': False}.items():
        if external.get(field) is not value:
            raise ValueError(f'planning/technology.json: external_components.{field} deve ser {json.dumps(value)}.')
    source_ids = text_list(policy.get('source_ids'), 'planning/technology.json: source_ids', allow_empty=False)
    if len(set(source_ids)) != len(source_ids):
        raise ValueError('planning/technology.json: source_ids contém fontes duplicadas.')
    if 'USR-05' not in source_ids or not any(source_id.startswith('RS-') for source_id in source_ids):
        raise ValueError('planning/technology.json: source_ids deve incluir USR-05 e fontes técnicas RS-.')
    return policy


def source_index():
    requirements = read_json('planning/requirements.json')
    result = {}
    owner_sources = requirements.get('sources')
    if not isinstance(owner_sources, list):
        raise ValueError('planning/requirements.json: sources deve ser uma lista.')
    for source in owner_sources:
        if not isinstance(source, dict) or not isinstance(source.get('id'), str) or not source['id'].strip():
            raise ValueError('planning/requirements.json: cada fonte deve ter id textual não vazio.')
        if source['id'] in result:
            raise ValueError('Fonte do proprietário duplicada: ' + source['id'])
        title = source.get('title') or source.get('description')
        if not isinstance(title, str) or not title.strip():
            raise ValueError('Fonte do proprietário sem título ou descrição: ' + source['id'])
        result[source['id']] = {
            'title': title,
            'url': source.get('url', REPO_URL + '/blob/main/PRD.md'),
        }
    for path in sorted((ROOT / 'docs').glob('fontes-*.json')):
        for source in read_json(str(path.relative_to(ROOT)), list):
            if not isinstance(source, dict) or not isinstance(source.get('id'), str) or not source['id'].strip():
                raise ValueError(f'{path.name}: cada fonte deve ter id textual não vazio.')
            if source['id'] in result:
                raise ValueError('Fonte duplicada: ' + source['id'])
            result[source['id']] = source
    for source_id, source in result.items():
        if any(not isinstance(source.get(field), str) or not source[field].strip() for field in ('title', 'url')):
            raise ValueError(f'Fonte {source_id} sem título ou URL textual válido.')
    return result


def render_issue(issue, sources, total):
    if not isinstance(issue, dict):
        raise ValueError('planning/backlog.json: cada issue deve ser um objeto JSON.')
    key = issue.get('key', 'Issue')
    for field in ('key', 'title', 'goal', 'phase', 'phase_name'):
        if not isinstance(issue.get(field), str) or not issue[field].strip():
            raise ValueError(f'{key}: {field} deve ser um texto não vazio.')
    if type(issue.get('sequence')) is not int:
        raise ValueError(f'{key}: sequence deve ser um inteiro.')
    if issue.get('implementation_language') != 'Rust' or issue.get('technology_policy') != TECHNOLOGY_POLICY_ID:
        raise ValueError(f"{issue.get('key', 'Issue')}: implementação própria deve ser Rust e technology_policy deve ser {TECHNOLOGY_POLICY_ID}.")
    for field in ('scope', 'acceptance', 'validation', 'norms', 'reuse_candidates', 'source_ids'):
        text_list(issue.get(field), f"{issue['key']}: {field}")
    n = issue['sequence']
    lines = [f"# {issue['key']}: {issue['title']}", '', '## Objetivo', '', issue['goal'], '',
             '## Ordem de implementação', '',
             f"**Etapa {n}/{total}. Fase {issue['phase']}: {issue['phase_name']}.**", '']
    if n == 1:
        lines.append('Primeira issue da sequência. Consultar o planejamento e confirmar a base atual do repositório antes de iniciar.')
    else:
        lines.append(f'**Depende de #{n-1}.** Iniciar após a integração e validação de todas as etapas anteriores. Fechamento por cancelamento ou mera declaração não satisfaz a dependência.')
    lines += ['', f"Regras: [AGENTS.md]({REPO_URL}/blob/main/AGENTS.md), [PRD]({REPO_URL}/blob/main/PRD.md) e [critério de conclusão]({REPO_URL}/blob/main/docs/CONCLUSAO-ISSUE.md).", '']
    for heading, field, checkboxes in [('Escopo', 'scope', False), ('Critérios de aceite', 'acceptance', True), ('Verificação e evidências', 'validation', True)]:
        lines += ['## ' + heading, '']
        lines += [('- [ ] ' if checkboxes else '- ') + text for text in issue[field]]
        lines.append('')
    lines += ['## Reuso de componentes abertos', '']
    if issue['reuse_candidates']:
        lines += ['**Candidatos a avaliar:** ' + '; '.join(issue['reuse_candidates']) + '.', '']
    else:
        lines += ['Pesquisar componentes pertinentes ao módulo antes de implementar rotinas equivalentes do zero.', '']
    lines += ['- [ ] Registrar adoção, adaptação ou rejeição justificada, com origem, licença, versão, manutenção e testes próprios.',
              '- [ ] Conferir as obrigações dos arquivos e dependências efetivamente incorporados. Candidato listado não significa homologação; nomes sem ficha documental permanecem com pesquisa pendente.',
              f'- [ ] Seguir [REUSO.md]({REPO_URL}/blob/main/docs/REUSO.md), mantendo uma referência independente do motor utilizado.', '',
              '## Contratos transversais', '',
              f'- [ ] {TECHNOLOGY_REQUIREMENT_ID} e {TECHNOLOGY_POLICY_ID}: implementar em Rust todo código próprio do produto, incluindo interface, BIM, regras de engenharia, motores, CLI e MCP. Usar workspace Cargo e toolchain Rust fixada, conforme [RUST.md]({REPO_URL}/blob/main/docs/RUST.md).',
              f'- [ ] Componentes externos em outras linguagens exigem integração e qualificação conforme [RUST.md]({REPO_URL}/blob/main/docs/RUST.md) e a política de reuso. A integração não autoriza nova lógica própria de produto em outra linguagem. Ferramentas de manutenção do planejamento podem usar outras linguagens.',
              '- [ ] Interface e MCP usam os mesmos comandos e validações; novas funções relevantes ampliam ambos, inclusive após a etapa 99.',
              '- [ ] Unidades, identidades, autoria, revisão e estados de resultado permanecem rastreáveis. Mudança relevante invalida evidências dependentes.',
              '- [ ] Código integrado, revisão independente e evidências reproduzíveis constam da entrega. Nenhuma pendência obrigatória é retirada para encerrar a issue.', '']
    if issue['norms']:
        lines += ['## Referências normativas a aplicar', '', '; '.join(issue['norms']) + '.', '',
                  f'Confirmar edição, emendas, acesso autorizado e domínio no [registro normativo]({REPO_URL}/blob/main/docs/NORMAS.md). Metadados públicos não substituem o texto técnico licenciado.', '']
    lines += ['## Fontes e documentação', '']
    for source_id in issue['source_ids']:
        source = sources[source_id]
        lines.append(f"- [{source_id}: {source['title']}]({source['url']})")
    lines += [f'- [Plano de validação]({REPO_URL}/blob/main/docs/VALIDACAO.md)',
              f'- [Condições de lançamento]({REPO_URL}/blob/main/docs/LANCAMENTO.md)', '',
              '**Estado inicial:** planejado. Esta issue registra trabalho a implementar e comprovar; não declara capacidade já disponível.', '']
    return '\n'.join(lines)


def rendered_outputs():
    data = read_json('planning/backlog.json')
    for field in ('issues', 'phases'):
        values = data.get(field)
        if not isinstance(values, list) or any(not isinstance(value, dict) for value in values):
            raise ValueError(f'planning/backlog.json: {field} deve ser uma lista de objetos JSON.')
    if type(data.get('issue_count')) is not int:
        raise ValueError('planning/backlog.json: issue_count deve ser um inteiro.')
    policy = technology_policy()
    sources = source_index()
    missing_sources = set(policy['source_ids']) - set(sources)
    if missing_sources:
        raise ValueError('planning/technology.json: fontes inexistentes: ' + ', '.join(sorted(missing_sources)) + '.')
    outputs = {}
    for issue in data['issues']:
        outputs[f"planning/issues/{issue['sequence']:03}.md"] = render_issue(issue, sources, data['issue_count'])
    lines = ['# Prumo: sequência de implementação', '',
             f"**{data['issue_count']} issues, implementadas em ordem numérica.** Estado inicial: planejamento. A conclusão da última issue exige estruturas e instalações completas, evidências aceitas e distribuição conferida.", '',
             'A fila ampliada preserva a correspondência entre número GitHub e sequência PRU. A etapa 120 é um marco estrutural; a liberação comercial pertence à etapa 250. Novas PRs não alteram os identificadores abaixo.', '',
             f'**Decisão de linguagem: Rust.** O requisito {TECHNOLOGY_REQUIREMENT_ID} e a política {TECHNOLOGY_POLICY_ID} abrangem as 250 entregas. Todo código próprio do produto, incluindo interface, BIM, regras de engenharia, motores, CLI e MCP, será escrito em Rust, com workspace Cargo e toolchain fixada. Integrações de componentes externos e ferramentas do planejamento seguem [RUST.md](docs/RUST.md) e [technology.json](planning/technology.json). Esta decisão registra uma exigência de implementação, sem declarar código já concluído.', '',
             '## Fases', '', '| Issues | Fase |', '|---|---|']
    for phase in data['phases']:
        lines.append(f"| #{phase['first']} a #{phase['last']} | {phase['name']} |")
    lines += ['', '## Como avançar', '',
              'Selecionar a menor issue da sequência ainda não concluída, confirmar as anteriores na branch principal e executar seus critérios completos. Uma issue pode precisar de várias PRs. A quantidade de issues não é uma estimativa de prazo.', '',
              'As etapas intermediárias entregam capacidades de desenvolvimento. Estado analisado, verificado por domínio e liberado para emissão são distintos. O lançamento depende de [LANCAMENTO.md](docs/LANCAMENTO.md) e da matriz de evidências.', '',
              'A [matriz de cobertura](planning/coverage.json) vincula as capacidades pesquisadas às implementações e campanhas de qualificação. Os dados estruturados da fila estão em [backlog.json](planning/backlog.json).', '']
    for phase in data['phases']:
        lines += [f"## Fase {phase['id']}: {phase['name']}", '', '| Issue | Entrega | Especificação local |', '|---|---|---|']
        for issue in data['issues']:
            if issue['phase'] == phase['id']:
                n = issue['sequence']
                url = issue['github_url'] or f'{REPO_URL}/issues/{n}'
                lines.append(f"| [#{n}]({url}) | {issue['title']} | [{issue['key']}](planning/issues/{n:03}.md) |")
        lines.append('')
    outputs['ROADMAP.md'] = '\n'.join(lines)
    return outputs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    differences = []
    try:
        for relative, content in rendered_outputs().items():
            path = ROOT / relative
            if args.check:
                if not path.exists() or path.read_text(encoding='utf-8') != content:
                    differences.append(relative)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding='utf-8')
    except (KeyError, TypeError, ValueError, OSError, UnicodeError) as error:
        raise SystemExit('ERRO: dados do planejamento não renderizáveis: ' + str(error)) from None
    if differences:
        raise SystemExit('Arquivos fora de sincronia: ' + ', '.join(differences))
    print('Roadmap e especificações de issues ' + ('conferidos.' if args.check else 'gerados.'))


if __name__ == '__main__':
    main()
