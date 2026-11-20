# PRU-008 e PRU-009: revisão independente de software

Revisão cruzada realizada pelo agente responsável pelas transações, que não escreveu originalmente a infraestrutura de ensaios nem a geometria. Foram lidos os contratos, executores, referências, serviço compartilhado, testes e documentos. Este registro descreve verificações de software e de consistência matemática; não constitui assinatura profissional ou liberação de cálculo de projeto.

## Achados corrigidos

| Achado | Correção | Evidência executada |
|---|---|---|
| O build observava arquivos fixos dentro de `.git`, sem observar a referência de branch. Um commit sem mudança de código ou um checkout vinculado poderia conservar a identidade anterior no binário. | `build.rs` resolve HEAD, índice, referências compactadas e branch ativo com `git --git-path`. | `commit_only_changes_refresh_the_binary_in_regular_and_linked_worktrees` cria repositórios temporários, compila um programa com o build real, altera apenas o commit e confere a identidade recompilada. O teste preserva o timestamp do índice para ele não esconder o problema. Passou. |
| A identidade de base do ensaio aceitava projeto nulo ou revisão zero. | `RunContext::validate` exige UUID não nulo, revisão positiva e SHA-256 válido quando há base de projeto. | `a_snapshot_requires_a_real_project_identity_and_positive_revision` passou. |
| Um encontro da ponta de uma peça com o interior de outra não aparecia no relatório geométrico. | A interseção exata de ponta também gera ocorrência de contato, conservando a ausência de conexão explícita. | O teste reproduziu relatório vazio antes da correção; `an_endpoint_touching_another_members_interior_is_reported_without_connecting_it` passou depois, sem alterar o arquivo ou criar conexão. |
| A translação prévia à classificação geométrica apagava separações pequenas quando o primeiro segmento tinha origem remota. Dois segmentos separados eram classificados como sobrepostos. | O adaptador entrega as coordenadas originais aos predicados robustos de `geo`. A normalização numérica do cálculo da interseção fica a cargo da biblioteca depois da classificação. | Dois testes adversariais reproduziram a falsa sobreposição. `geometry_review.rs` confere a separação de 1e-8 m em presença de coordenadas de ±1e9 m, a permutação dos segmentos e a preservação exata da coordenada de contato. |
| Números em notação científica dentro dos enums de comando eram recusados pela representação intermediária de `serde_json` com precisão arbitrária. | Tolerâncias e coordenadas de correção decodificam `serde_json::Number`, exigindo resultado finito em `f64`. | A regressão reproduziu `invalid type: map, expected f64`. Testes cobrem texto JSON, passagem por `Value`, razão em UTF-8, recusa de infinito e prévia transacional persistida com reabertura. |

## Contratos conferidos

Na infraestrutura de ensaios, a referência e a execução identificam sua linhagem. Compartilhar o núcleo numérico bloqueia a evidência. A revisão vincula o hash do protocolo antes de executar; o executor recebe uma versão imutável. Resultados com sinal trocado, unidade diferente, alvo ausente, não finito ou hash de outro protocolo não passam. Falha crítica e ausência de caso obrigatório bloqueiam a capacidade verificada. Comparações categóricas são exatas e não herdam a folga de tolerância numérica.

As referências de viga, treliça e pórtico usam valores fechados separados do executor matricial. Foram conferidos os sinais, as dimensões e a energia do pórtico em L, incluindo o encurtamento axial da coluna. O caso singular exige rejeição, sem rigidez artificial. Os relatórios conservam `engineering_release=false`.

Na geometria, as tolerâncias de seleção, topologia, malha e análise permanecem distintas. A alteração de seleção não aumenta a faixa de contato topológico. Faces degeneradas, invertidas, não planas, autointersectantes ou com vértices repetidos são recusadas ou diagnosticadas. A detecção de contato não cria uma relação resistente. A prévia de reparo depende de projeto, revisão, snapshot, conteúdo e ator; troca de ator, reparo alterado, revisão posterior e repetição da correção são recusados.

O serviço de ensaios confere escopo de cálculo antes de executar e associa o relatório a um único snapshot. A abertura do desktop foi relida após a integração: o host local concede leitura, modelagem e cálculo explicitamente. Não há concessão de documentos nem confiança em permissões armazenadas no projeto.

## Rechecagens locais

- `cargo test -p prumo-verification -p prumo-storage --offline`: 18 testes de contrato e um teste de proveniência do build passaram em `prumo-verification`; os 29 testes de armazenamento e um teste de documentação também passaram.
- Os quatro testes de geometria do serviço e os quatro testes adversariais do serviço passaram, incluindo prévia alterada, autoria, reabertura, contato e elemento muito curto.
- `cargo test -p prumo-domain --test geometry --test geometry_review --test catalogue_review --offline --locked`: os sete testes geométricos e os três testes adversariais do domínio passaram. As duas contraprovas de catálogo do mesmo comando são registradas na revisão da PRU-010.
- A regressão separada de transações, armazenamento do serviço e permissões passou com 12, nove e cinco testes, respectivamente.

Os resultados acima pertencem ao checkout local em desenvolvimento, com alterações presentes. O identificador do commit integrado, o artefato distribuído e os gates de todas as plataformas pertencem à evidência final de integração. Não foram presumidos como executados nesta revisão.
