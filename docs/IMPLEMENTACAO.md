# Estado da implementação

O planejamento integral permanece nas 250 issues. Esta página identifica o incremento executável e não libera capacidades futuras.

| Issue | Implementação | Evidência |
|---|---|---|
| PRU-001 | Fundação Rust, interface, persistência e MCP local | [Matriz PRU-001](evidencias/PRU-001.md) |
| PRU-002 | Registro normativo, cobertura, pacote por projeto e bloqueio de resultados incompletos | [Matriz PRU-002](evidencias/PRU-002.md) |
| PRU-003 | Adaptadores substituíveis, ensaios de componentes, execução isolada e decisões de reuso | [Matriz PRU-003](evidencias/PRU-003.md) |
| PRU-004 | Modelo físico e analítico, materiais, ações, unidades, identidades, topologia MEP e dependências rastreáveis | [Matriz PRU-004](evidencias/PRU-004.md) |
| PRU-005 | Banco transacional, pacote local versionado, hashes, autosave, backup, migração e recuperação | [Matriz PRU-005](evidencias/PRU-005.md) |
| PRU-006 | Comandos tipados, prévias por hash, lotes atômicos, idempotência, concorrência e desfazer/refazer auditados | [Matriz PRU-006](evidencias/PRU-006.md) |
| PRU-007 | Escopos por projeto/disciplina, recursos MCP e perfis moderno e legado testados separadamente | [Matriz PRU-007](evidencias/PRU-007.md) |
| PRU-008 | Protocolos prévios, referências independentes, casos analíticos e relatórios de verificação por SHA | [Matriz PRU-008](evidencias/PRU-008.md) |
| PRU-009 | Predicados, interseções, faces, tolerâncias distintas e correções explícitas com prévia | [Matriz PRU-009](evidencias/PRU-009.md) |
| PRU-010 | Catálogos versionados, materiais por condição e propriedades de seções com convergência de torção | [Matriz PRU-010](evidencias/PRU-010.md) |
| PRU-011 a PRU-250 | Planejadas | [Roadmap](../ROADMAP.md) |

O incremento modela peças, nós e barras analíticas, materiais, ações, ambientes, redes, circuitos, portas e conexões. Vínculos e conectividade são explícitos. A inspeção produz comprimentos e contagens com bases preservadas e invalidação rastreável. Nenhum solver, dimensionamento normativo, intercâmbio IFC completo ou documento de engenharia está liberado por esse incremento.


O pacote `.prumo` conserva banco, projeções auditáveis, catálogos, snapshots, resultados e extensões declaradas. A recuperação exige diagnóstico atual e escolha explícita da revisão. O diário de comandos conserva recibos idempotentes e as revisões de desfazer/refazer. Catálogos preservam a fonte e o histórico; a troca de uma entrada invalida apenas resultados que declaram essa dependência.
