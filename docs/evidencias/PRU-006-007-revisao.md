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

Após a inclusão de proveniência da face, a bateria MCP `modeling_tests` passou com **3 aprovados, 0 falhas**, usando `--offline --locked`. Ela verifica os payloads efetivos, hashes da base e do binário, expoentes JSON, unidades, negativas por escopo, ausência de mutação nas consultas, versões de catálogo, associação e remoção de entradas e reabertura. Área e inércias do retângulo são comparadas com fórmulas fechadas na prova do adaptador. A campanha numérica de torção e materiais pertence à PRU-010; este teste de integração não é apresentado como substituto dessa campanha.

A reexecução das quatro regressões de `prumo-desktop --test modeling` foi interrompida durante a ligação do binário de teste, antes de qualquer caso começar, para liberar o diretório de build à verificação completa do integrador. Não se registra aprovação desses quatro casos nesta revisão. Os ensaios com widgets da PRU-006 listados acima foram concluídos separadamente. O resultado final das novas regressões precisa ser conferido na matriz do conjunto.

## Conclusão delimitada

Os achados descritos foram corrigidos, o código foi reexaminado e as provas concluídas acima passaram. Não restou um desvio material na leitura dos caminhos examinados; a execução pendente dos quatro casos de interface continua necessária para liberar essa parcela. O retorno integral de `ProjectService` permanece uma API interna para o host confiável; clientes externos recebem os dados filtrados pelo adaptador MCP, conforme a fronteira documentada na [PRU-007](PRU-007.md).

A revisão não substitui formatação, Clippy, bateria completa, builds de distribuição nem conferência do commit final após a integração das PRU-008 a PRU-010. Também não afirma aprovação de CI remota, merge ou encerramento das issues.
