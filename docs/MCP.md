# MCP e operação do Prumo por agentes

**Estado:** servidor Rust implementado com transporte local stdio, cliente real e projeto persistente. **Data da referência:** 14/11/2026. A seção seguinte descreve o catálogo efetivo até a implementação das PRU-006 e PRU-007; as entregas seguintes ampliam o mesmo serviço. Os demais requisitos deste documento orientam a evolução sequencial e só estão disponíveis quando anunciados pelo servidor. Evidências de integração e limites constam das entregas de cada issue.

## Catálogo implementado

O processo recebe o arquivo selecionado pelo host em `--project <arquivo.prumo>`. A sessão começa somente para leitura. A opção de lançamento `--allow-write` concede edição nesse mesmo arquivo; nenhuma ferramenta pode trocar o projeto, o ator ou a permissão. O serviço impõe a autorização mesmo quando um cliente chama diretamente uma operação de escrita que não foi anunciada.

```sh
cargo run -p prumo-mcp-server -- --project meu-projeto.prumo
cargo run -p prumo-mcp-server -- --project meu-projeto.prumo --allow-write
```

O catálogo é filtrado pelos escopos e disciplinas concedidos no lançamento e pode ser consultado por `tools/list`. Usa MCP `2026-07-28`, `server/discover`, rmcp `3.10.0` fixado e metadados de versão por requisição. A opção `--protocol 2025-11-25` seleciona o adaptador legado separado, com `initialize` e `notifications/initialized`. O servidor recusa revisões incompatíveis com o perfil escolhido. HTTP permanece desabilitado.

| Ferramentas atuais | Dados e comportamento |
| --- | --- |
| `prumo.project.read` | Projeto completo selecionado pelo host, revisão e identidades |
| `prumo.registry.read`, `prumo.registry.audit` | Catálogo normativo e de capacidades; auditoria de cobertura e pendências |
| `prumo.norms.package` | Pacote candidato, pacote ativo e impacto previsto da migração |
| `prumo.results.gate` | Motivos da liberação ou do bloqueio de uma capacidade |
| `prumo.components.list`, `prumo.components.qualify` | Inventário e bateria fixa de qualificação em processo separado; não calculam o projeto |
| `prumo.model.snapshot` | Snapshot canônico completo de entradas, versão própria, revisão, SHA-256 e fontes; sem argumentos |
| `prumo.results.trace` | Medidas geométricas salvas, bases preservadas, dependências, estados atuais ou obsoletos e causas de invalidação; sem argumentos |
| `prumo.storage.status` | Formato, integridade, revisão, manifesto e candidatos de recuperação, inclusive quando o projeto não abre; sem argumentos e sem gravar arquivos |
| `prumo.project.create`, `prumo.project.rename` | Criação sem substituir arquivo existente e alteração do nome com revisão esperada |
| `prumo.model.add_member`, `prumo.model.add_environment` | Criação rápida de peça com barra analítica vinculada, ou de ambiente |
| `prumo.norms.register`, `prumo.norms.pin`, `prumo.norms.migrate` | Cadastro de metadados públicos, fixação e migração explícita de pacote; nenhuma aprovação é inventada |
| `prumo.model.edit` | Uma das variantes tipadas fechadas de `ModelEdit`, com unidades e referências explícitas |
| `prumo.model.inspect` | Atualiza comprimentos e resumo geométrico vinculados ao snapshot. Exige escrita e revisão esperada |
| `prumo.storage.migrate`, `prumo.storage.backup`, `prumo.storage.recover` | Migração de legado com original preservado, backup verificado e recuperação de candidato selecionado; exigem escrita e identidade atual do diagnóstico |

### Escopos, disciplinas e recursos

O host pode conceder `read`, `model`, `calculation` e `documents` por `--scopes`. As operações exigem leitura; `--scopes none` concede nenhum acesso. `--allow-write` mantém a compatibilidade e equivale a `--scopes read,model`. A opção de escrita não autoriza cálculo nem emissão. Os respectivos escopos não tornam uma capacidade futura disponível.

```sh
prumo-mcp --project meu-projeto.prumo --scopes read,model --disciplines structure
prumo-mcp --project meu-projeto.prumo --scopes read --project-id UUID_DO_PROJETO
prumo-mcp --project meu-projeto.prumo --protocol 2025-11-25
```

O UUID opcional é escolhido pelo host. O serviço também fixa a identidade do projeto na primeira leitura e recusa outro projeto colocado no mesmo caminho durante a sessão. O ator é `mcp-local` por padrão, ou o valor de `--actor` informado no lançamento. Nenhuma ferramenta recebe caminho, ator, escopo ou disciplina de autorização como argumento. Dados importados, nomes, metadados e extensões não concedem permissões.

`prumo.access.read` mostra a concessão ativa. `prumo.model.query` retorna somente peças, redes, portas e conexões autorizadas, com UUID do projeto, revisão e coordenadas em metros. Essa representação não é um arquivo completo de projeto: omite histórico, catálogo, identidades de origem e extensões genéricas para evitar referências a outras disciplinas. Uma concessão parcial pode editar as entidades permitidas, mas não alterar configurações globais, materiais compartilhados, normas ou arquivos. Consultas completas e transações que exibem histórico global exigem todas as disciplinas.

| Recurso | Conteúdo | Exigência |
| --- | --- | --- |
| `prumo://project/{UUID}/model` | Mesmo modelo filtrado de `prumo.model.query` | Leitura nas disciplinas concedidas |
| `prumo://project/{UUID}/snapshot` | Entradas canônicas, fontes e SHA-256 | Leitura de todas as disciplinas |
| `prumo://project/{UUID}/results` | Revisão, medidas em metros, dependências e invalidações | Leitura de todas as disciplinas |

`resources/list` anuncia somente recursos autorizados e `resources/read` verifica novamente a identidade. URI de outro projeto, URI de arquivo, caminho arbitrário ou recurso não anunciado não oferece acesso. No perfil moderno, consultas e recursos têm `ttlMs: 0` e `cacheScope: "private"`; o adaptador legado não mistura esses campos com seu formato de resposta.

### Prévia, aplicação e histórico de lotes

| Ferramenta | Entrada | Comportamento |
| --- | --- | --- |
| `prumo.model.preview_change` | `project_id`, `expected_revision`, `idempotency_key`, `operations`; `preconditions` opcional | Valida e prepara um lote sem alterar a revisão do modelo; retorna efeitos, invalidações e `plan_sha256` |
| `prumo.model.apply_change` | `plan_sha256` | Aplica exatamente o plano preparado, com os mesmos identificadores e base |
| `prumo.model.history` | Nenhuma | Recibos auditados e comando atualmente disponível para desfazer/refazer |
| `prumo.model.undo`, `prumo.model.redo` | `project_id`, `expected_revision`, `idempotency_key`, `target_command_id` | Nova transação compensatória, preservando autoria, identidade e histórico |

Cada operação do lote usa o discriminador `operation` do contrato Rust `Command`, com `expected_revision` igual à revisão base. `edit_model` contém a variante tipada `edit` já usada pela edição imediata. Criação de projeto fica fora do lote. Pré-condições identificam uma fonte por `kind` e `id`; `sha256: null` exige sua ausência. Os schemas recusam campos extras e limitam o número de operações.

Reenviar o mesmo plano ou comando idempotente recupera seu recibo, sem duplicar efeitos. Uma chave reutilizada com conteúdo diferente é recusada. O recibo conserva a revisão original; a resposta também traz o projeto atual, para que um reenvio não substitua na tela trabalho posterior. Revisão antiga, pré-condição alterada ou permissão insuficiente não produz gravação parcial. Desfazer/refazer conserva o histórico e não transforma resultados obsoletos em evidência atual.

### Arquivo, migração e recuperação

`prumo.storage.status` retorna o objeto `storage`, com estado, formato, identidade e revisão do projeto quando legíveis, hash do arquivo principal, resumo do manifesto, candidatos e diagnósticos. A consulta funciona para arquivo ausente, legado, corrompido ou de versão futura. Ela não cria banco de trabalho, autosave, backup ou qualquer arquivo auxiliar. A operação continua vinculada ao único caminho escolhido pelo host no lançamento.

O estado e o formato têm funções distintas. `format` descreve o arquivo principal; `state` resume também as fontes de recuperação. Por isso, um principal ausente ou legível pode apresentar `state: "unsupported"` se uma cópia ou o banco de trabalho tiver versão futura. Nesse caso, o serviço preserva o conteúdo e bloqueia escrita. `recovery_available` também pode indicar um banco de trabalho com revisão mais recente que o principal publicado. O cliente deve apresentar o diagnóstico e as opções anunciadas, sem supor que um arquivo legível é a revisão mais recente.

| Ferramenta | Argumentos obrigatórios | Efeito |
| --- | --- | --- |
| `prumo.storage.status` | Nenhum | Diagnóstico e candidatos com `id`, tipo, nome, projeto, revisão, SHA-256 e `recoverable` |
| `prumo.storage.migrate` | `expected_revision`, `inspection_sha256`, `confirm` | Com `confirm: true`, publica o pacote nativo e preserva os bytes do legado |
| `prumo.storage.backup` | `expected_revision`, `inspection_sha256` | Cria uma cópia verificada da revisão publicada |
| `prumo.storage.recover` | `candidate_id`, `inspection_sha256`, `confirm` | Com `confirm: true`, restaura o candidato permitido e preserva o arquivo substituído |

O `inspection_sha256` retornado pela consulta é uma identidade de 64 caracteres hexadecimais, vinculada ao principal, inclusive sua ausência, e às fontes inspecionadas. O serviço consulta novamente esses dados sob exclusão de escrita antes de executar a ação. Um token antigo produz `storage_conflict`; não autoriza operar sobre um arquivo ou candidato alterado. `candidate_id` é o identificador opaco retornado pela consulta, nunca um caminho. Todos os schemas recusam propriedades extras. A confirmação não concede escrita a uma sessão de leitura.

Em leitura, `can_migrate`, `can_backup`, `can_recover` e `recoverable` permanecem falsos. As cópias ainda podem ser identificadas no diagnóstico. Ao mudar a autorização de lançamento, consulte o estado novamente, pois o contexto de permissão participa do diagnóstico. Uma recuperação incompatível com a identidade conhecida do projeto é recusada. Versões futuras no principal ou nas fontes conhecidas bloqueiam a substituição por uma versão antiga.

As três mutações retornam `project`, `storage`, `changed`, `message` e `calculation_available: false`. Os códigos de negócio incluem `storage_conflict`, `recovery_required`, `invalid_storage` e `unsupported_storage`, além dos erros de permissão, entrada e revisão. A recusa não vira sucesso textual. Quando uma gravação interrompida deixa uma revisão pendente recuperável, o diagnóstico orienta a escolha explícita antes de outra edição.

Criação e edições válidas usam o banco transacional e publicam o pacote `.prumo`, com autosave e cópia anterior quando aplicáveis. A leitura de JSON legado não o modifica. A primeira edição válida também pode migrá-lo automaticamente, preservando o original, sem exigir que o cliente faça uma migração manual prévia. A migração explícita acima oferece o mesmo serviço para quem deseja conferir essa mudança antes de editar. Evidências dos fluxos reais por interface e processo MCP estão em [PRU-005-ui-mcp.md](evidencias/PRU-005-ui-mcp.md).

### Edição física, analítica e de redes

`prumo.model.edit` recebe `expected_revision` e `edit`. O segundo campo contém um objeto com discriminador também chamado `edit`. O schema usa `oneOf`, enumera todas as variantes e recusa propriedades extras em cada objeto. A desserialização e as regras finais pertencem ao domínio Rust compartilhado com a interface.

Exemplo de argumentos para criar um objeto somente visual:

```json
{
  "expected_revision": 1,
  "edit": {
    "edit": "add_physical",
    "name": "Referência visual",
    "discipline": "structure",
    "role": "visual_only",
    "start": { "values": [0, 0, 0], "unit": "metre" },
    "end": { "values": [400, 0, 0], "unit": "centimetre" },
    "material_id": null,
    "environment_id": null
  }
}
```

A criação acima não cria barra analítica nem libera dimensionamento. Nós, barras e vínculos são operações próprias. Um objeto pode se relacionar com várias barras e uma barra com vários objetos. Portas precisam ser atribuídas a uma rede ou circuito e conectadas explicitamente; proximidade ou interseção gráfica não cria conexão.

Força aceita `newton` ou `kilonewton`. Momento aceita `newton_metre` ou `kilonewton_metre`. Enviar `newton` no campo de momento é erro de entrada. Coordenadas aceitam `metre`, `centimetre` e `millimetre`; o armazenamento usa tipos em SI. Os campos `origin_world_m`, `map_origin_m` e `rotation_radians` declaram SI no próprio nome. A origem de engenharia e o georreferenciamento são separados.

Uma mutação válida retorna o projeto e a nova revisão. Edições sem permissão, revisão desatualizada, grandeza inválida ou referência órfã não alteram o arquivo. Os códigos de negócio atuais incluem `permission_denied`, `revision_conflict` e `invalid_input`; falhas no formato dos argumentos usam erro JSON-RPC `-32602`. Uma chamada repetida com a revisão anterior falha por conflito, evitando uma segunda alteração. Para lotes com chave durável de idempotência, prévia e aplicação exata por hash, use o contrato de transações abaixo. As operações imediatas preservam seu contrato de revisão esperada.

`prumo.model.inspect` mede a geometria efetiva e salva suas dependências. Seus registros mantêm `engineering_release: false`, e as respostas continuam declarando `calculation_available: false`. Uma alteração relevante conserva os valores históricos, marca os registros afetados como obsoletos e registra revisão, ator, fontes alteradas e resultados dependentes. O mapa completo entre variantes, controles da interface e ensaios está em [PRU-004-ui-mcp.md](evidencias/PRU-004-ui-mcp.md).

As mensagens de entrada permanecem limitadas a 64 KiB. Antes da desserialização do SDK, a guarda compartilhada recusa chaves de objeto no namespace interno `$serde_json::private::`, inclusive quando escritas com escapes Unicode. Isso impede que a biblioteca interprete um objeto como se fosse um número. O SDK responde erro JSON-RPC `-32600` com ID nulo, o motivo fica em stderr e a próxima linha pode ser processada. O mesmo texto como valor de uma string continua permitido. Erros de sintaxe seguem intactos ao SDK; o transporte rmcp 3.10.0 descarta a linha malformada sem resposta e continua na seguinte. Uma linha excessiva é descartada até a próxima quebra de linha, sem crescer o buffer indefinidamente.

Somente protocolo sai em stdout. Os testes usam o SDK oficial em processo filho e um cliente independente de JSON/stdio, além de criar pela interface, modificar por MCP e reabrir o mesmo projeto. Referências: [PRU-001](evidencias/PRU-001-mcp.md), [PRU-002](evidencias/PRU-002-cobertura.md), [PRU-003](evidencias/PRU-003.md) e [PRU-005](evidencias/PRU-005-ui-mcp.md).

### Geometria, verificação e catálogos

| Ferramenta | Escopo | Retorno e efeito |
|---|---|---|
| `prumo.geometry.audit` | leitura | Diagnósticos geométricos por filtro, revisão, snapshot e tolerâncias, sem alteração |
| `prumo.geometry.face` | leitura | Área, normal, validade e problemas de uma face fornecida em metros |
| `prumo.geometry.preview` | modelagem | Plano de correções, hash, dimensões antes/depois e conexões explícitas propostas |
| `prumo.verification.protocols` | leitura | Casos e tolerâncias registrados antes de executar |
| `prumo.verification.run` | cálculo | Bateria analítica e relatório com identidade SHA, commit, build, plataforma e dependências |
| `prumo.catalogue.read` | leitura | Entradas atuais e histórico do catálogo, com revisão e snapshot do projeto |
| `prumo.catalogue.material` | leitura | Estado único do material nas condições declaradas, fonte e valores preservados |
| `prumo.catalogue.section` | cálculo | Área, centroide, inércias e torção da seção, método e estado de convergência |

Essas operações exigem concessão para todas as disciplinas, pois retornam entradas ou evidências comuns ao projeto. O escopo `calculation` habilita somente os ensaios e propriedades implementados; ele não libera solver do projeto, dimensionamento ou emissão técnica.

`geometry.audit` recebe `filter` opcional com `discipline`, `network_id` e `environment_id`. `geometry.face` recebe `face`, com `vertices_m` e `expected_normal`. `geometry.preview` recebe uma lista `repairs`: `snap_endpoint` declara `object_id`, `endpoint` (`start`/`end`), `target_m` e `reason`; `connect_ports` declara `network_id`, `from_port`, `to_port` e `reason`. Aplicar usa a variante `apply_geometry_repair` de `model.edit`, com `plan_hash` e a mesma lista. Mudanças no projeto, no ator ou na lista invalidam a prévia. Contato visual nunca cria uma ligação.

`catalogue.section` recebe `entry_id`. `catalogue.material` recebe `entry_id` e `conditions`, mapa de grandeza para valor físico. Um valor é representado, por exemplo, por `{"quantity":"absolute_temperature","decimal":"20.000","unit":"degree_celsius","basis":"measured","design_basis":null}`. A representação decimal original e a unidade permanecem no registro; a conversão para SI é explícita. Falta de condição, ambiguidade, unidade incompatível e valor fisicamente impossível são erros.

As novas variantes de `model.edit` incluem `put_catalog_entry`, `assign_catalog_entry` e `clear_catalog_assignment`. A entrada declara identidade, revisão consecutiva, nome, fonte, aplicação, customização opcional e um conteúdo de material, seção ou propriedades. `tools/list` fornece o schema completo. Uma customização usa nova identidade e identifica a versão e o SHA da base. Não se pode substituir silenciosamente uma versão nem associar dois materiais de autoridades distintas à mesma peça.

A torção retorna `j_m4` somente para solução analítica ou convergência demonstrada. Se o cálculo limitado não convergir, o relatório conserva estimativa e diagnóstico, com `j_m4: null`. Esses relatórios de consulta identificam a versão exata da entrada; não são resultados de dimensionamento persistidos. A infraestrutura de dependências exige que um resultado derivado declare as entradas usadas para obter invalidação seletiva.

## 1. Princípios

O servidor MCP próprio será implementado em Rust. O MCP é um adaptador para os mesmos comandos Rust usados pelo usuário, conforme [RUST.md](RUST.md). O agente não altera diretamente banco, arquivos de resultados ou parâmetros internos do solver. Modelagem, cálculo, verificação e documentação são funções determinísticas, com entradas e saídas rastreáveis.

O agente pode propor alternativas, montar um modelo, consultar dados, solicitar cálculos, explicar diagnósticos e preparar documentos dentro da autorização recebida. O cálculo não é executado por geração de texto. Uma explicação deve apontar o resultado, a hipótese, a verificação e a versão correspondentes.

Autorização de edição não implica autorização de transmissão a qualquer provedor. Leitura, edição, análise, exportação e emissão são capacidades distintas e restritas ao projeto. Operações reversíveis previamente autorizadas podem continuar sem confirmação repetitiva; a autorização e seu limite permanecem auditáveis.

## 2. Revisões de protocolo e transportes

A referência pública atual consultada é **MCP 2026-07-28**. Ela usa versão e capacidades por requisição e exige `server/discover`. O fluxo de inicialização de **2025-11-25** pertence ao adaptador legado. A implementação fixa versões suportadas, registra as respectivas dependências e testa cada caminho; não deve combinar regras incompatíveis dos dois perfis. [Versionamento oficial](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning.md).

| Perfil | Comportamento que a implementação deve respeitar |
| --- | --- |
| 2026-07-28 | Descoberta, metadados e capacidades por requisição; rejeição explícita de versão não suportada |
| 2025-11-25, quando declarado | Inicialização e negociação legadas, com testes próprios |
| Extensões | Uso apenas quando ambas as partes as suportarem; fallback documentado ou erro explicativo |

**Transporte local inicial:** stdio. Mensagens de protocolo em stdout e logs em stderr; nenhuma mensagem humana misturada ao canal. O processo respeita fechamento de entrada e cancelamento. [Especificação stdio](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio.md).

**HTTP:** somente disponibilizar após implementação e testes específicos. Na revisão moderna o endpoint utiliza POST, sem sessão de protocolo nem endpoint GET de fluxo. Validar origem e autenticação; quando local, restringir a interface de rede autorizada. Requisições devem conter os metadados e cabeçalhos exigidos pela revisão. [Streamable HTTP](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http).

A presença do SDK não basta para afirmar compatibilidade. Testar com versões fixas de pelo menos dois clientes independentes que implementem o perfil anunciado. O [SDK Rust oficial rmcp](https://github.com/modelcontextprotocol/rust-sdk) é o candidato prioritário para o servidor próprio. Fixar versão/commit, features e revisão de protocolo; a declaração de main não qualifica automaticamente uma release. SDKs de outras linguagens podem servir a clientes externos de interoperabilidade. [IfcMCP](https://docs.ifcopenshell.org/ifcmcp.html) permanece candidato externo para funções IFC delimitadas, sem substituir os comandos Rust ou permitir mutações irrestritas. Aplicam-se [REUSO.md](REUSO.md) e [fontes-rust.json](fontes-rust.json).

## 3. Contratos de domínio

Todo payload tem schema versionado. Grandezas incluem unidade, números devem ser finitos, listas têm limites e campos desconhecidos são rejeitados quando não previstos. Textos externos não são instruções de execução.

### Contexto de um comando de mutação

| Campo | Regra |
| --- | --- |
| `project_id` | Identidade do projeto autorizado |
| `base_revision` | Revisão sobre a qual a mudança foi preparada |
| `command_id` | Identidade auditável do comando |
| `idempotency_key` | Identidade de negócio para impedir repetição de efeito |
| `schema_version` | Versão exata do contrato |
| `payload` | Dados tipados para a operação permitida |
| Ator e permissões | Derivados do contexto autenticado; nunca aceitos como autodeclaração do payload |

Chave idempotente é vinculada ao ator, projeto, operação e hash do payload. Reenvio idêntico retorna a execução original; reutilização com conteúdo diferente falha. O identificador JSON-RPC não substitui essa chave.

### Prévia e aplicação

1. A ferramenta de prévia valida o comando e calcula diferenças, objetos afetados, hipóteses, resultados que ficarão obsoletos e permissões necessárias.
2. A resposta fornece `plan_id`, `plan_hash`, `base_revision`, validade e diagnósticos, sem modificar o projeto.
3. Aplicar exige o plano e o hash correspondentes. Se revisão, autorização, conteúdo ou dependência relevante mudou, o plano é recusado e precisa ser recalculado.
4. O serviço executa transação atômica, grava auditoria e retorna nova revisão. Uma autorização anterior válida pode permitir essa aplicação sem nova interação humana.

Exemplo de payload de aplicação, apenas ilustrativo do contrato:

```json
{
  "project_id": "8de96549-6433-4144-8439-7329a16d8069",
  "base_revision": 12,
  "command_id": "8a491d76-5091-4e90-9742-382715c83574",
  "idempotency_key": "c5d31722-dcfe-4c47-b599-cdf88c025512",
  "schema_version": "1.0",
  "plan_id": "7b404a6f-324b-43a9-96e2-c8a38b4b07b2",
  "plan_hash": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
}
```

Os valores do exemplo não autorizam uma operação real. A validação final de schemas e a nomenclatura pública serão versionadas na implementação, preservando as garantias aqui definidas.

### Resposta de negócio

Retornar estado, revisão, identificadores, dados estruturados com unidades, diagnósticos e referência à auditoria. Resultados de cálculo também incluem snapshot, solver, perfil normativo, critério de convergência e identidade da evidência. O adaptador envolve essa resposta no formato da revisão MCP negociada, incluindo os campos exigidos por ela.

Erros de negócio possuem códigos estáveis, tais como `REVISION_CONFLICT`, `IDEMPOTENCY_CONFLICT`, `INVALID_UNIT`, `FORBIDDEN_OPERATION`, `UNSUPPORTED_CAPABILITY`, `INVALID_MODEL`, `ANALYSIS_NOT_CONVERGED`, `STALE_RESULTS` e `ISSUE_BLOCKED`. Eles não devem ser confundidos com os códigos JSON-RPC de protocolo. A resposta identifica elementos e correção possível, sem transformar falha em sucesso textual.

## 4. Catálogo funcional inicial

As ferramentas abaixo são requisitos; somente ferramentas efetivamente implementadas e autorizadas serão anunciadas. Seus nomes finais podem ser ajustados por versão de contrato, com migração explícita.

| Grupo de ferramentas | Entradas essenciais | Saída e restrição |
| --- | --- | --- |
| `prumo.project.create`, `prumo.project.read` | Dados mínimos do projeto ou identidade autorizada | Projeto, revisão e perfil; criação é idempotente |
| `prumo.model.query` | Projeto, revisão, seleção limitada, propriedades | Objetos e unidades; paginação e limites de volume |
| `prumo.model.preview_change` | Contexto, comando permitido e payload | Plano, diferenças, hipóteses e invalidações, sem mutação |
| `prumo.model.apply_change` | Projeto, revisão, plano, hash e chave idempotente | Nova revisão e auditoria; aplica exatamente a prévia |
| `prumo.model.undo` | Revisão e comando reversível | Comando compensatório auditado; não apaga histórico |
| `prumo.model.validate` | Projeto, revisão e conjunto de verificações | Diagnósticos de geometria, topologia, dados e aplicabilidade |
| `prumo.analysis.submit` | Snapshot, perfil de análise e chave idempotente | Identidade durável de job, estado e dados de acompanhamento |
| `prumo.analysis.status`, `prumo.analysis.cancel` | Job autorizado | Estado ou intenção de cancelamento; conclusão exige confirmação do worker |
| `prumo.results.query` | Job, snapshot e grandezas selecionadas | Valores com unidade, estado, versão e origem |
| `prumo.design.check` | Snapshot, resultados compatíveis e regras implementadas | Verificações e limites, com itens não cobertos explicitados |
| `prumo.exchange.preview_import` | Identificador controlado de arquivo e perfil | Mapeamentos, perda de informação e proposta de importação |
| `prumo.exchange.export` | Revisão, formato e perfil declarado | Arquivo em destino autorizado e relatório de intercâmbio |
| `prumo.documents.preview` | Conjunto coerente de modelo, resultados e template | Prévia identificada, sem declaração de emissão |
| `prumo.documents.issue` | Conjunto exato, revisão, aprovação válida e responsável | Emissão auditada ou bloqueio com motivo |

Não há ferramenta genérica para shell, execução Python arbitrária, SQL, chamada de função por caminho livre ou gravação em caminho escolhido pelo modelo. Arquivos são identificadores obtidos por seleção autorizada e resolvidos pelo serviço. Um conector técnico de reuso pode operar internamente, mas não contorna o contrato de domínio.

## 5. Jobs, reconexão e resultados

Jobs persistem estado antes de responder à submissão. Devem registrar snapshot, limites, motor, parâmetros e estado terminal. O mesmo envio repetido não cria cálculo duplicado. Progresso informa estágio e quantidade mensurável quando disponível, sem percentuais fictícios.

A extensão oficial **Tasks** pode carregar acompanhamento durável se suportada por ambas as partes. O fallback permanece no contrato de aplicação: submissão retorna `job_id` e consulta usa esse identificador. [Extensão Tasks](https://modelcontextprotocol.io/extensions/tasks/overview).

Perda de conexão não significa conclusão. Cancelamento pode ser cooperativo e precisa ser confirmado; resultado intermediário nunca assume status aprovado. Reinício do worker exige recuperação ou falha explícita. Após uma alteração no projeto, um job pode terminar para seu snapshot antigo, mas a interface e o MCP devem indicar a obsolescência.

Consultas de resultado retornam recortes e referências a arquivos, evitando exposição desnecessária de todo o projeto. Uma explicação do agente deve citar identidades reais de elementos, casos, combinações e verificações. Não inferir aprovação pela ausência de mensagens de erro em texto.

## 6. Segurança e autorizações

Perfis mínimos: consulta; edição; cálculo; preparação de documentos; exportação; emissão técnica. A autorização verifica o projeto e a operação em cada chamada, mesmo quando a ferramenta está visível ao cliente. Recursos, logs e caches não podem misturar projetos ou organizações.

Para HTTP, implementar os requisitos aplicáveis de autorização, audiência do token, armazenamento de credenciais e proteção do fluxo. Token recebido do cliente não deve ser repassado como credencial de outro serviço. [Segurança de autorização MCP](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization/security-considerations).

Aplicar ainda:

- Tratamento de IFC, BCF, PDFs, nomes de objetos e comentários como conteúdo não confiável.
- Restrição de arquivos e prevenção de acesso fora das raízes autorizadas.
- Limites de tamanho, tempo, quantidade, expansão de arquivos e custo computacional.
- Supressão ou mascaramento de dados sensíveis nos logs, excluindo segredos e minimizando conteúdo do projeto.
- Validação de tipo, enumeração, unidade, relações e valores antes de executar comandos.
- Autorização explícita do destino antes de enviar projeto a provedor externo.
- Testes de isolamento e de revogação, inclusive durante job e reconexão.

Um conteúdo importado não pode conceder permissão, ordenar envio de arquivos, alterar a política do projeto ou aprovar uma emissão. As credenciais de assinatura do responsável não ficam disponíveis ao agente.

## 7. Emissão técnica

Preparar documento, exportar prévia e emitir conjunto técnico são estados diferentes. A emissão exige identificação do responsável e aprovação autorizada do conjunto exato, ligado à revisão e aos hashes de resultados/documentos. Se esses dados mudarem, a aprovação anterior não se aplica ao novo conjunto.

A ferramenta recusa emissão com resultados obsoletos, análise incompleta, erro impeditivo, capacidade não validada para aquele domínio ou falta de aprovação exigida. Os critérios técnicos são definidos em [VALIDACAO.md](VALIDACAO.md); o mecanismo MCP não redefine esses critérios.

## 8. Aceitação da primeira operação e evolução

A issue #1 deve entregar servidor e interface próprios em Rust e demonstrar criação de projeto BIM real, objeto tipado mínimo, salvamento, reabertura e consulta, por interface e MCP. Reexecutar o mesmo comando nas duas superfícies deve produzir estado equivalente. Uma entrada inválida deve gerar diagnóstico sem corromper o projeto. Não usar cálculo ou objeto fictício para declarar conclusão.

Novas famílias de engenharia acrescentam ferramentas ou comandos ao contrato comum, com casos de equivalência. Gate de lançamento: versões e clientes declarados testados; zero mutação fora do escopo; nenhuma emissão obsoleta; recuperação de retries; isolamento de projetos; diagnósticos úteis; documentação de instalação que corresponda ao pacote distribuído.

Fontes e data de consulta: [fontes-arquitetura.json](fontes-arquitetura.json). Diretrizes de interação: [UX.md](UX.md).

## Ampliação para todas as instalações

O contrato de comandos cobre redes e ambientes além da estrutura. Consultar, lançar, conectar, desconectar, recalcular, explicar falhas, detalhar, extrair quantidades e preparar entregas são operações do mesmo serviço da interface. Ferramentas podem ser agrupadas por domínio para descoberta, preservando esquema, escopo e permissões comuns.

Operações em lote retornam uma prévia das conexões, escolhas de catálogo, conflitos, alterações de cálculo e documentos invalidados. A aprovação reutilizável respeita o escopo autorizado. Comandos carregam revisão esperada e chave de idempotência; repetição não duplica tubulações, fios, equipamentos ou documentos. A seleção de peça nunca aceita texto de catálogo como autorização para agir fora do projeto.

Os testes incluem um cliente real por família de operações, troca UI/MCP no mesmo projeto, falhas no meio de alterações, mudanças concorrentes e resultados que terminam depois de nova revisão. A issue 112 cobre o incremento estrutural; a completude ampliada é conferida nas etapas 227 a 250. Nenhuma operação relevante criada após a 112 fica dispensada de interface e MCP equivalentes.

O agente pode propor otimização de rotas, diâmetros, seções e equipamentos. Os resultados vêm dos motores determinísticos e de regras qualificadas, com restrições e objetivos explícitos. Uma explicação generativa não pode substituir diagnóstico, resultado numérico ou revisão técnica.
