# PRU-004: revisão independente do modelo semântico

**Data:** 14/11/2026. **Escopo:** implementação da issue 4, após integração e verificação da PRU-003. A revisão examinou requisitos, pesquisa de componentes, código Rust, testes, contrato MCP, estado dos formulários, migração, documentação e obrigações das dependências. Não antecipou trabalho da PRU-005.

**Parecer sobre os arquivos revistos:** favorável à integração, após as correções descritas abaixo. Não foi identificado bloqueante de implementação remanescente nesse estado. A inspeção da interface nativa foi concluída pelo responsável pela integração e consta da [evidência consolidada](PRU-004.md). A conclusão operacional da issue ainda exige a rodada final do workspace e os checks Windows/Linux do commit publicado. Esses portões não são substituídos por este parecer.

## 1. Critérios confrontados com a implementação

| Critério da issue e dos contratos transversais | Comportamento e evidência conferidos |
|---|---|
| Físico e analítico separados, com relações 1:n e n:1 | `PhysicalObject` e `AnalyticalMember` têm UUIDs próprios. `PhysicalAnalyticalLink` substitui o antigo campo singular e permite ambas as cardinalidades. O domínio recusa pares repetidos, entidades inexistentes e vínculo de objeto somente visual. Testes preservam o conjunto na reabertura |
| Objeto visual não aparece como dimensionado | `PhysicalRole` é explícito. Criar um objeto visual não gera nós, barras ou vínculos. O estado dos objetos permanece `modelled`; inspeções e respostas MCP não concedem aprovação. A criação rápida de peça declara a barra vinculada que produz |
| Materiais e ações sem perda de significado | Módulo elástico, massa específica, coeficiente de Poisson, força, binário livre, nó de aplicação e referencial são campos próprios. Materiais são dados declarados, sem classe normativa presumida. Força e momento têm entradas e tipos distintos |
| UUID, GUID IFC, origem e derivação | Codec confrontado com vetor oficial externo. Modelo confere identidade, sintaxe e unicidade. Cópia, fusão e divisão produzem novas identidades; pais, filhos, ator, revisão e entidades retiradas permanecem rastreáveis. Dados ausentes do legado não recebem autoria inventada |
| Grandezas tipadas e conversões explícitas | Tipos privados do domínio, construtores e desserialização validada usam uom. Foram conferidos os nove grupos básicos previstos pela arquitetura, além de pontos, densidade e intervalo de temperatura. Os DTOs com unidade variável exigem unidade; campos SI fixos a declaram no contrato |
| Orientação, translação e unidade conservam a grandeza física | Contraprovas fechadas distinguem movimento ativo, mudança de referencial e momento em relação a um ponto. Matrizes com escala ou reflexão são recusadas como rotação. Permutar a ordem de armazenamento conserva o resultado; inverter extremos de uma barra conserva seu significado direcional próprio |
| Origem de engenharia separada do mapa | O referencial de engenharia possui origem e rotação próprias. Dados de georreferenciamento são separados e não alteram coordenadas de engenharia. O contrato registra metadados cartográficos, sem apresentar projeção geodésica como executada |
| Ambientes, redes, portas, conexões e procedência | Portas têm disciplina e tipo; conexões exigem portas distintas atribuídas à rede compatível. Coincidência ou cruzamento geométrico não produz conectividade. Cópia física não clona ações ou ligações de engenharia silenciosamente |
| Snapshot canônico e dependências | Snapshot versionado conserva conteúdo canônico, identificadores e índice de fontes incluído no conteúdo protegido pelo hash. As bases históricas são deduplicadas por hash e retidas. O grafo valida fontes, resultados pais, hashes, estados, revisões, ciclos e causas de obsolescência |
| Mudança relevante invalida dependentes | O serviço compara fontes antes/depois de cada mutação e propaga a invalidação aos descendentes. Alterar uma peça invalida sua medida e o resumo, preservando a medida independente de outra peça. Adicionar entidade invalida o resumo da coleção. Alterar material alcança os contratos de evidência que dele dependem |
| Round trip nativo sem perda | Ensaios passam pelos tipos, por bytes JSON e pelo arquivo real. A migração v1 ocorre em memória, preserva identidades existentes e deriva de forma repetível os nós/vínculos novos. Antes da primeira mutação, o serviço preserva cópia exata do original em arquivo de nome derivado do hash |
| Interface e MCP usam uma autoridade compartilhada | As 25 variantes de `ModelEdit` convergem no mesmo serviço e domínio. Inspeção, consulta de snapshot e consulta de dependências existem nas duas superfícies. A interface usa formulários, seletores e unidades; JSON não é o fluxo necessário de autoria |
| Erros não alteram o projeto | Unidade incompatível, referência órfã, campo extra, revisão antiga, transformação inválida, ausência de permissão, atualização incompleta e backup incoerente são recusados. Os testes comparam o projeto e os bytes salvos antes/depois dessas recusas |
| Reuso, segurança e distribuição | uom e alternativas foram examinados em fontes oficiais e pacotes efetivos. Avisos de uom e da parcela lexical de serde_json estão preservados. O inventário e a auditoria passaram no lock examinado, com as exceções específicas herdadas explicitadas |

Os critérios são atendidos como fundação de modelagem e rastreabilidade. Não se atribui a uma inspeção de comprimento o significado de análise estrutural, dimensionamento ou aprovação normativa. A fixture de evidência dependente de material está identificada como fixture e não é ferramenta pública de cálculo.

## 2. Achados e correções conferidas

### 2.1 Fidelidade numérica ao persistir JSON

A configuração padrão de serde_json 1.0.154 não preservava todos os bits dos números ao serializar e reler texto. Isso era relevante tanto para a ida e volta nativa quanto para a validação dos hashes de snapshots. Testes que passam apenas por `serde_json::Value` não detectam esse defeito, pois não exercitam o parser textual.

A revisão executou um [ensaio Rust reproduzível](PRU-004-serde-ensaio.rs), com 100.000 valores determinísticos em `[1, 2)`. Foram observadas 19.757 divergências sem `float_roundtrip` e nenhuma na mesma amostra com a feature. A leitura independente por `str::parse::<f64>()` preservou todos os valores gerados.

O vetor `1.9109059804556539`, bits `0x3ffe9312263c9b07`, voltava como `0x3ffe9312263c9b08`. A configuração do workspace agora habilita `float_roundtrip`. A regressão do domínio confere posição, força e momento por `to_vec`/`from_slice`; a regressão do serviço salva, reabre, compara os bits e revalida a evidência canônica. Ambas passaram. Não houve arredondamento de dados ou ampliação de tolerância para contornar o defeito.

A feature ativa código lexical interno com autoria específica. O comentário de atribuição a Alexander Huszagh, sob MIT/Apache-2.0, foi incluído nos avisos pela extração já existente. O acréscimo não introduziu pacote externo, fork ou trecho copiado para código próprio.

### 2.2 Base histórica e índice de fontes

O rascunho do grafo guardava apenas o hash de origem de cada resultado. A base completa foi incorporada em uma coleção de snapshots únicos, permitindo consultar os dados históricos mesmo após retirar a entidade física.

Na revisão seguinte, o índice `snapshot.sources` ainda era paralelo ao conteúdo canônico protegido pelo hash. O conteúdo passou a incluir `source_digests`, e a validação exige igualdade exata com o índice exposto. Alterar somente esse índice é agora recusado. Unidade de apresentação também integra o snapshot, evitando que metadados diferentes compartilhem um hash com índices diferentes.

O indexador permanece separado da validação global para não criar recursão entre snapshot e projeto. A API pública de snapshot valida o projeto; os auxiliares de serialização/digest internos pressupõem tipos já validados. O grafo é conferido também ao reabrir o projeto. A propagação aos descendentes usa fila e mapa de relações; a busca das fontes históricas usa índice por snapshot, sem varrer repetidamente toda a coleção.

### 2.3 Limites de material e histórico de identidades

A primeira guarda de Poisson incluía o extremo `ν = -1`. O intervalo foi corrigido para `-1 < ν < 0,5` e os dois extremos são exercitados. A escolha corresponde ao parâmetro elástico declarado; não é aplicada a todos os materiais futuros ou a resultados normativos.

A validação de reabertura foi ampliada para recusar pais ou filhos repetidos, nascimento de filho em revisão/autoria incompatíveis, aposentadoria anterior à criação e histórico sem os objetos retirados. O comando já operava sobre candidato validado; os novos casos impedem que uma adulteração do arquivo contorne as invariantes do histórico.

### 2.4 Fechamento dos argumentos externos

A equipe de MCP encontrou que variantes unitárias de Serde ignoravam propriedades extras apesar do fechamento declarado no enum. `SourceOrigin::Native` e `ActionFrame::Engineering` passaram a usar variantes de struct vazia. O JSON válido não mudou, mas dados como `ifc_guid` ou `member_id` fora do contrato são recusados. A correção é exercitada diretamente no domínio e por processo MCP real.

A conferência final também identificou que uma chave ausente em uma atualização opcional poderia ser interpretada como remoção. `SetPhysicalMaterial` e `SetGeoreference` passaram a exigir a presença de `material_id` e `georeference`; a remoção exige `null` explícito. O desserializador do domínio e o schema MCP agora concordam. Campos opcionais de criação continuam opcionais. A regressão focal do domínio foi executada novamente nesta revisão e passou; o responsável pelo MCP confirmou três testes de processo, incluindo a preservação dos bytes quando essas chaves são omitidas e a remoção apenas quando solicitada explicitamente.

### 2.5 Texto numérico na interface

O parser padrão de `DragValue` removia espaços internos e não aceitava vírgula decimal. Assim, `1 2` poderia virar `12`, enquanto uma entrada com vírgula podia manter silenciosamente o número anterior. O formulário passou a conservar texto próprio, aceitar um separador decimal e recusar formato ambíguo, número não finito ou transbordamento.

Entradas inválidas permanecem visíveis com diagnóstico e impedem o envio da ação. O mesmo tratamento foi aplicado à criação rápida. A regra de texto fica na apresentação; conversão dimensional, referências e validade continuam no domínio.

### 2.6 Seleção, recarga e revisão de formulário

Foram encontrados dois problemas de estado: texto inválido de uma entidade podia alcançar outra com o mesmo valor numérico; e campos selecionados podiam manter os valores de uma revisão anterior após reabrir ou transformar o modelo em outra aba.

O segundo caso permitiria salvar uma geometria antiga com a revisão nova, contornando a intenção do controle de concorrência. A correção separa buffers por projeto, área e seleção, e recarrega entidades e referenciais quando muda a revisão recebida. Aba e seleção são mantidas, enquanto o texto numérico antigo é descartado.

A regressão real seleciona uma peça de 4 m, altera-a por outra instância para 6 m, reabre e grava novamente. A medida permanece em 6 m. Depois aplica translação pela outra aba e confere extremos de 2 e 8 m no formulário antes de outra gravação. Esse teste foi reexecutado pessoalmente na revisão e passou.

## 3. Execução e independência das referências

A revisão executou diretamente, no workspace com lock e `float_roundtrip` habilitado:

```sh
cargo test -p prumo-domain --test model -p prumo-app --test semantic_service --locked
cargo test -p prumo-desktop --test model --locked selected_forms_reload_new_revision_before_saving_or_after_transforming
```

Resultado: **17 testes de domínio, 10 testes de serviço/snapshot/grafo e a regressão focal de interface passaram**, sem casos ignorados nas seleções executadas. A execução dos dois pacotes incluiu as regressões textuais novas de precisão numérica. O teste focal da interface acionou widgets reais via egui_kittest, em vez de chamar apenas uma função de formulário.

Depois da correção de presença das chaves opcionais de atualização, foi executado diretamente o binário de teste recém-compilado para `optional_updates_require_presence_and_only_explicit_null_removes_values`: um caso passou, com os outros 17 filtrados. Esse é o 18º teste da bateria de domínio; não se atribui à execução anterior uma versão que ainda não continha essa correção.

A [evidência da interface/MCP](PRU-004-ui-mcp.md) registra também a execução pelo responsável de 7 testes novos de interface, 6 regressões anteriores e 14 testes de processo stdio. Os fontes e as asserções foram examinados nesta revisão, incluindo negação de escrita, recusa dimensional, identidade preservada entre superfícies, conexões explícitas e dados fechados. O resultado global da última compilação do workspace fica na evidência consolidada; não se somam execuções repetidas como testes distintos.

A inspeção nativa Linux, executada pelo responsável pela integração, abriu o projeto v1 na revisão 3, criou um objeto somente visual na revisão 4 e preservou três medidas na revisão 5. O resumo mostrou duas peças, uma visual, uma barra analítica e 8 m totais. A reabertura manteve as medidas e o backup do original foi conferido. Esse resultado está documentado na matriz geral; não é apresentado como uma segunda inspeção nativa realizada por este revisor.

As referências decisivas não são resultados obtidos novamente pela mesma função: há vetor oficial de GUID, relações SI conhecidas, geometria 3-4-5, coordenadas literais de rotação/translação, força e binário livre com momento calculado por braço vetorial, e comparação dos bits originais dos números após leitura textual. A ordenação de armazenamento foi diferenciada da orientação dos extremos analíticos.

Também foi executado o [ensaio isolado de uom](PRU-004-uom-ensaio.rs), com dois testes de conversão/limite e dois doctests negativos. Ele demonstrou que uom sozinho aceita NaN e temperatura absoluta negativa, justificando as guardas próprias. Esse resultado não é contado como qualificação de um motor de engenharia.

## 4. Licenças, manutenção e configuração

A [ficha de componentes](../componentes/PRU-004.md) compara uom 0.38.0, dimensioned 0.8.0 e tipos pequenos próprios com bibliotecas já qualificadas. Versão, commit de publicação, termos, arquivos efetivos e razões de adoção/rejeição foram examinados. Nalgebra passa a ser usada diretamente pelo domínio para os referenciais, mantendo a versão já incorporada.

O lock examinado tem SHA-256 `6755d05eb0c48f083291ab04d7417ed231606465e8c07672b4d3838a15daf35d`, 459 pacotes externos e 309 no grafo normal/build nativo. Os avisos preservam 262 textos distintos após incorporar uom e a atribuição específica do parser lexical. `generate_notices.py --check` passou, assim como `cargo deny --locked check advisories licenses sources`.

A base RustSec consultada foi o commit `4ef2df8e806a65e2f1ecc14f56f51249062b23d6`. As quatro exceções informativas de manutenção herdadas permanecem explícitas e restritas: cadeia AccessKit/zbus no Linux e proc-macro-error da compilação de ruststep-derive. Nenhuma foi ampliada para justificar a nova biblioteca. O parser STEP experimental da PRU-003 não se tornou importador IFC geral por causa dos novos tipos de identidade.

## 5. Limites do parecer e próximos portões

Esta revisão cobre a infraestrutura efetivamente implementada: modelar, converter grandezas, relacionar entidades, persistir, migrar, medir geometria, conservar bases e rastrear invalidação. Não qualifica cálculo estrutural ou de redes, interpretação de normas, assinatura profissional, geoprojeção ou intercâmbio IFC completo.

Material e procedência informados pelo usuário continuam declarações. A política normativa da PRU-002 e as restrições dos componentes experimentais da PRU-003 permanecem aplicáveis. A emissão de engenharia não foi habilitada por formulário, argumento MCP ou edição do estado de resultado.

A revisão local está concluída para a versão descrita. Antes de fechar a issue, a publicação deve ligar o commit e os artefatos corretos à execução final, à inspeção nativa e à matriz CI aprovada. Mudanças posteriores em regras, tipos, dependências ou comportamento precisam ser reconferidas no alcance afetado. A integração da PRU-004 é o pré-requisito para iniciar a PRU-005.

## 6. Revisão da correção do ambiente CI

A primeira execução Linux da PR 256 interrompeu a linkagem de executáveis de teste com `SIGBUS`, antes das asserções. O log não comprova a causa da falha remota. A ocorrência local de falta de espaço não foi tratada como diagnóstico confirmado do runner.

A correção do workflow limita a compilação a dois jobs, desativa o incremental, inaugura um namespace de cache que não restaura o cache anterior e executa `cargo clean --workspace --locked` antes da compilação. A opção de limpeza foi conferida na toolchain 1.99.1 e na [documentação do Cargo](https://doc.rust-lang.org/cargo/commands/cargo-clean.html): seu escopo são os artefatos dos membros do workspace, preservando dependências externas. O novo namespace também evita depender da substituição de um cache existente, operação que a [documentação do GitHub](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching) não permite.

O comando administrativo usa `subprocess.run` com argumentos separados e `check=True`, propagando uma falha do Cargo ao código de saída do Python também no Windows. Isso corrige a possibilidade, identificada na revisão inicial do passo, de um comando posterior de telemetria ocultar o erro. Não há shell montado por interpolação ou rotina própria de remoção de diretórios.

Essa alteração administrativa preserva o produto, os testes, as tolerâncias e a matriz Windows/Linux. A revisão da correção é favorável; o êxito só é estabelecido pela nova execução integral dos checks no commit corrigido. O parecer sobre os demais arquivos permanece inalterado.
