# PRU-004: interface e MCP do modelo semântico

Data dos ensaios locais: 14/11/2026. Esta evidência descreve o adaptador, os formulários e os testes executados. A matriz de integração, a inspeção visual nativa e a revisão final da issue são consolidadas em [PRU-004.md](PRU-004.md).

## Comportamento implementado

A interface oferece formulários para entidades físicas, nós e barras analíticas, materiais, forças e momentos, cópia, fusão, divisão, transformações, referências de coordenadas, procedência, redes, portas e conexões. Os seletores mostram nomes e identificadores das entidades existentes. O fluxo principal não exige escrever JSON ou digitar UUIDs.

O MCP anuncia `prumo.model.edit`, com `expected_revision` e uma variante fechada de `ModelEdit`. As 25 variantes usam o mesmo enum e passam pelo mesmo `ProjectService::execute` da interface. O adaptador não implementa regras próprias de material, geometria, conexão ou invalidação. O catálogo declara `oneOf` e `additionalProperties: false` nos objetos aninhados; a desserialização do domínio também rejeita campos desconhecidos.

| Área da interface | Variantes de `ModelEdit` disponíveis |
| --- | --- |
| Peças | `add_physical`, `set_physical_geometry`, `set_physical_role`, `set_physical_material` |
| Analítico | `add_analytical_node`, `add_analytical_member`, `link_physical_analytical`, `unlink_physical_analytical` |
| Materiais | `add_material`, `set_material` |
| Ações | `add_nodal_action`, `set_nodal_action` |
| Cópia e transformação | `copy_physical`, `merge_physical`, `split_physical`, `rigid_move` |
| Redes e portas | `add_network`, `add_port`, `assign_port`, `connect_ports`, `disconnect_ports` |
| Origem e unidades | `set_display_unit`, `change_engineering_frame`, `set_georeference`, `set_source_origin` |

A interface também oferece **Atualizar medidas do modelo**, que emite `Command::InspectModel`. O MCP oferece a operação equivalente em `prumo.model.inspect`. Trata-se de escrita porque persiste comprimentos e resumo geométrico vinculados às respectivas bases e dependências. Nenhuma medida recebe aprovação de engenharia.

As consultas `prumo.model.snapshot` e `prumo.results.trace` não recebem argumentos e não alteram o arquivo. A primeira expõe o snapshot completo, revisão, SHA-256 e fontes. A segunda expõe o grafo, seus snapshots preservados, resultados e causas de invalidação. A interface mostra medidas atuais ou obsoletas, produtor, revisão de entrada, identidade da base e fontes afetadas. O conteúdo canônico é uma consulta complementar recolhida, sem função de edição.

O catálogo total possui 18 ferramentas em uma sessão com escrita e 9 em uma sessão de leitura. O arquivo continua escolhido exclusivamente pelo host. `--allow-write` é a autorização de lançamento para mutações, imposta novamente pelo serviço; argumentos MCP não selecionam caminho, ator ou permissão.

## Entrada numérica e recarga dos formulários

Os campos numéricos da criação rápida e do editor semântico aceitam vírgula ou ponto decimal, sem interpretar agrupamento de milhares. `1,5` e `1.5` representam o mesmo valor. `1 2`, `1.234,56`, NaN e transbordamento não são convertidos em um valor anterior ou aproximado. O texto inválido permanece visível com diagnóstico, e o formulário impede o envio do comando até a correção.

Essa leitura do texto pertence à apresentação. A conversão de unidades e a validade dimensional continuam no domínio. Os campos de força oferecem N e kN; os de momento oferecem N·m e kN·m. Não há opção de força no seletor de momento. O MCP recusa essa confusão de unidade na desserialização do tipo compartilhado.

Os buffers de texto são separados por projeto, área e identidade selecionada. Uma nova revisão recebida recarrega os dados persistidos das peças, materiais e ações selecionados, bem como referência de engenharia, mapa e procedência. Essa recarga preserva aba e seleção, mas descarta o texto numérico anterior. Assim, atualizar depois de uma edição por outra instância não reutiliza valores antigos com a revisão nova. Uma transformação feita em outra aba também aparece no formulário da peça antes de outra gravação.

## Ensaios executados

Comando focal, executado com o lock e dependências locais:

```sh
cargo test -p prumo-desktop --test ui --test model -p prumo-mcp-server --test stdio --locked --offline
```

Resultado: **27 testes passaram**, compostos por 7 testes novos da interface semântica, 6 regressões anteriores da interface e 14 testes reais de stdio, dos quais 3 exercitam o incremento desta issue. Os testes de stdio iniciam o executável `prumo-mcp` como processo filho. O cliente de sucesso usa rmcp e o cliente de contraprovas usa apenas JSON e pipes.

### Interface

Os testes em [apps/prumo-desktop/tests/model.rs](../../apps/prumo-desktop/tests/model.rs) acionam os widgets com cliques, seleção, teclado e eventos de acessibilidade. Eles conferem:

1. Criação de objeto somente visual sem barra ou vínculo implícito; mudança explícita para origem analítica; alteração de geometria, material, nós, barra, vínculo e ações. Uma coordenada de 500 cm é persistida como 5 m; uma força de −2 kN e um momento de 3 kN·m chegam a −2.000 N e 3.000 N·m.
2. Cópia com nova identidade IFC, divisão e fusão com histórico preservado, rotação de 90° e translação. A referência literal de extremidade mundial é `(10, 4, 0)` m; o teste exige erro absoluto menor que `1e-12` m antes da mudança de referencial e `1e-10` m depois. A troca de unidade de apresentação e a atribuição de mapa não substituem a origem de engenharia.
3. Dois objetos e duas portas coincidentes permanecem sem conexão até atribuição à rede e comando explícito. A conexão pode ser desfeita, conservando as portas e a rede. O projeto é reaberto e comparado integralmente.
4. Inspeção geométrica real, consulta de snapshot e edição de comprimento que deixa medidas dependentes obsoletas. As causas permanecem disponíveis, e nenhum registro tem `engineering_release: true`.
5. Digitação real de vírgula decimal e formatos ambíguos. Entradas inválidas mantêm projeto e bytes do arquivo iguais; a correção permite gravar. A criação rápida também aceita vírgula e recusa espaços internos.
6. Seleção de duas peças com o mesmo comprimento, após deixar texto inválido na primeira. A segunda carrega seu próprio número e pode ser editada sem herdar o erro da primeira.
7. Formulário de peça selecionada com 4 m, alteração por outra instância para 6 m e reabertura. O formulário passa a mostrar 6 m e uma gravação subsequente preserva essa medida. Uma translação pela outra aba recarrega os extremos `(2, 0, 0)` e `(8, 0, 0)` m antes de nova gravação.

As regressões de [tests/ui.rs](../../apps/prumo-desktop/tests/ui.rs) continuam cobrindo criação e reabertura, erro de arquivo, seleção de caminho, proteção contra gravação no projeto anterior, cadastro normativo e migração explícita de pacote.

### MCP e persistência entre superfícies

Os testes em [apps/prumo-mcp/tests/stdio/model.rs](../../apps/prumo-mcp/tests/stdio/model.rs) conferem:

1. Criar pela interface um objeto visual, abrir o mesmo arquivo pelo MCP, cadastrar material e ações tipadas, criar nós e barras, estabelecer relações 1:n e n:1, retirar um vínculo, registrar procedência, inspecionar e alterar geometria. A reabertura pela interface preserva todos os dados, UUIDs, GUID IFC existente e causas de invalidação.
2. Executar cópia, divisão, fusão, transformação rígida, mudança de referencial, unidade, atribuição e retirada de mapa, rede, portas, atribuição, conexão e desconexão pelo processo MCP. O comprimento de referência permanece 4 m com tolerância `1e-10` m. A consulta de snapshot não altera os bytes persistidos.
3. Negar edição e inspeção em sessão de leitura; rejeitar unidade de força no campo de momento, campos extras aninhados, caminho arbitrário, referência analítica inexistente, comprimento nulo e matriz que contém escala. Revisão antiga produz `revision_conflict`; todas as recusas conservam o projeto e os bytes do arquivo.

O teste de argumentos extras encontrou uma particularidade concreta da desserialização: variantes sem campos ignoravam dados adicionais. `SourceOrigin::Native` e `ActionFrame::Engineering` foram convertidas em variantes de struct vazia, mantendo o JSON externo e recusando os extras. A contraprova continua no teste de processo e no domínio.

As regressões do arquivo principal [stdio.rs](../../apps/prumo-mcp/tests/stdio.rs) conferem descoberta e versão, catálogo somente leitura, mensagem excessiva, CLI, revisão, normas, permissões, projeto compartilhado com a interface e cancelamento seguido de nova qualificação de componentes.

## Limites e fontes de implementação

Estas funções modelam e inspecionam dados. Elas não executam análise estrutural, cálculo de rede, dimensionamento, emissão normativa ou intercâmbio IFC completo. Os registros de procedência informados pelo usuário não concedem licença, autorização externa nem aprovação profissional. O desenho continua representando os eixos físicos e não presume conectividade.

Não foi acrescentada dependência de UI ou MCP. As decisões de reuso de egui e rmcp permanecem nas evidências anteriores; unidades, serialização, identidades e fontes oficiais da PRU-004 estão em [docs/componentes/PRU-004.md](../componentes/PRU-004.md). A autoridade de projeto continua nos tipos, nas validações e no serviço Rust compartilhados.

Arquivos centrais desta parcela: [prumo-ui/src/model.rs](../../crates/prumo-ui/src/model.rs), [prumo-ui/src/numeric.rs](../../crates/prumo-ui/src/numeric.rs), [prumo-desktop/src/lib.rs](../../apps/prumo-desktop/src/lib.rs), [prumo-mcp/src/lib.rs](../../crates/prumo-mcp/src/lib.rs) e [model_schema.rs](../../crates/prumo-mcp/src/model_schema.rs).
