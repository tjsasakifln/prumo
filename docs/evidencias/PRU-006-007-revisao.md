# Revisão cruzada da PRU-006 e PRU-007

Revisão de implementação feita na branch de trabalho, com agentes separados responsáveis por transações e por MCP/autorização. Esta revisão examina contratos, persistência, isolamento e comportamento reproduzível. Não é revisão profissional de engenharia, assinatura ou aprovação normativa.

## PRU-006 examinada pelo agente de MCP/autorização

Foram lidos os contratos em `prumo-domain/src/transactions.rs`, o preparo, aplicação, repetição, persistência de recibos e restauração em `prumo-app/src/transactions.rs`, as consultas de revisão/journal e as verificações de integridade em `prumo-storage/src/store.rs`, além dos controles de lote em `prumo-ui/src/transactions.rs` e dos encaminhamentos no desktop.

Pontos conferidos:

- Pedidos recusam campos adicionais e não transportam ator, concessões ou caminhos. O serviço usa a identidade do host.
- Prévia prepara IDs e estado em arquivo interno limitado. Aplicação confere hash, versão, ator, projeto, revisão e base antes da publicação. O cliente fornece a identidade do plano, não um estado arbitrário para substituir o modelo.
- Recibo e estado pertencem ao mesmo commit do armazenamento. A repetição conserva o recibo original e devolve o projeto atual; a interface não recebe um estado antigo como se fosse o arquivo atual.
- A mudança de revisão é verificada sob o lock de escrita. O desktop conserva UUID e hash da base exibida, confere o resultado salvo e retira o editor se outro processo alterar o arquivo entre o commit e a leitura final.
- Desfazer/refazer cria novas revisões, restaura entidades e unidades, preserva auditoria e conserva o grafo atual de resultados. Mudanças de fontes tornam as evidências afetadas obsoletas. Inspeções entre desfazer e refazer continuam auditáveis.
- O journal valida encadeamento, hashes, identidade, autoria e proveniência das revisões recuperadas. Registros antigos ou de recuperação delimitam a pilha, sem inventar comandos reversíveis.

**Achado corrigido:** a pré-condição `sha256: Option<String>` aceitava omissão pela regra padrão do Serde, embora o contrato diferenciasse explicitamente hash e `null`. O implementador acrescentou desserialização obrigatória e a contraprova de omissão. `null` conserva o significado de exigir ausência da fonte.

Uma rechecagem examinou a prioridade dos erros nas regressões de armazenamento: `load_editor_basis` confere a base exibida antes do UUID para informar `storage_conflict`, mas só entrega o arquivo ao comando depois de conferir a autorização. Sem base de editor, a sessão conserva `permission_denied` na troca de UUID. Na criação, cópias pendentes e versões futuras bloqueiam a ação antes de vincular a identidade do novo projeto.

Ensaios reexecutados pelo revisor:

| Comando | Resultado |
| --- | --- |
| `cargo test -p prumo-app --test access_scopes --test transactions --locked` | 5 testes de acesso e 12 de transações aprovados |
| `cargo test -p prumo-mcp-server --test stdio transaction_tests --locked` | 2 testes com processo MCP real aprovados |
| `cargo test -p prumo-desktop --test transactions --locked` | 1 fluxo com widgets reais egui aprovado |

O fluxo de widgets prepara barra e ambiente, confere a prévia, aplica o lote, desfaz, refaz e reabre. As asserções comparam identidades, modelo, revisão e hash do snapshot. Os ensaios de processo cobrem reinício/repetição, preservação dos resultados aplicados, falha no meio do lote e recusa de plano obsoleto. Não se atribui a estes ensaios a inspeção de uma janela nativa ou uma campanha nova de interrupção abrupta do processo; essas parcelas pertencem à matriz de integração e às provas de armazenamento.

## PRU-007 examinada pelo agente de transações

O revisor conferiu o binding de UUID nas leituras e no armazenamento, os controles do serviço, a projeção de dados e extensões nas respostas e a resolução de URIs. Informou um defeito funcional no controle de efeitos indiretos: IDs de vínculos físicos/analíticos e de conexões eram tratados como entidades globais desconhecidas. Assim, a criação de relações permitidas podia ser recusada em uma sessão parcial.

**Correção aplicada:** a resolução passou a verificar as disciplinas dos participantes. Uma regressão cria uma barra Structure com vínculo analítico automático e uma rede Electrical com portas e conexão sob concessões parciais. Os cinco testes `access_scopes` passaram após a alteração.

A prova independente de JSON/stdio também encontrou e motivou a remoção explícita dos campos modernos de cache no adaptador legado. Essa descoberta veio dos ensaios do implementador de MCP, não é atribuída ao outro revisor. A rodada final local de `stdio access_tests` aprovou cinco testes, incluindo ambos os perfis, recursos e troca do arquivo durante uma sessão ativa.

## Revisão adicional da integração PRU-008 a PRU-010

O agente de MCP/autorização examinou as consultas em `prumo-app/src/modeling.rs`, schemas e encaminhamentos MCP em `extensions.rs`, `model_schema.rs` e `catalogue_schema.rs`, os formulários `prumo-ui/src/geometry.rs` e `catalogue.rs` e seus handlers no desktop. As consultas completas exigem todas as disciplinas. A prévia de reparo exige Model; a bateria de referência e as propriedades de seção exigem Calculation. O serviço usa a revisão lida para construir a evidência, preserva origem/unidades e não grava o projeto durante consultas. Os campos editáveis recebem rótulos acessíveis; as correções não exigem arrastar elementos.

| Achado de revisão | Correção conferida no código |
| --- | --- |
| Face sem identificação da política que decidiu sua validade | `ProjectFaceInspection` inclui UUID, revisão, snapshot, tolerâncias e coordenadas, além dos resultados geométricos |
| Resultado anterior permanecia após editar JSON da face ou condições do material, inclusive depois de uma consulta falhar | Formulários descartam o relatório ao editar; handlers removem o anterior antes de consultar |
| Texto numérico inválido podia conservar uma prévia válida, pois o campo mantinha seu último número | Erros numéricos descartam a prévia e impedem aplicar a correção |
| Filtros ou tolerâncias em edição podiam deixar um diagnóstico antigo aparente | Mudança de filtro limpa a auditoria; tolerâncias diferentes das salvas impedem auditoria e reparo até salvar |
| O glifo de seta no texto de comprimento impedia a renderização da prévia na fonte padrão do ensaio | O texto passou a usar “de ... m para ... m”, conservando os valores anterior/posterior e nove casas decimais; o controle de fontes e as verificações do teste foram preservados |

Após a inclusão de proveniência da face, a bateria MCP `modeling_tests` passou com **3 aprovados, 0 falhas**, usando `--offline --locked`. Ela verifica os payloads efetivos, hashes da base e do binário, expoentes JSON, unidades, negativas por escopo, ausência de mutação nas consultas, versões de catálogo, associação e remoção de entradas e reabertura. Área e inércias do retângulo são comparadas com fórmulas fechadas na prova do adaptador. A campanha numérica de torção e materiais pertence à PRU-010; este teste de integração não é apresentado como substituto dessa campanha.

A tentativa inicial do revisor foi interrompida durante a ligação do binário de teste para liberar o diretório de build ao integrador. Na execução posterior, o integrador identificou a falha de renderização pelo glifo de seta descrita acima. O revisor releu a substituição do texto e conferiu o log final `ui-final.log`, produzido por `cargo test --workspace --test modeling --locked`: **4 aprovados, 0 falhas e 0 ignorados**, em **8,73 s**, após **10,05 s** de compilação.

Os quatro casos aprovados foram `invalid_numeric_text_discards_a_previously_valid_geometry_plan`, `editing_material_conditions_discards_the_previous_state`, `section_widgets_create_analyse_assign_and_reopen_catalogue_with_provenance` e `reference_verification_and_geometry_queries_never_edit_the_open_project`. A regressão numérica alcançou a prévia renderizada e confirmou o descarte do plano quando o texto se tornou inválido. Não houve ampliação de tolerância nem desativação da verificação de fontes para obter esse resultado. Os ensaios com widgets da PRU-006 listados acima foram concluídos separadamente.

A regressão completa detectou ainda que o desktop conservava o editor antigo quando a sessão vinculada ao projeto A recusava o projeto B colocado no mesmo caminho. O serviço e o MCP mantiveram a recusa correta. A correção descarta o editor e obtém diagnóstico em uma sessão independente somente de leitura, no caminho já selecionado. Essa sessão não substitui a de edição; apenas **Reabrir / atualizar**, uma ação explícita do host, cria novo vínculo. O estado de expansão do painel é preservado, mas confirmações e evidências antigas são descartadas. O agente de transações releu essa fronteira sem indicar afrouxamento de autorização. Os seis testes antigos de armazenamento do desktop passaram no workspace após a correção, sem alterações nos testes.

## Regressões encontradas na bateria completa

O log `workspace-complete.log` registrou **23 aprovações e 5 falhas** nos 28 testes do cliente MCP. A leitura dos trechos de falha separou quatro expectativas anteriores à ampliação do contrato e uma regressão efetiva na prioridade dos erros de comandos unitários.

| Teste ou contrato | Ajuste conferido |
| --- | --- |
| Inventário somente de leitura | A posição inicial de `prumo.project.read` deixou de ser requisito. O teste compara o conjunto explícito de ferramentas de Read, exige nomes únicos, argumentos fechados e indicação de leitura em cada ferramenta. Ferramentas de Model, Calculation e Documents continuam ausentes. As chamadas diretas de escrita continuam exigindo `permission_denied`. |
| Recuperação após objetos com chave numérica reservada | A contagem antiga de 22 ferramentas foi substituída pela conferência dos nomes permitidos em Read e Model e pela comparação integral do catálogo antes e depois de cada mensagem rejeitada. Foram mantidos os três formatos de chave hostil, o código de erro, os bytes do arquivo, o estado do projeto e a prova de que o texto literal da chave pode ser um nome válido. |
| Versão desconhecida e metadados omitidos | A contagem antiga de 10 ferramentas foi substituída pelo mesmo contrato nominal de Read e pela comparação integral antes e depois dos erros. Os códigos `-32022` e `-32602` continuam exigidos, e o teste também confere que o arquivo não foi criado. |
| Objetos do contrato de edição | A recursão continua recusando objetos livres. A exceção é um mapa com limite de 256 entradas, cujos valores são objetos `PhysicalValue` fechados: campos e campos obrigatórios exatos, grandeza/unidade/base enumeradas e decimal textual limitado. O conjunto explícito das 30 variantes substitui a contagem antiga de 25, preservando todos os nomes anteriores e incluindo geometria e catálogo, com unicidade dos identificadores. |

Na regressão efetiva, `rename_project` com revisão zero chegava ao validador do envelope transacional antes da comparação com a revisão corrente e retornava `invalid_input`. O agente de transações corrigiu `execute_single`: depois de exigir escrita e carregar a base vinculada sob o lock, `check_command` confere a autorização e `check_basis` decide o conflito antes de construir o envelope. O agente de MCP/autorização releu essa ordem e os novos testes. UUID alheio, ausência de escrita e disciplina não autorizada continuam prevalecendo sobre detalhes de revisão. O contrato de lote e a repetição idempotente não foram alterados; a asserção antiga do cliente independente que exige `revision_conflict` foi preservada.

O agente de transações revisou os ajustes dos testes MCP e comparou a exceção dos mapas com o contrato de produção, sem indicar novo achado. Conferiu a preservação dos códigos de erro, bytes, modelo, campos que exigem `null` explícito e fluxo com cliente real. A recursão examinada é aplicada ao contrato de `model.edit`; outros mapas tipados, como o de condições de material limitado a 32 entradas, não são liberados implicitamente por essa exceção.

O integrador executou as duas regressões `command_conflicts` com `--locked`; o revisor conferiu `command-conflicts.log`: **2 aprovados, 0 falhas e 0 ignorados**, em **0,03 s**, após **5,28 s** de compilação. Nesta atualização, os ajustes dos quatro testes MCP foram relidos e formatados, mas sua reexecução pertence à nova bateria global coordenada pelo integrador. Não se atribui aprovação a essa rodada ainda em andamento.

## Conclusão delimitada

As correções descritas foram aplicadas e o código foi reexaminado. As campanhas individuais com resultado positivo estão identificadas acima, incluindo os quatro casos de interface e as duas regressões de prioridade de erros; a aprovação da nova rodada completa permanece pendente. Nos trechos reexaminados, não foi identificado desvio material remanescente. O retorno integral de `ProjectService` permanece uma API interna para o host confiável; clientes externos recebem os dados filtrados pelo adaptador MCP, conforme a fronteira documentada na [PRU-007](PRU-007.md).

A revisão não substitui formatação, Clippy, bateria completa, builds de distribuição nem conferência do commit final após a integração das PRU-008 a PRU-010. Também não afirma aprovação de CI remota, merge ou encerramento das issues.
