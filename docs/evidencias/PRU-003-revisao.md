# PRU-003: revisão independente

Data: **14/11/2026**.

## Parecer

**Parecer favorável à integração do recorte experimental da PRU-003, sem bloqueante remanescente no código e nas evidências locais conferidas.** A integração continua condicionada às verificações obrigatórias de Windows e Linux da PR. Este parecer não declara intercâmbio IFC geral, solver estrutural, dimensionamento ou emissão técnica disponíveis.

A revisão confrontou a issue 003, REUSO, REUSO-INSTALACOES e os contratos Rust com os arquivos implementados, fontes oficiais, licenças efetivas, testes, artefatos e medidas. Os adaptadores e as integrações foram implementados por outros agentes. Esta revisão produziu a pesquisa dos candidatos, a conferência de licenças e os avisos; as mudanças do gerador de avisos foram também examinadas pelo responsável independente da parcela numérica, e os trechos de BIM/documentos e numéricos da ficha foram confrontados pelos respectivos responsáveis.

O objeto desta conclusão é o código identificado pelos hashes do [registro de medição](PRU-003-metricas.json), junto das obrigações e dos limites da [ficha de componentes](../componentes/PRU-003.md). Não foi usada a popularidade dos fornecedores como critério de validação.

## Critérios confrontados

| Exigência | Conferência independente | Resultado |
|---|---|---|
| Comparar implementar, integrar, encapsular e contribuir | Tabela por função necessária; recorte das operações próprias; ausência de forks; custo de runtimes e de fronteiras explicitado | Atendido no recorte |
| Avaliar IfcOpenShell, PyNite, OpenSees, SuiteSparse, Three.js e OCCT | Versões e commits em fontes oficiais, textos de licença e arquivos pertinentes; decisões separadas de execução | Atendido documentalmente, sem alegar testes que não ocorreram |
| Executar experiências de BIM, geometria, solver, seções e documentos | Inspeção de código, referências e testes; cinco adaptadores reais e bateria adicional de WASM | Atendido |
| Manter referência independente | Geometria 3-4-5; coordenadas e comprimentos preparados separadamente; inversa 2×2 e molas; fórmulas de formas e seção L por composição; XML conferido por quick-xml | Atendido para os casos demonstrados |
| Interfaces substituíveis e origem rastreável | Traits delimitados, DTOs comuns, inventário de componentes e erros tipados; backend incorreto detectado nas baterias parametrizadas de geometria, sistema linear e seções | Atendido |
| Unidades, precisão, identidade e revisão | SI explicitado, tipos e nomes de grandezas, tolerâncias anteriores à execução, contexto de projeto com hash, limpeza de evidência na UI e retorno sem liberação de engenharia | Atendido |
| Licenças e transitivas | Manifests, arquivos de licença, comentários jurídicos específicos, checksums, fontes upstream recuperadas e avisos de distribuição | Atendido no grafo distribuído |
| Restrições comerciais | OpenSees não redistribuído; SuiteSparse tratado por módulo; subprodutos GPL/LGPL de IfcOpenShell e exceção OCCT discriminados | Atendido; nenhuma autorização comercial presumida |
| Rust e fronteiras externas | Código próprio em Rust, worker separado e WASM de origem Rust; scripts Python somente administrativos | Atendido |
| Falha, cancelamento, recursos e reinício | Limites de protocolo, processo recolhido, saída excessiva, falhas, prazo, reinício, fuel/memória WASM, imports e versão incompatível | Atendido para o worker revisado e o módulo fixo |
| FFI | Nenhuma nova FFI nativa de motor estrangeiro; ABI WASM escalar sem ponteiros; qualificação nativa futura explicitamente pendente | Aplicação correta ao escopo efetivo |
| Instalações | Estratégia individual de água, gás, elétrica, PV, termodinâmica, roteamento e catálogos, com independência e direitos dos dados | Atendido como estratégia, sem adoção desses motores |
| UI e MCP | Mesmo ProjectService e processo; operação de leitura, cancelamento, relatório de ensaio e manutenção dos bytes do projeto | Atendido |
| Build limpo, precisão, desempenho, tamanho e manutenção | Diretório de compilação inicialmente ausente, cinco amostras, hashes, pacote extraído, contagem de superfície própria e critérios de atualização | Atendido nas medidas locais; CI Windows/Linux permanece condição de integração |

## Achados e correções conferidos

1. **Licença comercial e escopo dos candidatos.** A pesquisa confirmou que o COPYRIGHT de OpenSees não concede distribuição comercial geral e que SuiteSparse possui licenças distintas por módulo. As decisões de não incorporação, os commits e os limites foram registrados; nenhum desses binários integra o pacote.
2. **Textos de licença ausentes em publicações Cargo.** ruststep, sua macro, geo, rstar, família wasmi e outras transitivas exigiram recuperação de arquivos upstream. O índice associa as fontes às versões exatas. O aviso posterior de convert_case 0.4.0 foi identificado como complemento, sem atribuí-lo falsamente ao pacote original.
3. **Obrigações por arquivo além do SPDX principal.** A inspeção identificou atribuição e licença Boost/SLEEF em wide, avisos Sun/libm em minimal-lexical e atribuição Turf/Chris Veness em geo. Os blocos e termos foram preservados. O gerador conserva também o hash do texto de origem antes da normalização de apresentação; os arquivos upstream não foram alterados.
4. **Aviso novo de manutenção.** proc-macro-error 1.0.4, dependência de compilação de ruststep-derive, gera RUSTSEC-2024-0370. A exceção ficou restrita ao ensaio, com motivo, alcance e condição de remoção/requalificação antes de importação de projetos. Não foi confundida com vulnerabilidade descrita nem silenciosamente ignorada. As três exceções herdadas de AccessKit também foram reavaliadas.
5. **Limitações reais de STEP.** O parser upstream não suporta escaping por apóstrofo duplicado no recorte examinado. O adaptador recusa esse caso e declara o limite. Também foram conferidos descarte de conteúdo posterior, referência órfã, unidade vinculada ao IfcProject do perfil, limites de recursos e rejeição de controles ASCII, inclusive dentro de strings. Não há reparo silencioso de texto ou unidade.
6. **Falha escondida pela última repetição.** As baterias numéricas passaram a conservar a primeira falha, mesmo se as repetições posteriores forem corretas. Há regressão com backend intermitente. BIM e SVG acumulam a aprovação de cada execução e confrontam os valores com a referência em cada repetição.
7. **Unidades e precisão numérica.** A estimativa do condicionamento usa a norma infinito, sem chamá-la de condição espectral exata. O resíduo é calculado fora da fatoração. A tolerância do centroide usa a escala local da seção, impedindo que uma origem global grande permita erro de centímetros. Ixy tem convenção de sinal explícita.
8. **SVG e texto.** Foram recusados caracteres U+FFFE/U+FFFF e controles incompatíveis com XML. O título ficou limitado a 40 caracteres e o caso de 40 letras W foi inspecionado visualmente. Texto com aparência de markup é escapado, sem criar script ou link. A renderização padrão foi examinada pessoalmente, sem corte ou sobreposição.
9. **Vida do processo e contexto da UI.** O encerramento recebeu guarda de ownership também nos erros antecipados. A saída da conversão WASM ganhou limite superior coerente com a entrada. A interface limpa a evidência quando o projeto muda e descarta resposta de outra revisão.
10. **Evidência de build e pacote.** Os scripts administrativos passaram a verificar protocolo, suíte, componentes, versões, unicidade, hash/tamanho, erros/tolerâncias e snapshots. As validações permanecem ativas com Python otimizado. A medição recusa alterações de fontes ou binários durante a execução; não apresenta build incremental como limpo.

Nenhuma dessas correções ampliou o domínio para cálculo de projetos. Os limites do perfil continuam publicados pelos descritores e pela documentação.

## Verificação executada e examinada

A rodada integrada registrada pelo responsável do workspace passou **86 testes**, além de build, formatação, Clippy com avisos como erros, compilação da UI para WASM, reprodução dos 172 bytes do módulo Rust, auditoria transitiva e conferência dos avisos.

Esta revisão executou diretamente os binários de teste já compilados, sem interferir no build congelado da medição:

| Conjunto executado pela revisão | Resultado |
|---|---:|
| Geometria, sistema linear, seções e WASM | 17 testes passaram |
| Inspeção BIM/STEP, incluindo controles ASCII | 7 testes passaram |
| Composição e XML do documento SVG | 5 testes passaram |

Os testes de processo foram lidos e confrontados com a implementação: executável real, hash/tamanho, término anormal, protocolo inválido, excesso de saída, prazo, cancelamento e execução posterior. Os testes de interface e MCP cobrem widgets reais, processo externo, cancelamento, relatório completo e ausência de alteração do projeto. Esses resultados integrados não foram apresentados como uma segunda implementação independente do algoritmo; a independência numérica vem das referências descritas.

A revisão examinou também a versão final de measure_components.py e package_dev.py após a conferência adicional registrada em [PRU-003-core.md](PRU-003-core.md). A inspeção dos scripts confirmou a rejeição explícita de inconsistências, a conferência dos snapshots e a validação do sistema/arquitetura.

## Conferência independente da medição

O [registro JSON](PRU-003-metricas.json) corresponde a Rust **1.99.1**, Linux x86_64, build release com dois jobs e target inicialmente vazio. O cache de downloads foi reutilizado. O build levou **262,831644113 segundos**.

Foram recalculados os hashes dos **53 arquivos de entrada** registrados. Nenhum divergiu, e a impressão digital conjunta conferiu:

`dccff3b74657af9fd19626fd10f5cd0b4f834da891d211b7707d2d384f815032`

Os cinco relatórios possuem identificadores de requisição distintos, contexto de projeto nulo para a medição de fixtures, seis componentes esperados, hash do mesmo worker e `engineering_release = false`. Cada erro numérico registrado foi confrontado novamente com sua tolerância.

| Bateria | Repetições por amostra | Verificações no relatório | Mediana observada | Intervalo das cinco amostras |
|---|---:|---:|---:|---:|
| BIM/STEP | 20 pares válido/inválido | 3 | 3,303 ms | 2,965 a 4,737 ms |
| Geometria rígida | 128 | 24 | 1,399 ms | 1,155 a 1,931 ms |
| Sistema linear | 64 | 41 | 1,175 ms | 0,948 a 1,465 ms |
| Seção poligonal | 64 | 66 | 2,772 ms | 2,079 a 3,782 ms |
| Documento SVG | 100 pares válido/inválido | 3 | 8,935 ms | 8,571 a 12,387 ms |
| Fronteira WASM | 1 | 8 | 0,996 ms | 0,549 a 6,082 ms |

Os tempos medem a bateria e suas verificações, não uma operação isolada nem desempenho relativo dos candidatos não executados. O processo completo variou de **25,735 a 88,345 ms**; a variação das cinco amostras locais não fundamenta promessa de desempenho de um projeto real.

Precisão observada nas referências: BIM e SVG tiveram erro zero nos valores conferidos; a geometria teve erro máximo de aproximadamente 2,845×10⁻¹⁶ m; o sistema linear, 2,132×10⁻¹⁴ m e resíduo relativo de 8,882×10⁻¹⁷; as seções, 2,220×10⁻¹⁶ m⁴ nos momentos e 1,388×10⁻¹⁷ m no centroide. A conversão WASM teve erro zero nos quatro valores de controle. Esses números pertencem às fixtures e não são tolerâncias gerais de produção.

### Artefatos conferidos

| Executável release | Tamanho | SHA-256 |
|---|---:|---|
| prumo-desktop | 19.556.824 bytes | `278b8ec10ea7bd524b9cfada1eee4c0325eea7757e49b3a949d5ecbf82459d04` |
| prumo-mcp | 7.127.728 bytes | `ac7dea3261345698bf24e5afea39473368722d3e2f42336c18cb01b894a4e358` |
| prumo-component-worker | 3.892.304 bytes | `29d18dee867abcc1006e4b400087138074a4fa0d65b751cec1d09301080df6e3` |

O arquivo `prumo-development-linux-x86_64-release.zip` tem **9.382.466 bytes** e SHA-256:

`f583f586c5d01393f8c09bb74797fb1e4b37294e2d68428e7137e2a14c52d615`

O hash do ZIP e de cada entrada com digest no manifesto foi recalculado. Não houve divergência. O pacote contém os três executáveis, avisos, instruções e manifesto; a medição registra inicialização dos três e execução da bateria do worker após extração. A extração ocorre no mesmo sistema com suas bibliotecas nativas, não em outra máquina. O manifesto marca `source_dirty = true`, identificando corretamente a medição local anterior ao commit de integração.

## Obrigações que permanecem explícitas

A auditoria do lock `3b5d52a43f7ab63e92bf170682adb5397419a920af49f4b98d9dc48e266f3baf` passou com 458 pacotes externos inventariados, 308 no grafo normal/build nativo e 259 textos jurídicos preservados. As quatro exceções são de manutenção, com justificativas e gatilhos de reavaliação. Não há dispensa geral de vulnerabilidades ou de obrigações de distribuição.

Os ensaios aprovam somente os recortes descritos. O parser não está liberado para importação IFC geral; os componentes numéricos não recebem aprovação estrutural; o SVG não é uma emissão técnica; o worker não é uma sandbox de código hostil; o limite de memória WASM não representa a memória inteira do interpretador. Cada expansão depende da issue pertinente, de referências próprias e de nova avaliação das licenças e fronteiras.

A revisão local, portanto, sustenta a submissão e integração da PRU-003 **depois dos checks obrigatórios da PR**, preservando as dependências sequenciais para a próxima etapa.
