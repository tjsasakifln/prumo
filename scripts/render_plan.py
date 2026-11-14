#!/usr/bin/env python3
"""Renderiza os textos de issues e o roadmap a partir do registro do plano."""
from pathlib import Path
import argparse
import json

ROOT = Path(__file__).resolve().parents[1]
REPO_URL = 'https://github.com/tjsasakifln/prumo'


def source_index():
    result = {
        'USR-01': {'title': 'Objetivo e requisitos do Prumo definidos pelo proprietário', 'url': REPO_URL + '/blob/main/PRD.md'},
        'USR-02': {'title': 'Diretriz do proprietário: incorporar código aberto pertinente e validado', 'url': REPO_URL + '/blob/main/docs/REUSO.md'},
    }
    for path in sorted((ROOT / 'docs').glob('fontes-*.json')):
        for source in json.loads(path.read_text(encoding='utf-8')):
            if source['id'] in result:
                raise ValueError('Fonte duplicada: ' + source['id'])
            result[source['id']] = source
    return result


def render_issue(issue, sources):
    n = issue['sequence']
    lines = [f"# {issue['key']}: {issue['title']}", '', '## Objetivo', '', issue['goal'], '',
             '## Ordem de implementação', '',
             f"**Etapa {n}/120. Fase {issue['phase']}: {issue['phase_name']}.**", '']
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
    data = json.loads((ROOT / 'planning/backlog.json').read_text(encoding='utf-8'))
    sources = source_index()
    outputs = {}
    for issue in data['issues']:
        outputs[f"planning/issues/{issue['sequence']:03}.md"] = render_issue(issue, sources)
    lines = ['# Prumo: sequência de implementação', '',
             '**120 issues, implementadas em ordem numérica.** Estado inicial: planejamento. A conclusão da última issue exige produto completo, evidências aceitas e distribuição conferida.', '',
             'A fila foi criada antes de qualquer PR para preservar a correspondência entre número GitHub e sequência PRU. Novas PRs não alteram os identificadores abaixo.', '',
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
    for relative, content in rendered_outputs().items():
        path = ROOT / relative
        if args.check:
            if not path.exists() or path.read_text(encoding='utf-8') != content:
                differences.append(relative)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding='utf-8')
    if differences:
        raise SystemExit('Arquivos fora de sincronia: ' + ', '.join(differences))
    print('Roadmap e 120 especificações de issues ' + ('conferidos.' if args.check else 'gerados.'))


if __name__ == '__main__':
    main()
