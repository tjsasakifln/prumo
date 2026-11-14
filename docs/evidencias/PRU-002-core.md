# PRU-002: registro, pacotes normativos e bloqueios

## Implementação

O contrato tipado fica em `prumo-domain::registry`; `prumo-registry` carrega e confere o inventário embarcado, registra versões, calcula impactos e aplica os bloqueios. `prumo-app` expõe as mesmas consultas e alterações usadas pela interface e pelo MCP. Nenhuma dessas operações executa cálculo estrutural ou de instalações.

O catálogo contém 72 famílias, decompostas em 368 capacidades por função e contexto documental, e 171 referências normativas. O teste de cobertura confronta as famílias com `planning/coverage.json`, verifica as issues em `planning/backlog.json` e preserva a união dos vínculos de implementação. Os dados e a procedência são mantidos em `crates/prumo-registry/data`.

As 368 capacidades embarcadas permanecem `planned`. As 171 referências permanecem `public_metadata_only`, sem evidência de texto licenciado consultado e sem resolução técnica aprovada. Os papéis responsáveis pelo inventário não são apresentados como pessoas habilitadas que já revisaram as normas.

## Metadados, acesso e qualificação

Cada norma registra identificação, parte, edição quando conhecida, emendas, erratas, fontes, classe de evidência, domínios, responsabilidade e situação do acesso. O tipo `LicensedEvidence` guarda referências de autorização e consulta, nunca o texto protegido nem credenciais.

`NormResolution` distingue vigência, aplicabilidade e revisão das referências cruzadas. Uma resolução precisa de data válida, responsável, evidências identificadas, domínios aprovados e conferência de emendas e erratas. Isso mantém explícitas pendências como consolidação de uma emenda ou vigência de uma parte com histórico de suspensão, mesmo que uma edição tenha sido identificada publicamente.

O estado pertence à combinação identificada de capacidade, método, material e referências normativas. A progressão normal é `planned`, `experimental`, `verified` e `released`. Verificação exige implementação identificada, relatório de testes, referência independente, consulta licenciada, resolução normativa e aprovação do domínio exato. Liberação exige ainda versão e hash do artefato liberado. Rebaixamentos invalidam a qualificação anterior.

As evidências de teste e de referência independente não podem apontar para o mesmo hash, mesmo com nomes ou endereços diferentes. A aprovação inclui os hashes de todas as normas dependentes, inclusive referências cruzadas transitivas. Um conjunto visitado trata ciclos sem travamento nem perda de dependências.

As transições qualificadas são operações internas de manutenção de registros revisados. Não há ferramenta MCP para fabricar aprovação, licença ou liberação. O cadastro disponível ao usuário aceita exclusivamente metadados públicos e sempre remove a possibilidade de herdar uma qualificação da referência substituída.

## Fluxo real de alteração normativa

1. A consulta apresenta o catálogo e o pacote candidato com seu hash SHA-256.
2. O usuário fixa explicitamente um pacote no projeto, informando a revisão e o hash que conferiu.
3. O cadastro de uma edição ou referência nova cria outra versão do catálogo local e grava um relatório de impacto. O pacote ativo permanece intacto.
4. A prévia fornece, em uma única leitura, revisão do projeto, hash ativo, pacote candidato e impacto completo entre ambos. Duas alterações pendentes aparecem juntas nessa comparação.
5. A migração exige confirmação explícita, revisão esperada, hash de origem e hash de destino. Um desses valores obsoleto impede a alteração.
6. A migração preserva integralmente o pacote anterior, grava o impacto e exige recálculo. A edição antiga continua recuperável no histórico.

O cadastro do formulário altera fonte e domínio principais, preserva os demais e mantém os identificadores de procedência. Os mapas são deduplicados. Uma alteração em uma referência usada indiretamente também invalida as capacidades dependentes.

Projetos anteriores sem pacote continuam sem pacote ao abrir. A desserialização não consulta o registro atual nem escolhe uma norma automaticamente. Pacotes existentes são validados pelo seu conteúdo e hash; a reabertura não os reconstrói a partir de edições atuais.

## Bloqueio de resultados finais

A consulta de liberação recusa capacidade incompleta, não liberada, fora do domínio, pacote incompatível e resultado desatualizado. O serviço atual sempre informa ausência de resultado calculado, pois os motores ainda pertencem a etapas futuras. O cliente não fornece um resultado ou uma aprovação no pedido dessa consulta.

Os casos em que o avaliador interno aceita uma combinação completa usam exclusivamente fixtures sintéticas em testes. Elas não estão no catálogo do produto e não representam aprovação profissional, interpretação de norma ou resultado de engenharia.

## Evidências executadas

```sh
cargo test -p prumo-domain -p prumo-app -p prumo-registry --locked
cargo clippy -p prumo-domain -p prumo-app -p prumo-registry --all-targets --locked -- -D warnings
```

Foram aprovados 34 testes dessas crates: 3 de domínio já existentes, 13 do serviço inicial, 6 do novo fluxo normativo e 12 do registro. A análise estática das três crates também foi aprovada. Os ensaios adicionais cobrem:

- Compatibilidade com projeto legado sem pacote e ausência de atualização silenciosa.
- Cadastro real, prévia, confirmação e migração, incluindo reabertura e preservação da edição original.
- Revisão obsoleta, pacote candidato obsoleto, consulta sem alteração e sessão somente leitura.
- Alteração do conteúdo sem atualizar o hash e recusa de campos de licença no cadastro público.
- Correspondência integral das famílias e vínculos de implementação, sem tratar pendências como capacidades liberadas.
- Recusa de fonte pública, resolução incompleta, domínio diferente, licença ausente e referência independente com hash repetido.
- Fecho de referências `A → B → C → A`, mudança transitiva, prova antiga incompatível e referência cruzada inexistente ainda no estado planejado.
- Preservação de fontes, domínios e procedência secundários na atualização de uma referência existente.
- Ausência de promoção implícita de outras capacidades da mesma família.

Os ensaios completos da interface e do cliente MCP são registrados na integração da PRU-002.

## Reuso e revisão

O contrato reutiliza Serde e serde_json, já triados na PRU-001. O cálculo de integridade usa [RustCrypto SHA-2](https://github.com/RustCrypto/hashes/tree/master/sha2), versão 0.11.1 fixada, com licença MIT ou Apache-2.0 declarada na distribuição Cargo. Não foi implementado um algoritmo criptográfico próprio. A serialização canônica ordena os registros e o teste confere identidade do pacote após permutação e reabertura.

A revisão independente corrigiu a comparação de evidências somente por endereço, acrescentou a resolução normativa explícita, ampliou hashes e impactos às referências transitivas e detectou perda de fontes e domínios secundários no formulário inicial. Os testes correspondentes foram incluídos e executados após as correções.

A consulta e a interpretação dos textos normativos continuam pendentes para as capacidades de engenharia correspondentes. O registro conserva essas pendências e impede que seu preenchimento apenas documental seja confundido com cálculo verificado.
