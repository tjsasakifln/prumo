# PRU-002: revisão independente do registro

**Data:** 14/11/2026. **Objeto:** implementação, catálogo, interface, MCP e dependências da segunda issue. A revisão foi realizada por agente distinto dos autores do núcleo, do adaptador MCP e da interface. Não constitui revisão profissional de regras de engenharia.

## Alcance da conclusão

A issue entrega o software de registro normativo, inventário e controle de capacidade. A aceitação desse software preserva as obrigações técnicas de [NORMAS.md](../NORMAS.md), [NORMAS-INSTALACOES.md](../NORMAS-INSTALACOES.md) e das issues de cálculo. A distinção decorre do escopo de [PRU-002](../../planning/issues/002.md), que exige detectar norma não resolvida e recusar qualificação sem fonte licenciada, em conjunto com o PRD, seção 11, sobre concluir fundações sem exigir motores futuros.

O registro não encerra a base técnica normativa. Acesso autorizado, aplicabilidade, vigência, emendas, erratas, referências cruzadas e interpretação profissional continuam pendentes por capacidade. Nenhuma obrigação foi removida de NORMAS.md. Essas pendências impedem a promoção a verificado ou liberado; o aplicativo também mantém ausente a evidência de cálculo exigida para resultado final.

## Conferência dos critérios

| Critério | Conferência independente |
|---|---|
| Abrangência e backlog | Comparados os IDs e os vínculos com `planning/coverage.json` e `planning/backlog.json`: 72 famílias, sem família omitida, com responsável funcional e issues de implementação. Não houve redução da união dos vínculos de implementação de cada família |
| Produto, módulo, versão e operação | Inspecionados 368 registros, separados em modelar, calcular, verificar e emitir; 38 identificam versão documental. As oito disciplinas de instalações estão presentes. Versão desconhecida permanece explícita |
| Metadados e texto consultado | As 171 referências distribuídas têm acesso somente a metadados públicos, sem evidência licenciada e sem resolução técnica. Das referências, 43 possuem edição identificada e 128 ainda não. Nenhuma edição conhecida recebe vigência ou aprovação presumida |
| Responsabilidade | Papéis de governança permanecem distintos de profissional habilitado e de aprovação. As pessoas e os registros profissionais sintéticos aparecem apenas nas fixtures de teste, identificadas como tal |
| Estados por domínio | A transição verifica método, material, hashes de todas as referências dependentes, acesso, resolução normativa, revisão, contraprova e evidência própria do artefato liberado. Promover uma combinação não promove outra capacidade da mesma família |
| Pacote imutável | O conteúdo fixado recebe SHA-256, preserva edição e estado e tem integridade verificada na leitura. A mudança do catálogo candidato não altera o pacote ativo. O projeto legado sem pacote continua sem pacote ao abrir |
| Migração | A prévia é somente leitura. Revisão, origem, destino e confirmação são conferidos na mutação. Recusa, revisão vencida ou hash divergente preservam o arquivo. O pacote anterior e o impacto efetivo permanecem no histórico |
| Impacto e recálculo | A comparação identifica normas e capacidades afetadas, inclusive por referências cruzadas transitivas. Alterar referência remove a qualificação dependente do candidato e exige recálculo após fixação ou migração |
| Auditoria de cobertura | Testes removem família, responsável, issue e referência normativa. Uma referência cruzada inexistente é detectada mesmo em capacidade planejada, antes de qualquer tentativa de qualificação |
| Interface e MCP | Foram inspecionadas e executadas as operações pelos dois adaptadores. Ambos usam `ProjectService`; a permissão somente leitura também impede chamadas diretas de ferramentas de escrita não anunciadas |

As contagens e os perfis documentais conferidos correspondem à [matriz de cobertura](PRU-002-cobertura.md). O catálogo inteiro continua planejado, com método pendente e sem aprovação. A modelagem mínima já entregue não equivale a liberar integralmente essas famílias.

## Pontos críticos corrigidos e conferidos

| Risco encontrado | Correção e evidência |
|---|---|
| O mesmo arquivo podia ser usado como referência independente sob outro nome | Comparação dos SHA-256 das evidências. O teste `the_same_artifact_with_a_different_url_is_not_an_independent_reference` recusa a tentativa |
| Edição preenchida e licença podiam ocultar pendência de vigência ou aplicabilidade | `NormResolution` exige situação, data válida, responsável, evidências de catálogo/aplicabilidade/referências cruzadas, domínios e revisão de emendas/erratas. Os casos NBR 6122 e NBR 16401-3 continuam explicitamente pendentes |
| Alteração de referência cruzada podia escapar do vínculo de aprovação e do impacto | O fecho transitivo entra nos hashes, na qualificação, na auditoria e na invalidação. O teste usa A, B e C com ciclo de retorno, muda C e comprova rejeição da prova antiga e impacto sobre a capacidade vinculada a A |
| O impacto era apresentado somente depois da confirmação | A interface compara versões antes da escolha. `prumo.norms.package` retorna revisão, pacote candidato, hash ativo e impacto previsto. O teste confronta esse relatório com a migração efetivamente persistida |
| Editar uma fonte ou domínio principal podia apagar vínculos secundários | O comando conserva fontes, domínios e IDs de origem secundários, com deduplicação. O ensaio altera a referência NBR 5410 e verifica preservação. O formulário informa a regra e a prévia exibe os vínculos |
| A digitação de emendas e erratas podia perder o separador entre itens | A interface mantém buffers textuais e só converte as listas no registro. O caso de interface digita o primeiro item, o ponto e vírgula e o segundo item em frames separados |

Também foram conferidas a nomeação acessível do seletor normativo, a limpeza do catálogo ao trocar de projeto e a continuidade do bloqueio de edição quando o caminho digitado não corresponde ao arquivo aberto. Esses controles evitam consulta ou alteração do projeto errado.

## Verificações executadas nesta revisão

No Linux, o revisor executou diretamente:

```sh
cargo test -p prumo-registry -p prumo-app --locked
cargo test -p prumo-registry -p prumo-desktop -p prumo-mcp-server --locked
cargo test -p prumo-desktop --test ui --locked real_ui_pins_registers_previews_and_explicitly_migrates_normative_package
python3 scripts/generate_notices.py --check
cargo deny --locked check advisories licenses sources
```

A rodada de serviço aprovou os seis casos normativos e os 13 casos de regressão de projeto. A rodada final dos demais componentes aprovou 12 testes do registro, seis da interface e dez do MCP em processo real. Esses resultados incluem preservação do projeto anterior, recusa de confirmação, diferença entre candidato e ativo, reabertura do Desktop e ausência de aprovação fabricada pelo protocolo. Os testes de interface operam widgets por eventos; não se limitam a chamar comandos de serviço diretamente.

A versão final do teste de listas, com entradas em frames separados, foi executada novamente pelo revisor e passou. Os comandos gerais de build, formatação, Clippy, WASM, inspeção nativa, planejamento e empacotamento são registrados na [evidência consolidada](PRU-002.md). A revisão não atribui execução própria desses controles quando foram realizados pelo responsável pela integração.

O inventário de licenças e avisos foi regenerado e conferido. Não houve adição ou atualização de dependência externa: `sha2 0.11.1` passou de transitiva a direta. A auditoria retornou `advisories ok, licenses ok, sources ok`, com `cargo-deny 0.20.2` e RustSec no commit `4ef2df8e806a65e2f1ecc14f56f51249062b23d6`. Permanecem somente as três exceções informativas de manutenção já documentadas na PRU-001, com reavaliação prevista na PRU-003. A [ficha PRU-002](../componentes/PRU-002.md) registra origem, licença, escolha e limites.

O cache acrescentado ao CI usa ação oficial fixada por commit, separa sistema operacional, compilador e lockfile e mantém a execução de build e testes com `--locked`. Foi conferido como otimização de compilação; um cache encontrado não substitui qualquer verificação.

## Parecer e limites de uso

A revisão do software foi aprovada, sem apontamento funcional obrigatório em aberto nos critérios examinados. O aceite de integração depende dos controles da PR e da matriz Windows/Linux executados sobre o commit final, conforme o critério de conclusão do repositório. Esta revisão não declara esses checks aprovados antecipadamente.

O catálogo é utilizável para consulta, cadastro de referência pública e fixação explícita de metadados por projeto. Não contém regras técnicas qualificadas, fonte licenciada consultada, responsável profissional designado nem motor capaz de produzir resultado de engenharia. A conclusão da PRU-002 não dispensa essas obrigações nas capacidades correspondentes nem no lançamento completo.
