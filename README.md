# Prumo

**Estruturas e instalações prediais em um projeto BIM. Modelagem por peças e conexões compreensíveis. Interface e agentes trabalhando sobre o mesmo modelo.**

O Prumo será um sistema profissional brasileiro para conceber, calcular, dimensionar, detalhar, compatibilizar e documentar estruturas e instalações. O objetivo é reduzir o esforço de aprendizagem e de operação, mantendo as hipóteses de engenharia claras, os resultados verificáveis e as entregas coerentes.

**Estado em 14/11/2026: aplicação de desenvolvimento em Rust.** É possível criar, salvar, reabrir e alterar um projeto BIM pela interface e por MCP local, cadastrar referências normativas, fixar sua versão por projeto e conferir impedimentos de emissão. Os componentes de geometria, BIM, seções, álgebra linear e documentos têm ensaios acessíveis pela interface e por MCP. Cálculo, dimensionamento e emissão técnica ainda não estão disponíveis. A fila contém **250 issues sequenciais**; as entregas e evidências estão em [Estado da implementação](docs/IMPLEMENTACAO.md).

## Executar a versão de desenvolvimento

```sh
cargo build --workspace --locked
cargo run -p prumo-desktop --locked
```

A tela permite criar ou abrir um arquivo `.prumo`, escolher a unidade e adicionar peças com identidade física e analítica. Cada alteração é salva localmente. Consulte [instalação, servidor MCP e verificações](docs/DESENVOLVIMENTO.md).

## Implementação em Rust

**O Prumo será implementado em Rust**, incluindo interface, núcleo BIM, regras de engenharia, persistência, serviços e servidor MCP. Essa decisão é obrigatória nas 250 issues. Componentes externos qualificados podem conservar sua linguagem original, com integração própria em Rust e testes de contrato. Os critérios estão em [Implementação em Rust](docs/RUST.md).

## Começar pela próxima issue

A implementação segue **#1 até #250**, em ordem. A primeira entrega já exige criar, salvar, reabrir e consultar um projeto BIM real pela interface e por um cliente MCP. Cada disciplina acrescenta seus próprios comandos, cálculos, desenhos e ensaios. A última entrega libera o produto completo após as verificações técnicas e de operação.

- [Fila de implementação](https://github.com/tjsasakifln/prumo/issues?q=is%3Aissue+is%3Aopen+sort%3Acreated-asc)
- [Issue #1: fundação executável, BIM e MCP](https://github.com/tjsasakifln/prumo/issues/1)
- [Issue #250: lançamento completo](https://github.com/tjsasakifln/prumo/issues/250)
- [Sequência das 250 entregas](ROADMAP.md)
- [Instruções para agentes e implementadores](AGENTS.md)

Cada issue permanece aberta até ter código integrado, critérios atendidos e evidências reproduzíveis. Os marcos intermediários entregam versões de desenvolvimento; a liberação comercial pertence à última etapa.

## O que faz parte do produto

| Área | Abrangência planejada |
|---|---|
| Concreto | Vigas, pilares, lajes, paredes, escadas, armaduras, estabilidade, serviço, protensão, pré-moldados e incêndio |
| Fundações | Sapatas, blocos, radiers, estacas, tubulões, contenções, recalques e interação com o solo |
| Aço e estruturas mistas | Perfis, estabilidade, ligações, bases, chumbadores, fabricação e incêndio |
| Madeira e sistemas complementares | Madeira serrada e engenheirada, ligações, alvenaria estrutural, paredes de concreto e alumínio com base normativa própria |
| Hidrossanitária | Água fria e quente, reservação, bombas, PEX, piscinas, esgoto, ventilação, águas pluviais, aproveitamento de chuva e tratamento local |
| Gás | Redes de baixa e média pressão, consumo, simultaneidade, perdas, reguladores, medição e centrais |
| Incêndio | Hidrantes, mangotinhos, reserva técnica, bombas, sprinklers e representação dos sistemas preventivos, com limites de cálculo explícitos |
| Elétrica | Pontos, circuitos, quadros, demanda, condutores, proteções, eletrodutos, barramento blindado, geração e documentos |
| Fotovoltaica | Módulos, séries, arranjos, inversores, circuitos CC/CA, baterias, geração estimada e análise econômica |
| SPDA | Avaliação de risco, captação, descidas, proteção geométrica, aterramento e equipotencialização |
| Cabeamento | Redes metálicas e ópticas, dados, telefonia, TV, interfone, CFTV, rotas, racks, portas e mapas de cabos |
| Climatização | Ambientes, carga térmica, equipamentos, linhas frigorígenas, dutos, água gelada, drenagem e documentação |
| Coordenação e entrega | Referências BIM/CAD, interferências, passagens, desenhos, memoriais, listas de materiais e revisões sincronizadas |
| Agentes e experiência | MCP desde a primeira entrega, operações rastreáveis, montagem assistida e uso por uma mão ou teclado |

Os [requisitos do produto](PRD.md), a [cobertura funcional](docs/PARIDADE.md) e o [escopo de instalações](docs/INSTALACOES.md) distinguem modelagem, cálculo, verificação e documentação. Uma representação geométrica não implica dimensionamento concluído.

## Normas e confiabilidade

O produto abrange NBR 6118, NBR 8800, série NBR 7190 e correlatas estruturais, além das famílias normativas aplicáveis às instalações, como NBR 5626, NBR 8160, NBR 10844, NBR 15526, NBR 13714, NBR 10897, NBR 5410, série NBR 5419, NBR 16690, NBR 14565 e normas de climatização. A edição e o domínio de cada regra ficam no [cadastro normativo](docs/NORMAS.md).

Cada projeto preservará as versões, hipóteses e dados utilizados. A qualificação combina referências independentes, conservação, convergência, casos de fronteira, projetos completos e revisão profissional. Alterações técnicas invalidam os resultados e documentos afetados.

## Reutilização de código aberto

**Avaliar reuso é obrigatório em toda issue pertinente.** A adoção de um componente exige procedência, licença compatível, versão fixada, manutenção, testes próprios e domínio de aplicação demonstrado. O projeto prioriza bibliotecas adequadas para interoperabilidade, geometria, visualização, redes, cálculo e documentação. A [política de reuso](docs/REUSO.md) registra os critérios e candidatos pesquisados.

## Documentação

| Documento | Finalidade |
|---|---|
| [PRD](PRD.md) | Objetivo, requisitos e critérios de sucesso |
| [Roadmap](ROADMAP.md) | Ordem de implementação das 250 entregas |
| [Cobertura funcional](docs/PARIDADE.md) | Inventário, fontes e limites do escopo |
| [Instalações](docs/INSTALACOES.md) | Disciplinas, funções e integração com estruturas |
| [Normas](docs/NORMAS.md) | Edições, acesso e interpretação das regras |
| [Arquitetura](docs/ARQUITETURA.md) | Modelo compartilhado, persistência e motores |
| [Rust](docs/RUST.md) | Linguagem obrigatória, compilação, interface, MCP e integrações |
| [MCP](docs/MCP.md) | Comandos, permissões e operação por agentes |
| [Experiência de uso](docs/UX.md) | Montagem intuitiva, acessibilidade e métricas |
| [Reuso](docs/REUSO.md) | Seleção e qualificação de componentes |
| [Validação](docs/VALIDACAO.md) | Ensaios, tolerâncias, evidências e revisão |
| [Lançamento](docs/LANCAMENTO.md) | Condições obrigatórias para distribuição |
| [Revisão do planejamento](docs/REVISAO.md) | Conferências e correções do plano |

## Verificação do planejamento

Execute `python3 scripts/validate_plan.py` para conferir sequência, dependências, cobertura, fontes, textos das issues, política Rust, identidade da apresentação e registro publicado. Os scripts Python de manutenção do planejamento ficam fora do runtime do produto. Esse verificador avalia a integridade do plano; os cálculos de engenharia terão suas próprias campanhas de qualificação.

## Direitos e procedência

O desenvolvimento preservará a autoria, a licença e os avisos dos componentes incorporados. As condições comerciais do Prumo e os direitos de distribuição serão definidos e conferidos antes do lançamento. Textos normativos licenciados sem autorização de redistribuição não integram o repositório.
