# Verificação, validação e liberação técnica

Atualizado em 14/11/2026. **Estado: plano de aceitação, ainda não executado.** Não há neste repositório evidência de validação do produto, certificação, parecer de responsável técnico ou aprovação de engenheiro atribuída a esta sessão.

Este documento define o que cada implementação e a versão de lançamento deverão demonstrar. O registro de normas está em [NORMAS.md](NORMAS.md); as fontes estão em [fontes-normas.json](fontes-normas.json).

## Plano de verificação e validação

Verificação avalia a implementação e a solução numérica. Validação avalia adequação do modelo ao fenômeno e ao uso pretendido. Os conceitos e o tratamento de incertezas têm referências metodológicas primárias em ASME e NASA; servem como base de processo do Prumo, sem alegar certificação por essas entidades. [N16, N17]

### Camada 1: matemática e mecânica elementar

Casos com solução analítica independente: barra axial, torção, viga simplesmente apoiada, balanço, viga contínua, pórtico plano/espacial, treliça, deslocamento imposto, variação térmica e mola. Cobrir cargas pontuais, distribuídas, lineares e excêntricas; eixos locais; sinais; unidades; rotações; restrições; liberações e offsets.

Testes de propriedades: conservação de forças e momentos; energia; reciprocidade para modelos em que se aplica; superposição apenas no regime linear; invariância à rotação/translação da origem e equivalência após conversão de unidades. Casos com resposta zero precisam de erro absoluto, nunca divisão por zero no erro relativo.

### Camada 2: elementos finitos e estabilidade

Ensaios de placa, casca, membrana e sólido conforme elementos efetivamente disponíveis. Testar patch test, modos rígidos, travamento numérico, espessuras extremas, razão de aspecto, qualidade de malha, interfaces barra-casca e convergência de refinamento. Não usar tensão singular de um nó como critério de convergência física.

Para não linearidade: segunda ordem global e local, imperfeições, contatos/apoios unilaterais, fissuração e rigidez efetiva, material inelástico, fluência/retração e fases quando suportados. Registrar malha, passos, tolerância, norma dos resíduos e critério de parada. Instabilidade, divergência e falta de equilíbrio devem gerar falha clara, nunca resultado reutilizado silenciosamente.

NAFEMS disponibiliza descrições de conjuntos independentes para elementos, estática, vibração e não linearidade. Os alvos podem precisar de acesso licenciado, e a versão/errata do próprio benchmark deve ser controlada. [N18, N19]

### Camada 3: aplicação das normas

Para cada regra implementada, preparar exemplos próprios calculados de forma independente do código produtivo e revisados antes de aceitar a implementação. Cobrir pelo menos um caso abaixo, um no limite e um acima de cada fronteira relevante; mudanças de regime; valores inválidos; aplicação e inaplicabilidade. Comparar a classificação de atende/não atende, não apenas o valor numérico.

| Família | Casos essenciais |
|---|---|
| Ações e combinações | Favorável/desfavorável, alternância de sobrecarga, variável principal, correlação/exclusão, vento em direções, fases, ações excepcionais, temperatura, retração/recalque imposto; combinações ELU e ELS completas |
| Concreto | Flexão simples/composta/oblíqua, compressão, cisalhamento, torção, punção em pilares internos/borda/canto e aberturas, instabilidade, fissuração, flecha imediata/diferida, fadiga aplicável, ancoragens/emendas/cobrimento |
| Protensão | Perdas imediatas e diferidas, sequência, traçado, zonas de ancoragem, tensões de serviço e transferência, estado final e fases temporárias |
| Pré-moldados | Transporte/içamento/montagem, ligações e consolos, apoios, tolerâncias, estabilidade temporária e ação composta |
| Aço e mistos | Tração/compressão, flambagens, flexão/cisalhamento/torção, interação, estabilidade lateral, ligações soldadas/parafusadas e bases, fadiga aplicável, conectores e sequência construtiva |
| Perfis formados a frio | Modos locais, distorcionais e globais; seção efetiva; furos e interfaces de ligação no domínio permitido |
| Madeira | Classes/propriedades, umidade e duração, compressão perpendicular, tração, flexão, cisalhamento, estabilidade e fluência; ligações; MLC/CLT apenas com método e dados validados |
| Alumínio | Base normativa própria, propriedades e ligações compatíveis; instabilidade e efeitos da fabricação no domínio selecionado, sem aprovar por reutilização indiscriminada de aço |
| Alvenaria estrutural | Modulação, propriedades, compressão/cisalhamento/flexão, estabilidade, aberturas, graute/armaduras e ligações; compatibilidade entre modelo, verificações e detalhe |
| Fundações | Capacidade geotécnica e estrutural separadas, punção/flexão/cisalhamento, excentricidade/contato parcial, grupos, recalques e interação solo-estrutura; laudo geotécnico identificável |
| Vento, sismo e dinâmica | Aplicabilidade, massa participante, modos, espectros/combinações modais quando previstos, amortecimento, deriva, aceleração, conforto e efeitos torsionais |
| Incêndio | Exposição, tempo exigido, método térmico/estrutural adotado, combinação excepcional, materiais e proteção, detalhamento; domínio e limitações explícitos |

### Camada 4: comparação com ferramentas independentes

Comparações com Eberick, TQS, CYPE e solver de referência só são válidas com licenças autorizadas, mesma geometria analítica, unidades, apoios, malhas, materiais, rigidezes, ações, combinações e edição normativa. Registrar versão de cada sistema e arquivo de entrada. Toda diferença relevante deve ser explicada por método, premissa ou defeito.

Concordância entre softwares pode reproduzir o mesmo erro. Por isso não substitui solução analítica, cálculo independente e dados experimentais onde o modelo os exige. Não ajustar tolerâncias após ver divergência apenas para obter aprovação.

### Camada 5: validação integrada e experiência de uso

Projetos completos representativos: residência em concreto, edifício com núcleo e lajes, lajes sem vigas com punção, garagem de vãos significativos, galpão de aço, edificação mista, cobertura e pórtico de madeira, fundações com recalques, estrutura pré-moldada, estrutura protendida e modelos combinados. Para cada caso, cobrir todas as etapas prometidas: importação, concepção, análise, dimensionamento, edição, detalhamento, memorial, quantitativo, exportação IFC/DWG/PDF e revisão.

Injetar defeitos reais: elemento desconectado, apoio removido, unidade errada, malha inadequada, IFC truncado, carga não atribuída, ligação incompatível, dado de solo ausente, combinação incompleta, resultado de revisão anterior, alteração por agente em concorrência e perda de conexão. Aceite exige diagnóstico compreensível, preservação dos dados e bloqueio da emissão como aprovada quando necessário.

O conjunto final precisa incluir obras representativas revisadas por engenheiros habilitados, independentes dos autores do módulo avaliado. Nomes, competências, escopo, parecer e responsabilidade da revisão devem estar registrados. A avaliação técnica de projeto, quando aplicável, é processo específico e não se confunde com revisão automatizada de software.

## Aproveitamento de código aberto com domínio comprovado

Incorporar bibliotecas existentes sempre que elas reduzirem esforço total com adequação demonstrada, preservando módulos substituíveis. A aprovação é por capacidade, formulação e build, e não apenas pelo nome ou popularidade da biblioteca. Uma biblioteca de análise não entrega automaticamente dimensionamento, detalhamento ou conformidade com as normas brasileiras.

| Candidato | Evidência oficial consultada | Uso na decisão de arquitetura |
|---|---|---|
| PyNite | Documentação 3.2.0 informa P-Delta em barras, sem o mesmo efeito em placas; pushover público ainda em primeira ordem sem integração P-Delta | Avaliar análise de barras e casos delimitados; cobrir limitações com outro módulo ou bloqueio explícito, sem ampliar alegação por inferência [N20, N21] |
| OpenSees | Documentação oficial apresenta processo de verificação com exemplos e distingue validade do modelo de correção do código | Candidato para análise não linear/dinâmica e comparação, condicionado a licença, capacidade exata e ensaios do adaptador [N22] |

O pacote de qualificação de cada dependência registra repositório, commit/tag, licença e obrigações, dependências transitivas, build, compilador, plataforma, precisão, configuração e domínio aceito. Inclui documentação da formulação, limitações, defeitos conhecidos, benchmarks próprios, resultados e revisão independente. Mudanças no upstream passam por comparação de saídas e revisão das capacidades afetadas antes da promoção.

Testar o adaptador importa tanto quanto testar o solver: convenções locais/globais, sinais, unidades, offset, ordem dos graus de liberdade, cargas distribuídas, rigid links, diafragmas, releases, massas, amortecimento e extração/envelopamento de resultados. Não eliminar singularidade por acréscimo automático e silencioso de apoios, rigidezes ou vínculos artificiais. A correção precisa ter significado físico e aprovação explícita do projetista.

**Independência das referências:** duas bibliotecas que chamam o mesmo motor, duas interfaces do OpenSees ou um oráculo que importa a função produtiva não são referências independentes. Registrar a genealogia das soluções, identificando código, formulação e dados compartilhados. Se houver reutilização, manter comparação útil de integração, mas complementar com solução analítica, implementação diferente e/ou ensaio físico conforme a capacidade. A quantidade de wrappers concordantes não aumenta a força da evidência física.

O roadmap deve incluir uma issue inicial de seleção/qualificação de dependências e aceites de integração em cada módulo. Manter fallback executável ou estratégia de substituição, registro de vulnerabilidades, acompanhamento de defeitos numéricos e bateria de regressão. A licença precisa ser avaliada no cenário real de distribuição desktop, serviço remoto, plugins e eventual uso comercial; não assumir permissão por o código estar publicamente acessível.

## Tolerâncias pré-registradas

Nenhum número universal de tolerância demonstra confiabilidade de todas as grandezas. Cada benchmark fixa previamente valores e justificativa para: erro absoluto, erro relativo, escala de referência, equilíbrio, energia, malha, convergência iterativa, variações entre plataformas e classificação normativa. Os limites são aprovados antes da execução e versionados.

Para comparação escalar, pode-se adotar critério do projeto `|resultado - referência| <= atol + rtol * escala`, com unidades e definição explícita de escala. Tolerâncias algébricas devem ser distintas das diferenças de discretização e das incertezas físicas. Não são coeficientes de segurança e não autorizam exceder limite normativo.

Valores arredondados não podem esconder mudança de classificação. Testes perto da taxa de utilização 1 devem preservar decisão conservadora e indicar sensibilidade. Alvos críticos com incerteza maior que a margem disponível ficam inconclusivos, exigindo revisão ou refinamento.

### Metas iniciais para preparar a bateria

Os valores a seguir são **propostas de engenharia para testes do Prumo, não exigências da ABNT nem evidências de desempenho já obtido**. A issue de validação inicial deve confirmá-los, ajustá-los com justificativa técnica ou substituí-los por limites específicos antes de executar o caso contra a implementação avaliada. Um caso sem tolerâncias aprovadas permanece pendente.

| Tipo de ensaio | Meta inicial sugerida | Condições e significado |
|---|---|---|
| Conversões de unidades e ida/volta do formato canônico | Erro relativo até `1e-12`; estados categóricos e IDs idênticos | Usar escala e erro absoluto documentados para valores próximos de zero; não estender a arquivos externos que quantizem dados |
| Barras, treliças e vigas lineares com solução analítica representada exatamente pelo elemento | `rtol = 1e-6`; `atol = 1e-10 × escala do caso`, com unidade | Verificar esforço e deslocamento separadamente; não impor a casos que dependem de refinamento ou adotam teoria diferente |
| Equilíbrio global de modelo linear estável | Resíduo normalizado de forças e momentos até `1e-8` | Escala baseada na soma dos módulos das ações ou referência positiva pré-definida; registrar equilíbrio por componente |
| Equilíbrio em solução não linear convergida | Resíduo normalizado até `1e-6` em cada passo aceito | Acrescentar critério de incremento/energia apropriado; atingir resíduo sozinho não prova alcançar a solução física |
| Placas/cascas elásticas regulares contra referência independente | Até `1%` em deslocamentos/resultantes selecionados; diferença entre duas últimas malhas até `0,5%` | Ao menos três refinamentos; avaliar ordem/convergência e excluir singularidades físicas do alvo pontual; pode exigir limites menores |
| Frequências próprias de problemas lineares com solução analítica | Até `0,1%` nas frequências-alvo; correspondência de modos previamente definida | Tratar modos repetidos como subespaços, sem depender da ordem arbitrária dos autovetores |
| Oscilador linear transiente com solução conhecida | Até `0,5%` em amplitude-alvo e `0,5%` em fase/período definido; estabilização com redução de passo | Fixar intervalo, amortecimento, passo e forma de extrair picos; não extrapolar para toda resposta sísmica |
| Fórmula normativa escalar reproduzida por cálculo independente | `rtol = 1e-6` e erro absoluto por unidade; `100%` de correspondência na classificação dos casos obrigatórios | Coeficientes e valores de tabela devem coincidir com a precisão especificada; tolerância numérica não concede margem além do limite normativo |
| Propriedades geométricas do modelo canônico após exportar/importar | Distâncias até `0,1 mm`; áreas/volumes até `0,01%` nos casos de referência | Fixar suporte do formato e precisão da origem; vínculos, materiais, unidades e conectividade exigem correspondência sem perda silenciosa |
| Comparação entre motores com modelos harmonizados | Investigar desvio acima de `1%` em respostas elásticas globais ou `2%` em alvos não lineares selecionados | São gatilhos de investigação, não liberação automática de diferença menor. A aceitação depende de explicar premissas e cumprir o alvo independente |
| Dados experimentais para validação física | Definir previamente intervalo de incerteza, cobertura e erro admissível para cada resposta | Sem percentual universal. O protocolo identifica precisão instrumental, variabilidade, erro do modelo e finalidade da previsão |

A escolha dos valores depende da formulação, condicionamento do problema, escalas e precisão da referência. Um teste que compara resultado com o próprio valor gravado pelo motor só detecta regressão; não demonstra correção inicial.

### Pré-registro e revisão de tolerâncias

1. O autor do caso prepara objetivo, referência independente, entrada, alvos, métricas, tolerâncias, domínio e justificativa, sem consultar a saída da implementação para escolher o limite.
2. Um revisor tecnicamente competente, independente do autor do módulo, aprova o protocolo e registra identidade, competência e decisão. Isso é requisito futuro; nenhum parecer humano é presumido pelo texto deste documento.
3. O protocolo recebe versão e hash antes da execução. O relatório conserva os limites originais inclusive quando o caso reprova.
4. Mudança posterior exige registro de causa, revisão independente e repetição dos casos afetados. Ampliar limite apenas para acomodar desvio do produto é proibido.
5. Casos de fronteira normativa devem acrescentar decisões imediatamente abaixo e acima do limite na precisão relevante. Casos inconclusivos não podem ser classificados como aprovados.

## Evidência mínima por caso

Cada caso conserva: ID, requisitos cobertos, fonte/autor da solução independente, licença dos dados quando aplicável, modelo e unidades, hipóteses, método, versão normativa, resultado esperado, tolerâncias pré-aprovadas, executor, revisor, hashes SHA-256 das entradas e saídas, versão/commit/build, plataforma, saída numérica completa e decisão. As imagens são complementares; não substituem dados de comparação.

O manifesto da execução também identifica o SHA do commit do **produto Prumo** e o SHA-256 do executável, contêiner ou pacote efetivamente executado. O hash apenas do script de teste não atende a esse requisito. Registrar versão/commit/build de cada motor e biblioteca, configuração, imagem de ambiente, variáveis numéricas relevantes e arquivo de dependências congeladas. A evidência da versão A não autoriza a versão B sem análise de impacto e ensaios correspondentes.

Identificadores mínimos previstos para o manifesto: `case_id`, `requirements`, `protocol_sha256`, `reference_origin`, `independence_assessment`, `model_sha256`, `analytical_model_sha256`, `loads_sha256`, `code_bundle_id`, `solver_commit`, `solver_binary_sha256`, `prumo_commit`, `prumo_artifact_sha256`, `environment_digest`, `expected_results_sha256`, `actual_results_sha256`, `metrics`, `tolerances`, `decision`, `reviewer_record` e `execution_timestamp`.

As evidências devem ser reproduzíveis a partir de uma versão fechada. Casos reprovados permanecem visíveis no histórico. Correção não apaga o resultado antigo nem troca o alvo sem análise formal de causa.

## Validade e caducidade dos resultados

O resultado pertence a um conjunto imutável de entradas. A interface, API e MCP devem consumir esse mesmo vínculo. Não basta mostrar que algum cálculo anterior terminou com sucesso.

| Evento | Comportamento exigido |
|---|---|
| Alteração de geometria, seção, apoio, conexão, carga, material, malha, fase, parâmetro ou pacote normativo | Invalidar os resultados dependentes; preservar execução antiga com identificação de histórico; impedir emissão atual como aprovada até recalcular e revisar |
| Alteração puramente visual sem efeito técnico | Preservar o cálculo apenas se o classificador de impacto comprovar que nenhuma entrada de análise ou verificação mudou |
| Atualização de solver, motor de regras ou dependência numérica | Nova versão recebe qualificação e comparação; resultados antigos permanecem ligados ao ambiente original e não mudam silenciosamente |
| Nova norma ou errata identificada | Registrar disponibilidade e aplicabilidade; manter histórico do pacote antigo; emissão nova exige decisão normativa documentada e eventual migração/revisão |
| Detecção de defeito que afeta resultados já calculados | Marcar famílias, versões e projetos potencialmente atingidos; alertar destinatários conforme processo aprovado; recalcular e reemitir quando necessário |
| Comando concorrente ou execução terminada após nova edição do modelo | Publicar como histórico da versão original, sem substituir o estado da revisão atual; exigir correspondência de versão no aceite |
| Arquivo exportado sem correspondência ao cálculo atual | Identificar como documento anterior ou incompleto; bloquear a declaração de aprovação atual |

Testar toda a tabela por interface e por MCP. Mudanças não podem ser escondidas em notas de agente ou substituídas por simples atualização do selo visual. O estado de cálculo, atendimento normativo, revisão técnica e emissão documental deve ser rastreável separadamente.

## Zero falsos aprovados na bateria obrigatória

Critério de lançamento: **zero ocorrências de falso aprovado em todos os casos obrigatórios executados**. Trata-se de um critério observável da bateria e de bloqueio de liberação, não de garantia de inexistência de qualquer defeito em qualquer obra futura.

Um falso aprovado inclui: resultado fora do limite classificado como atende; verificação obrigatória omitida; uso fora do domínio sem indicação; falta de convergência ocultada; resultado vencido mostrado como atual; ausência de propriedades essenciais substituída por padrão invisível; conexão importada incorretamente; saída de agente apresentada como cálculo; ou emissão declarada aprovada sem os aceites exigidos.

Cada caso negativo deve demonstrar o diagnóstico e a interrupção adequada, sem perda do modelo. Nenhuma falha dessa classe pode ser dispensada por média de acerto, cobertura de linhas, contagem de testes verdes ou concordância de dois wrappers do mesmo motor.

## Critérios técnicos obrigatórios para lançamento

1. Todas as capacidades obrigatórias da matriz de equivalência possuem implementação e domínio definido, documentação, casos independentes e evidências da versão candidata. Uma capacidade pendente não pode ser marcada como lançada por esconder o botão.
2. Todo pacote normativo aplicável foi conferido no catálogo, interpretado com acesso licenciado, revisado e congelado; todas as pendências P de escopo obrigatório foram resolvidas.
3. Nenhum defeito aberto que possa permitir falsa aprovação, perda de dados, mistura de revisões ou resultado não rastreável. Toda falha detectada na bateria obrigatória bloqueia o lançamento.
4. Casos integrados obrigatórios, testes de regressão, testes de dados inválidos e testes de concorrência de agentes passaram nas plataformas prometidas.
5. Dossiê técnico independente aprovado, com resultados e limitações coerentes com a documentação e a oferta comercial. Os nomes das fontes pesquisadas não serão usados como certificado de equivalência.
6. Emissão de projeto exige autor/revisor e confirmação sobre o modelo calculado; todos os arquivos entregues carregam identificação de versão e validade do resultado.
7. Processo de relato de defeitos, triagem de erro estrutural, correção, comunicação de versões afetadas, rastreio de projetos impactados, reversão e recomputação está ensaiado.
8. Mudança de versão de norma, catálogo ou solver reabre a bateria impactada. Atualização de segurança que muda resultado precisa de análise técnica e registro.


## Referências metodológicas e bibliotecas

Todas as referências abaixo foram consultadas em 14/11/2026. Sua inclusão orienta a implementação e a validação; não representa certificação, auditoria contratada ou chancela ao Prumo.

- **N16**. [V&V 10 Standard for Verification and Validation in Computational Solid Mechanics](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019). ASME. Referência metodológica para verificação, validação e quantificação de incertezas em mecânica computacional. Não representa certificação do Prumo.
- **N17**. [NASA-STD-7009 Standard for Models and Simulations](https://standards.nasa.gov/standard/nasa/nasa-std-7009). NASA. Página apresenta revisão B e requisitos de credibilidade, com critérios de aceitação definidos pelo projeto; referência de processo, sem aplicação normativa automática ao Prumo.
- **N18**. [The Standard NAFEMS Benchmarks](https://www.nafems.org/publications/resource_center/p18/). NAFEMS. Conjunto independente cobre respostas estáticas, térmicas e vibração; descreve alvos analíticos/numéricos e erratas de benchmarks. Acesso completo pode depender de licença.
- **N19**. [A Review of Benchmark Problem for Geometric Non-linear Behaviour of 3D Beams & Shells](https://www.nafems.org/publications/resource_center/r0029/). NAFEMS. Descreve dez problemas de barras/cascas 3D em não linearidade geométrica, com domínio limitado a pequenas deformações específicas e grandes deslocamentos/rotações.
- **N20**. [Stability, PyNite 3.2.0 documentation](https://pynite.readthedocs.io/en/stable/stability.html). PyNite maintainers. Documenta instabilidade e P-Delta; declara que o efeito de segunda ordem não é considerado nas placas.
- **N21**. [Pushover Analysis, PyNite 3.2.0 documentation](https://pynite.readthedocs.io/en/latest/pushover.html). PyNite maintainers. Documenta pushover público com primeira ordem; integração P-Delta pendente e limitações do comportamento de placas e quads.
- **N22**. [VerificationAndValidation, OpenSeesWiki](https://opensees.berkeley.edu/wiki/index.php/VerificationAndValidation). OpenSees, UC Berkeley. Descreve verificação por casos conhecidos e diferenças entre correção do código e adequação do modelo físico; exemplos não significam garantia universal.

## Campanhas de instalações e validação integrada

A ampliação preserva os ensaios estruturais. A qualificação final cobre também cada capacidade de instalações, com referências, fontes normativas e limites por domínio. A licença da biblioteca e o histórico de seus testes são insumos; o adaptador e as regras do Prumo precisam de verificação própria.

| Domínio | Provas mínimas a especificar e executar |
|---|---|
| Água e redes pressurizadas | Continuidade, energia, perdas distribuídas/localizadas, bombas, pressões extremas, válvulas, ramificações e redes fechadas; referência independente e casos não convergentes |
| Esgoto, chuva e tratamento | Declividades, unidades de contribuição, capacidades, ventilação, cotas, área e intensidade de chuva, balanço de reservação e dimensionamento de unidades com hipóteses explícitas |
| Gás | Pressões absolutas/manométricas, condições de referência, densidade, simultaneidade, regimes baixa/média pressão, reguladores e limites do método; nenhum reaproveitamento de água sem adequação |
| Incêndio | Cenários simultâneos, alcance, pressão/vazão nos pontos críticos, reserva, curva de bomba, área de operação de sprinklers, equilíbrio em malhas e perfil regulamentar local |
| Elétrica | Demanda, fases, neutro/PE, fatores de correção, seções, queda de tensão, sobrecarga/curto, DR/DPS, ocupação, equipamentos e barramento, com casos imediatamente antes/depois dos limites |
| Fotovoltaica | Séries/arranjos, extremos térmicos e elétricos, inversor, CC/CA, cabos/proteções, energia e hipóteses econômicas; incerteza dos dados de geração separada de resistência elétrica |
| SPDA | Componentes de risco, medidas, geometria dos três métodos, descidas/componentes naturais, aterramento e equipotencialização, com contraexemplos desprotegidos |
| Cabeamento | Continuidade lógica, comprimento, categoria/tipo, ocupação, segregação, racks/portas, numeração e mapa físico/lógico coerente |
| Climatização | Cargas sensíveis/latentes, propriedades psicrométricas, condições de contorno, perda de carga, vazão, comprimento equivalente, condensado e domínio de equipamentos |

Os casos conservam topologia e parâmetros além da geometria. Pressão, vazão, energia, corrente, potência, temperatura e umidade têm métricas e tolerâncias próprias. Tolerâncias de mecânica estrutural não são aplicadas automaticamente aos demais domínios. O protocolo de cada caso deve definir unidades, arredondamento e decisão antes de observar a saída.

EPANET e ferramentas que o usam não são referências mutuamente independentes quando compartilham o mesmo núcleo. O mesmo vale para qualquer solver e seu adaptador. A origem das referências e a preparação das entradas devem permitir identificar erros comuns.

As issues 239 a 243 executam campanhas por domínio; 244 realiza pilotos autorizados; 245 avalia usabilidade; 246 verifica segurança; 247 resolve normas/licenças/oferta; 248 verifica distribuição e desempenho; 249 consolida o candidato e os aceites profissionais; 250 libera o mesmo artefato aprovado.

Os pilotos incluem no mínimo doze projetos reais autorizados, três com integração efetiva de estrutura e instalações e cada uma das oito disciplinas de instalações em pelo menos dois projetos. Casos sintéticos adicionais cobrem hipóteses e fronteiras ausentes nos pilotos. Dados identificáveis e arquivos de terceiros exigem autorização compatível.

O aceite técnico é realizado por profissionais habilitados nas respectivas atribuições. A revisão estrutural isolada não qualifica elétrica, gás ou qualquer outra disciplina. O dossiê final liga cada capacidade a seus revisores, evidências, normas e limites. Planejamento revisado por agentes não equivale a esse aceite profissional.
