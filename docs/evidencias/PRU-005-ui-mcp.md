# PRU-005: arquivo, migração e recuperação por interface e MCP

Data dos ensaios locais: 14/11/2026. Esta evidência cobre os controles, o adaptador e a interação com arquivos reais. As verificações do contêiner, do banco transacional, das interrupções de processo, da instalação e da interface nativa estão consolidadas na [matriz PRU-005](PRU-005.md).

## Contrato compartilhado

A interface e o MCP consultam `ProjectService::storage_status` e enviam as variantes de `StorageAction` a `ProjectService::execute_storage`. O serviço continua responsável por autorização, integridade, revisão, compatibilidade, transação e publicação. Os adaptadores não abrem SQLite, não montam arquivos ZIP e não implementam uma migração própria.

| Ação | Interface | MCP | Dados vinculados |
| --- | --- | --- | --- |
| Diagnosticar | **Inspecionar arquivo** | `prumo.storage.status` | Arquivo selecionado no host; nenhum argumento MCP |
| Migrar legado | Confirmação e **Migrar arquivo Prumo** | `prumo.storage.migrate` | Revisão, hash do diagnóstico e `confirm: true` |
| Criar backup | **Criar backup verificado** | `prumo.storage.backup` | Revisão e hash do diagnóstico |
| Recuperar | Seleção do candidato, confirmação e **Recuperar candidato selecionado** | `prumo.storage.recover` | Identificador opaco do candidato, hash do diagnóstico e `confirm: true` |

O diagnóstico inclui estado, formato, UUID e revisão quando disponíveis, SHA-256 do principal, identidade da inspeção, resumo do manifesto, candidatos e mensagens. Cada candidato identifica sua origem funcional, projeto, revisão, hash e possibilidade de recuperação. `inspection_sha256` também representa a ausência do principal e vincula as fontes conhecidas, permitindo recusar uma confirmação preparada antes de outra alteração.

As três operações de escrita exigem a autorização do serviço. No MCP, o host a concede com `--allow-write` para o arquivo escolhido no lançamento. As ferramentas recusam campos extras e não recebem caminho, SQL, ator ou permissão. Em leitura, o catálogo contém **10 ferramentas**; em escrita, **22**. Uma chamada direta de escrita omitida do catálogo continua sujeita à mesma autorização.

Uma ação concluída retorna o projeto, o diagnóstico atualizado, `changed` e a mensagem do serviço. As respostas MCP conservam `calculation_available: false`. A persistência não qualifica resultados de engenharia nem altera o estágio das capacidades normativas.

## Fluxo da interface

O painel **Arquivo e recuperação** é renderizado antes dos formulários que dependem de um projeto aberto. Ele mostra o diagnóstico mesmo quando a leitura do modelo falha. Os estados de legado, recuperação, invalidez e incompatibilidade expandem o painel. Detalhes dos hashes e conteúdos do manifesto ficam recolhidos no uso normal.

Concluir uma ação solicita novo desenho da tela. Isso atualiza o resultado de migração, backup ou recuperação sem depender de outro movimento do ponteiro; o ajuste foi motivado pela inspeção nativa da migração, na qual o disco já continha o pacote publicado e a tela ainda exibia o diagnóstico anterior.

No legado, a leitura preserva os bytes. A tela explica que a primeira edição válida migra automaticamente com cópia original e oferece a migração explícita com confirmação. O fluxo não exige uma etapa manual extra antes de editar. A migração conserva a identidade e o modelo validados pelo serviço.

Na recuperação, o usuário escolhe uma cópia pelo nome, revisão e começo do hash. A tela apresenta o projeto, a revisão e o hash completo do candidato antes da confirmação. Não é necessário digitar UUID, token ou JSON. A alteração do candidato, a atualização do diagnóstico e a digitação de outro caminho descartam a confirmação anterior. O adaptador também recusa uma ação quando o caminho exibido difere do arquivo efetivamente selecionado, inclusive se nenhum projeto foi aberto.

Inspecionar um arquivo substituído externamente também confere a identidade e a revisão do modelo exibido, além do hash do principal anteriormente observado. Uma divergência retira o editor antigo e solicita **Reabrir / atualizar**, mesmo se a substituição conservar UUID e revisão. A reabertura carrega o conteúdo atual e descarta os formulários da identidade anterior. O diagnóstico e o editor não passam a representar projetos diferentes no mesmo caminho.

Para cobrir o clique direto antes de uma nova inspeção, os comandos de edição da interface, inclusive a inspeção geométrica que salva medidas, usam `ProjectService::with_write_basis`. A base contém UUID do projeto exibido e SHA-256 dos mesmos bytes lidos por `read_with_basis`; ela fica guardada junto do modelo, independentemente do diagnóstico. O serviço compara ambos sob exclusão de escrita antes de aplicar o comando; não há uma segunda regra de integridade na interface. A criação continua sem base anterior. Se a base não estiver disponível, o formulário não envia a edição. Depois de uma recusa por mudança de arquivo, atualizar o diagnóstico também retira o editor antigo, sem aceitar apenas o hash novo sobre dados antigos.

Depois de um comando ou ação de armazenamento, a interface lê novamente modelo e hash como um par. Esse modelo precisa ser igual ao projeto retornado pela operação antes de receber uma nova base de edição. Se outro processo mudar o conteúdo entre essas etapas, ou se a leitura falhar, o editor é limpo e a tela solicita reabertura. O hash de uma consulta posterior de diagnóstico nunca substitui a base do modelo anterior.

O botão de criação só aparece quando o serviço informa `state: "missing"` e nenhum candidato. `format: "missing"` sozinho não permite criar. Se o serviço detectar uma versão futura em qualquer fonte conhecida, `state: "unsupported"` mantém os controles de escrita indisponíveis. O estado também retira um modelo anteriormente carregado da tela, evitando editar um principal legível cujo contexto de armazenamento foi bloqueado.

## Ensaios executados

As duas baterias novas usam arquivos temporários reais e o serviço de produção. O cliente MCP inicia `prumo-mcp` como processo filho, com o SDK rmcp oficial. As contraprovas de argumentos também usam o cliente independente de JSON e pipes. Os testes da interface acionam os widgets egui e a árvore de acessibilidade, incluindo seleção e confirmação.

Comandos focais:

```sh
cargo test -p prumo-desktop --test storage --locked --offline
cargo test -p prumo-mcp-server --test stdio storage_tests --locked --offline
```

Resultado local: **6 testes de interface e 3 testes MCP passaram**. Também passaram os **6 fluxos anteriores da interface e os 7 testes do modelo semântico**. A rodada completa de MCP e a inspeção nativa são registradas na matriz de integração após o build completo do workspace.

### Interface: seis provas

Arquivo: [apps/prumo-desktop/tests/storage.rs](../../apps/prumo-desktop/tests/storage.rs).

1. **Migração real.** O teste abre um JSON legado que contém modelo e extensão opaca de instalações, confirma a igualdade do projeto e dos bytes após a leitura, aciona confirmação e migração e verifica um pacote real com assinatura ZIP `PK`. O diagnóstico apresenta o hash exato do original como candidato preservado. UUID, objetos, modelo e extensão permanecem iguais. Uma nova instância de `Desktop` abre exatamente o projeto retornado pela migração.
2. **Recuperação quando o modelo não abre.** O teste cria e modifica pela interface, produz backup pelo botão, encerra a instância e corrompe intencionalmente o principal. A nova interface mostra erro e diagnóstico, sem oferecer criação. A seleção real da cópia e a confirmação restauram um pacote válido. Objetos, modelo e identidade são preservados e outra instância reabre o mesmo projeto restaurado.
3. **Confirmação e caminho desatualizados.** Depois de selecionar uma cópia para o principal corrompido, inspecionar novamente limpa a confirmação. Mudar o caminho exibido e tentar despachar a ação antiga é recusado mesmo com `project: None`; os bytes corrompidos permanecem e o outro arquivo não é criado.
4. **Versão futura.** Um pacote futuro independente não abre como projeto. A tela informa incompatibilidade e não oferece criação ou migração. Inspecionar e reabrir preservam os bytes exatos.
5. **Substituição externa no mesmo caminho.** Dois pacotes reais possuem UUIDs distintos e a mesma revisão. Substituir o primeiro pelo segundo e inspecionar retira os formulários, mantém o novo arquivo intacto e solicita reabertura. O teste também troca o conteúdo mantendo UUID e revisão, comprovando a comparação de hash. Reabrir carrega exatamente o projeto correspondente aos novos bytes.
6. **Edição direta antes de inspecionar.** O projeto é aberto a partir de uma cópia portátil e o teste confirma que nenhum banco local foi criado. O arquivo é substituído por outro projeto da mesma revisão ou por conteúdo diferente com os mesmos UUID e revisão. Clicar diretamente em **Adicionar peça e salvar** é recusado pelo serviço com a base guardada da leitura; o arquivo continua intacto, o editor é limpo e a reabertura mostra exatamente o conteúdo substituto, sem peça acrescentada.

### MCP: três provas de processo

Arquivo: [apps/prumo-mcp/tests/stdio/storage.rs](../../apps/prumo-mcp/tests/stdio/storage.rs).

1. **Legado, permissão e migração.** Um cliente de leitura consulta o legado e chama diretamente as três mutações; todas são negadas. O teste compara recursivamente nomes e bytes de todos os arquivos da pasta antes e depois, comprovando ausência de novos arquivos auxiliares. Uma nova sessão com escrita obtém seu próprio diagnóstico, recusa `confirm: false` e migra com confirmação válida. O original, as identidades e a extensão de instalações são preservados. Novos clientes MCP e Desktop reabrem o mesmo projeto.
2. **Interface, backup e recuperação por MCP.** A interface cria e salva uma peça; o cliente MCP cria o backup. Após corrupção intencional, a sessão de leitura ainda enxerga as cópias, mas não pode recuperá-las e não modifica a pasta. Uma sessão com escrita recusa confirmação falsa e hash forjado. A confirmação do candidato realmente consultado restaura o modelo, que também é reaberto pela interface.
3. **Diagnóstico e token desatualizado.** Arquivos ausente, corrompido e futuro são inspecionados sem modificar a pasta. Um argumento extra `path` é recusado como erro JSON-RPC `-32602`. Em projeto válido, alterar a revisão depois da consulta faz um backup com o token anterior retornar `storage_conflict`, mesmo que o argumento de revisão já esteja atualizado. Uma nova consulta permite o backup.

A fixture de versão futura fica em [prumo-storage/tests/fixtures/future-format.prumo](../../crates/prumo-storage/tests/fixtures/future-format.prumo). Possui 221 bytes e SHA-256 `14d0d4c4a43015b0c0723d6431b43ef64b80eb413c705cced691c30287576f64`. Ela foi produzida com um gravador ZIP independente, declara formato 2 e contém campo futuro. Não representa um projeto válido ou resultado calculado. As provas do codec distinguem versão incompatível de corrupção.

### Representação JSON antes do SDK

A preservação de números de extensões desconhecidas utiliza a opção `arbitrary_precision` de serde_json no workspace. Essa opção possui um marcador interno que, sem guarda, pode converter um objeto com chave reservada em número. A fronteira MCP agora chama a mesma `validate_json_representation` do domínio antes de o SDK construir seus valores genéricos.

O filtro de [prumo-mcp/src/input.rs](../../crates/prumo-mcp/src/input.rs) conserva as linhas válidas, limita a mensagem a 64 KiB, descarta o restante de uma linha excessiva e encerra sua tarefa quando o transporte é descartado. Somente `ReservedNamespace` é encaminhado ao SDK como um envelope inválido, para que o próprio SDK emita `-32600` com ID nulo; nenhum identificador ou argumento é interpretado dos bytes recusados. O motivo fica em stderr. A sessão continua disponível para a próxima linha.

`InvalidJson` conserva os bytes originais para o SDK. O código da versão fixada rmcp 3.10.0 descarta erros de sintaxe ou final incompleto sem emitir resposta e retoma a próxima linha. A guarda não converte esse comportamento em `Invalid Request` e a evidência não presume um `ParseError` que o transporte não emitiu. A prova inclui string interrompida e escape inválido, seguidos de uma consulta cujo ID válido precisa ser o único respondido.

A contraprova em [stdio.rs](../../apps/prumo-mcp/tests/stdio.rs) envia bytes diretamente, sem construir antes um `serde_json::Value`. Ela tenta fornecer uma coordenada como objeto numérico reservado, com chave literal e escapes `\u0024` e `\u004e`, compara projeto e bytes após cada recusa e consulta a lista de ferramentas na linha seguinte. Um nome contendo o mesmo marcador como texto continua sendo aceito. A guarda restringe chaves de objeto, não o conteúdo normal de strings. Esse teste passou em processo real, assim como a regressão da mensagem excessiva e da retomada do transporte.

### Orçamento da prova normativa completa

O `LIMIT` de 10 segundos do cliente de teste foi introduzido na PRU-001, commit `eb47170246492a97233ab4a65822dbcbbef38f9f`. Ele limita a descoberta, o helper comum de chamadas, a leitura do cliente independente e o encerramento do processo. É uma proteção contra espera indefinida do teste, não um requisito de latência do armazenamento nem uma configuração do servidor.

Com o catálogo completo e o histórico preservado, a amostra final do serviço levou **10,867 s** para migrar em debug e gerou um pacote de **12.660.812 bytes** na revisão 4. Essa medição excedeu o orçamento anterior e continua preservada no [registro de latências](PRU-005-latencia.json). Após conferir as validações e eliminar somente trabalho redundante, foi aprovado um orçamento funcional explícito de **30 s**, local ao teste `normative_change_preserves_active_snapshot_until_explicit_migration`, para `pin`, `register` e `migrate`.

Esse orçamento permite concluir a transação observada, a serialização e a comunicação com o processo, com margem para agendamento de CI. Não é um percentil medido, uma qualificação de desempenho ou um novo prazo do produto. Discovery, o helper comum e o cliente wire continuam com 10 s. Neste cenário, as consultas e a recusa de migração com `confirm: false` também conservam 10 s; as demais provas não tiveram seus prazos alterados.

Todas as asserções foram preservadas: identidade do pacote ativo antes da migração, alteração do candidato, impacto previsto, histórico, hashes, ausência de mutação na prévia e na recusa, persistência após reinício, bloqueio normativo e reabertura pela interface. A mudança de orçamento não reduz a fixture nem retira verificações do serviço.

Execução focal completa, sem outro build ou ensaio concorrente:

```sh
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo test -p prumo-mcp-server --test stdio normative_change_preserves_active_snapshot_until_explicit_migration --locked --offline -- --nocapture
```

| Chamada no processo MCP | Tempo observado | Orçamento da chamada neste teste |
| --- | --- | --- |
| `prumo.norms.pin` | 2,812271755 s | 30 s |
| `prumo.norms.register` | 7,187015797 s | 30 s |
| `prumo.norms.migrate` | 11,319194606 s | 30 s |

Resultado: **1 teste passou**, 17 ficaram fora do filtro, duração total de **46,91 s**, além de **16,60 s** de compilação. O pacote desta execução MCP terminou na revisão **4**, com **12.660.856 bytes**. O teste concluiu as comparações e a reabertura final; a migração de 11,32 s continua sendo maior que o antigo orçamento de 10 s, e isso não foi apresentado como atendimento daquele limite.

## Limites e revisão

As provas acima cobrem a interação e o uso do serviço. A interrupção abrupta entre as fases de transação, autosave, substituição e recuperação pertence aos testes de processo do `prumo-app`; não foi introduzida flag de falha no servidor distribuído. A segurança dos nomes internos, dos limites de expansão, das versões e dos hashes pertence aos testes de `prumo-storage` e à revisão independente. Essas evidências são relacionadas na matriz principal, sem atribuir às superfícies uma garantia que elas não implementam.

O armazenamento opera localmente. Os adaptadores não introduzem nuvem, intercâmbio IFC completo, cálculo de estruturas ou instalações, nem emissão normativa. A UI e o MCP mantêm as mesmas regras do serviço para criação, edição, revisão e recuperação.

Arquivos centrais: [prumo-ui/src/storage.rs](../../crates/prumo-ui/src/storage.rs), [prumo-ui/src/lib.rs](../../crates/prumo-ui/src/lib.rs), [prumo-desktop/src/lib.rs](../../apps/prumo-desktop/src/lib.rs), [prumo-mcp/src/lib.rs](../../crates/prumo-mcp/src/lib.rs) e o [contrato compartilhado](../../crates/prumo-domain/src/storage.rs).
