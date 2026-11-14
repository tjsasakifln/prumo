# PRU-002: inventário de cobertura e contrato MCP

## Entrega e limites

O catálogo instalado em [`crates/prumo-registry/data/catalogue.json`](../../crates/prumo-registry/data/catalogue.json) materializa as **72 famílias obrigatórias**, com **368 registros de capacidade** e **171 referências normativas ou de escopo**. A versão inicial do catálogo é `2026-11-14.1`, com schema `1`. Os 368 registros separam produto, módulo e operação; 38 possuem versão de referência expressamente identificada. Os demais mantêm `reference_version: null` e uma nota de pendência ou de requisito próprio.

Os registros são requisitos de evolução do Prumo ligados a fontes documentais. Todos permanecem `planned`, com `method: pending` e `qualification: null`. O catálogo não declara paridade atingida, cálculo implementado, aprovação profissional nem direito de redistribuir manuais de terceiros. As operações simples de modelagem já entregues na PRU-001 continuam disponíveis pelo serviço compartilhado; não liberam integralmente as famílias ampliadas.

As 171 referências usam `access: public_metadata_only`, `licensed_evidence: null` e `resolution: null`. Nenhum texto de norma foi incorporado ou declarado consultado. Há 43 registros com edição identificada na pesquisa de origem, 128 ainda sem edição; edição conhecida não significa vigência confirmada nem domínio técnico aprovado. As classes de evidência registram 16 entradas de catálogo, 18 institucionais, 18 de fabricantes e 119 não resolvidas.

A trilha de origem está em [`seed-provenance.json`](../../crates/prumo-registry/data/seed-provenance.json). Ela liga os textos de escopo do backlog aos IDs do cadastro, preserva as operações previstas por família e registra a consulta pública pontual realizada nesta implementação. Os dados derivam de [coverage.json](../../planning/coverage.json), [backlog.json](../../planning/backlog.json), [PARIDADE.md](../PARIDADE.md), [NORMAS.md](../NORMAS.md), [INSTALACOES.md](../INSTALACOES.md), [NORMAS-INSTALACOES.md](../NORMAS-INSTALACOES.md) e dos catálogos de fontes indicados nesses documentos. A pesquisa documental anterior continua identificada como origem; não foi apresentada como nova consulta ao texto normativo.

## Identidade e responsabilidade

Cada capacidade contém `family`, `product`, `module`, `reference_version`, `version_note`, `source_ids`, URLs, `owner_role`, issues de implementação, operação, disciplina, método, material e normas relacionadas. A capacidade de modelar é distinta da capacidade de calcular, verificar ou emitir. O sufixo do identificador expressa a operação; variantes documentais de produto ou versão têm IDs adicionais. Um mesmo produto não recebe automaticamente toda a capacidade descrita por outro produto da família.

O responsável é um papel de governança por disciplina, com indicação expressa de que a interpretação normativa requer profissional habilitado a designar. Não há nomes, registros profissionais ou aprovações fictícias no catálogo distribuído. `material: unspecified` mantém visível a lacuna quando o escopo depende de múltiplos materiais ou equipamentos. Esses marcadores impedem promoção técnica sem completar o domínio específico.

| Disciplina | Registros por operação/produto | Responsabilidade funcional |
|---|---:|---|
| Estruturas | 200 | Equipe responsável por estruturas; interpretação por profissional a designar |
| Hidrossanitária | 42 | Equipe responsável por hidrossanitária; interpretação por profissional a designar |
| Gás | 10 | Equipe responsável por gás; interpretação por profissional a designar |
| Incêndio | 20 | Equipe responsável por incêndio; interpretação por profissional a designar |
| Elétrica | 19 | Equipe responsável por elétrica; interpretação por profissional a designar |
| Fotovoltaica | 8 | Equipe responsável por fotovoltaica; interpretação por profissional a designar |
| SPDA | 12 | Equipe responsável por spda; interpretação por profissional a designar |
| Cabeamento | 8 | Equipe responsável por cabeamento; interpretação por profissional a designar |
| Climatização | 18 | Equipe responsável por climatização; interpretação por profissional a designar |
| Coordenação e plataforma compartilhada | 31 | Equipe responsável por coordenação e plataforma compartilhada; interpretação por profissional a designar |

As oito disciplinas de instalações têm registros próprios. Coordenação BIM, bibliotecas, documentos, quantitativos e interoperabilidade ficam identificados como funções compartilhadas, sem substituir disciplinas. A distribuição por operação é: **100 modelar, 77 calcular, 100 verificar e 91 emitir**. Não foram criados cálculos artificiais para funções puramente gráficas; a política de decomposição de cada família está preservada no arquivo de proveniência.

## Versões e limites documentais preservados

| Recorte documentado | Versão ou situação | Tratamento no inventário |
|---|---|---|
| CYPE 3D: barras somente à tração e levantamento de apoio, fonte CY-04 | Introdução em 2024.d | Perfil separado de modelagem, cálculo e verificação; sem inferir comportamento de outras versões |
| TQS: pacote Full LVP&S + PREO, fonte TQ-06 | Campo público de versão V26 | Aplicado aos dois perfis de família diretamente apoiados nessa página; referências históricas do texto continuam identificadas |
| PREO: painéis e insertos, fonte TQ-09 | Versão não identificada | Modelagem e emissão; o manual não comprova dimensionamento automático dos painéis |
| Builder: válvula redutora de pressão, fonte QH-041 | Desde 2024-05 | Perfil específico; o procedimento anterior de pressão imposta não comprova a rede inteira |
| Builder: distribuidores PEX, fonte QH-043 | Desde 2022-11 | Perfis separados para água e gás; cálculo de água não qualifica gás compressível |
| Builder: gerador e QTA, fonte QE-013 | 2025-05, com extensão em 2026-08 | Recursos e limitação de um QTA por gerador preservados; conexão em outro pavimento tem perfil próprio |
| Builder: subestação, fonte QE-014 | Artigo de 20/01/2026 sem release exata | Representação e emissão; a documentação informa ausência de dimensionamento dos elementos |
| Builder: SPDA e dados Ng, fontes QE-022 e QE-036 | 2026-08 | Perfil da edição normativa 2026 separado da documentação histórica de risco multizona |
| Builder: carga térmica por ambiente, fonte QE-033 | 2026-08 | Registros de modelagem, cálculo, verificação e emissão; sem presumir seleção ou aprovação automática de equipamentos |
| Builder: Split e VRF, fontes QE-032 e QE-035 | Documentação histórica sem release identificada | Modelagem e detalhamento; não exclui a carga térmica introduzida posteriormente |
| Builder: simplificação DWG/DXF e exportação seletiva IFC, fonte QP-027 | 2025-11 | Registros próprios com limites de preservação semântica ainda sujeitos à qualificação |
| Builder: conexões ranhuradas, fonte QH-028 | 2026-08 | Inventário da função; nenhum catálogo proprietário de peças foi copiado |

As fontes completas de instalações estão em [hidrossanitária](../fontes-instalacoes-hidrossanitario.json), [elétrica e disciplinas relacionadas](../fontes-instalacoes-eletrico.json) e [plataforma](../fontes-instalacoes-plataforma.json). A fonte [TQ-06](https://www.tqs.com.br/%C3%82/systems/tqs-preo/366i9tyvyu) foi aberta novamente em 14/11/2026: o caminho codificado é acessível e a página identifica V26. O mesmo texto contém descrição histórica de VPRO incorporado em V22 e referência à NBR 6118:2014. Essa coexistência foi registrada na nota de versão; não prova suporte atual a uma edição normativa. Nenhuma informação dessa página equivale a acesso autorizado ao texto da norma.

## Normas, partes e pendências

Os IDs são estáveis por documento e parte. A edição pertence ao snapshot do pacote, permitindo comparar a mesma norma entre versões sem sobrescrever o histórico. As partes documentadas foram individualizadas: NBR 7190, partes 1 a 7; NBR 5419, partes 1 a 4; NBR 15575, partes 1 a 6. Não há um registro agregado que esconda diferenças de edição entre essas partes.

Emendas e erratas públicas ficam em campos separados. O cadastro preserva, por exemplo, a referência documental à emenda da NBR 6118 de 11/03/2026, à versão corrigida de 2025 da NBR 8800 e às versões corrigidas por parte da NBR 5419 identificadas no inventário. A incorporação correta dessas alterações exige consulta autorizada e revisão, ainda pendentes.

Há pendências concretas adicionais: a NBR 6122 tem edição histórica de 2019 com consolidação de emenda a confirmar; a NBR 16401-3 tem edição histórica de 2024 e notícia institucional de suspensão em 2025; NBR 7198, 7229 e 13969 são referências legadas com transições a resolver. A classe da fonte descreve sua origem; `resolution: null` impede interpretar o ano preenchido como vigência aprovada. A auditoria emite `normative_resolution_pending` também para registros que já possuem edição.

Normas locais, instruções de Corpos de Bombeiros, concessionárias, regulamentos e catálogos cujo instrumento específico ainda não foi selecionado recebem referências de escopo `scope-*`. O cadastro preserva o texto exigido, a origem no planejamento e a responsabilidade pela resolução. Não se atribui uma jurisdição, edição, licença ou aplicabilidade com base em um portal genérico. O conjunto de documentos aplicáveis precisa ser identificado antes da qualificação do método.

`NormResolution` exige situação de vigência, evidências de catálogo, aplicabilidade e referências cruzadas, domínios aprovados e revisão de emendas/erratas. `LicensedEvidence` é distinto e registra o acesso efetivo ao documento. O comando público de cadastro aceita apenas metadados e sempre produz referência não resolvida, sem licença ou aprovação. Ele não permite preencher esses campos de qualificação por MCP ou interface.

## Pacote por projeto e migração

O projeto sem pacote fixado pode ser modelado e consultado, mas não obtém autorização para resultado final. A primeira fixação recebe o hash exato do candidato. O snapshot ativo guarda catálogo, edições, fontes e estados de capacidade. Abrir um projeto já fixado conserva esse snapshot, mesmo se o catálogo instalado ou o catálogo de trabalho tiver outra versão.

Cadastrar ou alterar uma referência cria versão local nova, relatório de alteração e candidato com outro hash. O pacote ativo permanece igual. A consulta prévia retorna revisão atual, hash ativo, pacote candidato e impacto entre os dois estados. O impacto identifica normas modificadas, capacidades afetadas e necessidade de recálculo; não altera o projeto.

A migração exige revisão atual, hash de origem, hash de destino e confirmação explícita. A operação preserva o pacote anterior no histórico, registra o impacto efetivo e marca recálculo necessário. Recusar a confirmação deixa o projeto intacto. A consulta prévia e a mutação passam pelo mesmo `ProjectService` usado pela interface.

O bloqueio de resultado final considera ausência de cálculo, falta de pacote, revisão ou pacote divergente, recálculo pendente e qualificação incompleta. Todos os registros distribuídos nesta etapa continuam bloqueados para resultados de engenharia finais. As fixtures de testes usam documentos e revisores explicitamente fictícios apenas para provar o mecanismo de estados; não integram o catálogo distribuído.

## Contrato MCP acrescentado

O processo continua limitado ao arquivo selecionado pelo host. A autorização de escrita vem de `--allow-write`, e o serviço impõe a permissão mesmo quando o cliente chama uma ferramenta omitida do catálogo somente leitura. Não há seleção de caminhos, comandos externos, SQL ou aprovação técnica pelo protocolo.

| Ferramenta | Conteúdo e efeito | Permissão |
|---|---|---|
| `prumo.registry.read` | Retorna catálogo, capacidades, fontes e referências | Leitura |
| `prumo.registry.audit` | Retorna cobertura, erros de integridade e pendências | Leitura |
| `prumo.norms.package` | Retorna candidato em `package`, `project_revision`, `active_hash` e `impact` previsto | Leitura |
| `prumo.results.gate` | Explica o bloqueio ou autorização de resultado da capacidade solicitada | Leitura |
| `prumo.norms.register` | Registra metadados no candidato, sem substituir o snapshot ativo | Escrita e revisão |
| `prumo.norms.pin` | Fixa pela primeira vez o candidato identificado por hash | Escrita e revisão |
| `prumo.norms.migrate` | Migra entre hashes confirmados e registra impacto | Escrita, revisão e confirmação |

O catálogo total tem 12 ferramentas em escrita e 5 em leitura. Os schemas e desserializadores recusam campos adicionais, inclusive campos aninhados de licença, aprovação e caminhos. Os hashes são SHA-256 em 64 caracteres hexadecimais minúsculos; as entradas permanecem limitadas a 64 KiB pelo transporte. A resposta pode incluir o catálogo completo. As características de negociação e as garantias do transporte permanecem descritas em [PRU-001-mcp.md](PRU-001-mcp.md).

## Evidências reproduzíveis

```sh
cargo test --locked -p prumo-registry
cargo test --locked -p prumo-app
cargo test --locked -p prumo-mcp-server
cargo clippy --locked -p prumo-mcp -p prumo-mcp-server --all-targets -- -D warnings
```

Os dez testes em `apps/prumo-mcp/tests/stdio.rs` executam o binário real em processo separado. Os novos casos verificam o inventário de 72 famílias e ausência de aprovações, autorização das três mutações normativas, recusa de licença ou aprovação forjada, fixação de pacote, preservação do pacote anterior ao registrar outra edição, encerramento do processo MCP e reabertura na interface antes de migrar, prévia de impacto sem mutação, confirmação recusada sem alteração, igualdade entre impacto previsto e efetivo, necessidade de recálculo e reabertura do histórico na interface `Desktop`. O teste da PRU-001 continua clicando na interface real via `egui_kittest`, encerrando a instância, alterando pelo MCP e reabrindo a interface.

A execução Linux final dos dez casos, incluindo a prévia, passou em 14/11/2026. A formatação dos dois pacotes MCP foi aplicada e o Clippy dos dois pacotes, com todos os targets e `-D warnings`, passou. A integração e a matriz de sistemas operacionais são registradas na evidência consolidada da issue. Os testes de domínio e registro confrontam o inventário com o planejamento e exercitam exclusão de família, falta de issue, norma inexistente, ausência de licença, vigência, referências cruzadas, domínio e aprovação. Esses testes demonstram restrições do software; não constituem qualificação profissional dos cálculos futuros.

## Reuso e manutenção

O adaptador amplia o `rmcp 3.10.0` já examinado na PRU-001, preservando as mesmas features e avisos. `serde 1.0.229` e `serde_json 1.0.154` são reutilizados para contratos tipados e rejeição de campos desconhecidos. O registro usa `sha2 0.11.1`, já fixado no workspace, para identificar o conteúdo dos snapshots. Não foi implementado um protocolo, parser JSON ou algoritmo criptográfico próprio. As origens oficiais são o [SDK MCP Rust](https://github.com/modelcontextprotocol/rust-sdk), [Serde](https://serde.rs/), [serde_json](https://github.com/serde-rs/json) e [RustCrypto SHA-2](https://github.com/RustCrypto/hashes/tree/master/sha2); as licenças e os arquivos efetivamente incorporados seguem o [inventário de componentes](../componentes/PRU-001.md) e `Cargo.lock`.

Não foi incorporado um catálogo proprietário de normas ou equipamentos. A alternativa de copiar uma base externa de regras foi rejeitada por ausência de direito de uso e de evidência de domínio aplicável. O inventário próprio combina IDs, metadados públicos e vínculos do planejamento; toda a lógica de validação, estados, serviço e transporte permanece Rust. As transformações administrativas usadas para preparar JSON não fazem parte do runtime.

## Relação completa das famílias

A tabela apresenta as 72 famílias e suas operações de base. Variantes por produto, módulo ou release continuam no JSON completo, mantendo os mesmos vínculos de família. As issues listadas são as do planejamento; registros de recursos específicos acrescentam seus vínculos correspondentes.

| Família | Escopo | Operações | Registros | Issues de implementação |
|---|---|---|---:|---|
| PAR-01 | Concepção e modelagem | modelar, verificar | 4 | 4, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 |
| PAR-02 | Modelo analítico | modelar, verificar | 2 | 4, 6, 9, 16, 18, 21, 24, 25, 40 |
| PAR-03 | Análise global | calcular, verificar, emitir | 6 | 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 95, 98 |
| PAR-04 | Ações e combinações | modelar, calcular, verificar, emitir | 4 | 31, 32, 33, 34, 35, 38, 39 |
| PAR-05 | Instabilidade e não linearidade | calcular, verificar, emitir | 3 | 16, 22, 23, 24, 26, 27, 30, 36, 45, 49, 73, 78, 83, 91, 94 |
| PAR-06 | Concreto armado | modelar, calcular, verificar, emitir | 4 | 37, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53 |
| PAR-07 | Lajes e pavimentos | modelar, calcular, verificar, emitir | 8 | 14, 23, 24, 25, 30, 37, 46, 47, 48, 53, 58, 66, 74, 87, 95 |
| PAR-08 | Elementos especiais | modelar, calcular, verificar, emitir | 8 | 13, 15, 22, 23, 24, 25, 42, 43, 45, 49, 50, 53, 86 |
| PAR-09 | Escadas e rampas | modelar, calcular, verificar, emitir | 8 | 15, 23, 25, 37, 48, 50, 53 |
| PAR-10 | Fundações | modelar, calcular, verificar, emitir | 8 | 61, 62, 63, 64, 65, 66, 67, 69, 76, 93 |
| PAR-11 | Interação solo-estrutura | modelar, calcular, verificar, emitir | 4 | 30, 61, 62, 63, 64, 66, 67, 93 |
| PAR-12 | Contenções e reservatórios | modelar, calcular, verificar, emitir | 4 | 31, 32, 39, 48, 50, 59, 61, 68, 69 |
| PAR-13 | Protensão | modelar, calcular, verificar, emitir | 8 | 30, 41, 42, 46, 48, 54, 55 |
| PAR-14 | Pré-moldados | modelar, calcular, verificar, emitir | 10 | 30, 50, 51, 52, 53, 54, 55, 57, 58, 69, 93, 101, 102, 104 |
| PAR-15 | Aço | modelar, calcular, verificar, emitir | 8 | 71, 72, 73, 75, 76, 77, 78, 79, 80, 93 |
| PAR-16 | Estruturas mistas | modelar, calcular, verificar, emitir | 4 | 30, 42, 71, 72, 73, 74, 75, 76, 79, 80, 93 |
| PAR-17 | Ligações metálicas | modelar, calcular, verificar, emitir | 4 | 16, 73, 75, 76, 77, 78, 79, 80, 93 |
| PAR-18 | Madeira | modelar, calcular, verificar, emitir | 4 | 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 93 |
| PAR-19 | Alumínio | modelar, calcular, verificar, emitir | 8 | 10, 22, 23, 94, 101, 102 |
| PAR-20 | Material genérico | modelar, calcular, verificar, emitir | 4 | 10, 22, 23, 40, 94 |
| PAR-21 | Alvenaria e paredes | modelar, calcular, verificar, emitir | 4 | 14, 15, 25, 31, 32, 91, 92, 93 |
| PAR-22 | Vento, dinâmica e sismo | modelar, calcular, verificar, emitir | 8 | 28, 29, 31, 32, 33, 34, 35, 36, 38 |
| PAR-23 | Incêndio | modelar, calcular, verificar, emitir | 8 | 31, 32, 39, 56, 79, 88 |
| PAR-24 | Desenhos e armaduras | modelar, verificar, emitir | 6 | 51, 52, 53, 57, 58, 69, 80, 90, 92, 101, 103, 106 |
| PAR-25 | Memoriais e quantitativos | modelar, calcular, verificar, emitir | 8 | 25, 40, 96, 102, 106 |
| PAR-26 | Interoperabilidade BIM | modelar, verificar, emitir | 6 | 4, 5, 18, 104, 105, 106 |
| PAR-27 | Coordenação | modelar, verificar, emitir | 6 | 18, 20, 51, 52, 53, 69, 92, 97, 105, 106, 107 |
| PAR-28 | Programação e agentes | modelar, calcular, verificar, emitir | 4 | 1, 6, 7, 19, 98, 99, 100, 108 |
| PAR-29 | Formatos e ecossistemas | modelar, verificar, emitir | 6 | 17, 18, 103, 104, 105, 107, 108 |
| PAR-30 | Operação simples e confiável | modelar, verificar | 2 | 1, 5, 6, 11, 12, 19, 20, 40, 108, 109, 110 |
| PAR-31 | Paredes de concreto | modelar, calcular, verificar, emitir | 4 | 15, 23, 24, 25, 31, 32, 37, 41, 48, 49, 53, 69, 93 |
| PAR-32 | Barras unilaterais e tirantes | modelar, calcular, verificar, emitir | 7 | 16, 22, 26, 27, 32, 73, 75, 77, 85 |
| PAR-33 | Cálculo isolado | modelar, calcular, verificar, emitir | 8 | 40, 42, 62, 65, 72, 99, 102 |
| PAR-34 | Perfis de alma aligeirada | modelar, calcular, verificar, emitir | 4 | 10, 13, 71, 72, 73, 80 |
| PAR-35 | Apresentação móvel | modelar, verificar, emitir | 3 | 104 |
| PAR-36 | Intercâmbio analítico | modelar, verificar, emitir | 3 | 108 |
| MEP-01 | Modelo físico, redes, portas e ambientes | modelar, verificar | 2 | 121, 123, 124, 125 |
| MEP-02 | Catálogos técnicos, kits e blocos dinâmicos | modelar, verificar, emitir | 5 | 122, 126, 235 |
| MEP-03 | Referências CAD/BIM, RVT/RTE e planificação | modelar, verificar, emitir | 5 | 229 |
| MEP-04 | Traçado assistido, encaixes e vistas técnicas | modelar, verificar | 2 | 127, 128 |
| MEP-05 | Água fria, demanda, perdas e pressões | modelar, calcular, verificar, emitir | 4 | 131, 132, 133, 134, 135 |
| MEP-06 | Reservação, medição, recalque e controle de pressão | modelar, calcular, verificar, emitir | 7 | 136, 137, 138, 139 |
| MEP-07 | Água quente, aquecimento e solar térmico | modelar, calcular, verificar, emitir | 4 | 140, 141, 142 |
| MEP-08 | PEX e montagem automática de ambientes | modelar, calcular, verificar, emitir | 7 | 143, 144 |
| MEP-09 | Piscinas e recirculação | modelar, calcular, verificar, emitir | 4 | 145 |
| MEP-10 | Esgoto, ventilação, declividades e caixas | modelar, calcular, verificar, emitir | 4 | 146, 147, 148 |
| MEP-11 | Elevatórias sanitárias e pluviais | modelar, calcular, verificar, emitir | 4 | 149 |
| MEP-12 | Chuva, captação, condutos e aproveitamento | modelar, calcular, verificar, emitir | 4 | 150, 151, 152 |
| MEP-13 | Tratamento local e disposição no solo | modelar, calcular, verificar, emitir | 4 | 153, 154 |
| MEP-14 | Gás, demanda, simultaneidade e pressão | modelar, calcular, verificar, emitir | 6 | 156, 157, 158 |
| MEP-15 | Regulação, medição e centrais de gás | modelar, calcular, verificar, emitir | 4 | 159, 160 |
| MEP-16 | Hidrantes, mangotinhos, cobertura e RTI | modelar, calcular, verificar, emitir | 4 | 162, 163, 164, 165, 166 |
| MEP-17 | Bombas de incêndio e jockey | modelar, calcular, verificar, emitir | 4 | 167, 168 |
| MEP-18 | Sprinklers, risco e áreas de operação | modelar, calcular, verificar, emitir | 4 | 169, 170, 171 |
| MEP-19 | Redes fechadas e sistemas conjuntos de incêndio | modelar, calcular, verificar, emitir | 4 | 172, 173 |
| MEP-20 | Extintores, detecção, alarme e emergência | modelar, calcular, verificar, emitir | 4 | 174 |
| MEP-21 | Modelagem elétrica, fiação e encaminhamento | modelar, calcular, verificar, emitir | 4 | 176, 177, 178, 179, 186 |
| MEP-22 | Quadros, demanda, condutores e proteções | modelar, calcular, verificar, emitir | 4 | 180, 181, 182, 183, 184, 185 |
| MEP-23 | Barramento, geração e equipamentos elétricos | modelar, calcular, verificar, emitir | 11 | 187, 188, 189 |
| MEP-24 | Fotovoltaico: arranjos, inversores e CC/CA | modelar, calcular, verificar, emitir | 4 | 191, 192, 193, 194, 195 |
| MEP-25 | Baterias, usinas, geração e análise econômica | modelar, calcular, verificar, emitir | 4 | 196, 197, 198, 199 |
| MEP-26 | SPDA: avaliação de risco | modelar, calcular, verificar, emitir | 8 | 201 |
| MEP-27 | SPDA: captação, descidas e aterramento | modelar, calcular, verificar, emitir | 4 | 202, 203, 204, 205, 206, 207 |
| MEP-28 | Dados, telefonia, TV, interfone e cabeamento | modelar, calcular, verificar, emitir | 4 | 209, 210, 211, 212 |
| MEP-29 | Racks, portas e mapas de cabos | modelar, calcular, verificar, emitir | 4 | 213, 214, 215 |
| MEP-30 | Ambientes, carga térmica e psicrometria | modelar, calcular, verificar, emitir | 8 | 217, 218, 219 |
| MEP-31 | Equipamentos e linhas frigorígenas | modelar, calcular, verificar, emitir | 6 | 220, 221 |
| MEP-32 | Dutos, água gelada e condensado | modelar, calcular, verificar, emitir | 4 | 222, 223, 224, 225 |
| MEP-33 | Integração de cargas, passagens e coordenação | modelar, calcular, verificar, emitir | 4 | 227, 228 |
| MEP-34 | IFC, revisões, BCF, IDS e mesclagem | modelar, verificar, emitir | 5 | 230, 231 |
| MEP-35 | Quantitativos, pranchas, memoriais e acervo | modelar, calcular, verificar, emitir | 4 | 232, 233, 234, 235 |
| MEP-36 | Interface/MCP, robustez e operação integrada | modelar, calcular, verificar, emitir | 4 | 236, 237 |
