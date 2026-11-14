# Prumo: abrangência e comprovação de paridade

**Referência da pesquisa:** documentação pública consultada em 14/11/2026. **Estado do Prumo:** planejamento. Nenhuma capacidade deste documento deve ser anunciada como implementada ou validada enquanto não houver as evidências correspondentes.

O objetivo é reunir a cobertura funcional dos sistemas de referência em um produto BIM nativo, acessível pela interface e por MCP, com operação intuitiva e cálculo rastreável. A comparação considera funcionalidades documentadas do Eberick, do CYPECAD, do TQS e do QiBuilder/Builder, identificando os módulos e aplicativos associados quando necessários para completar o fluxo.

## 1. Como interpretar a comparação

**Documentado** significa que uma fonte do fabricante descreve a capacidade. Não significa que a equipe do Prumo executou o software, validou seus resultados ou confirmou disponibilidade em toda licença. **Não apurado** significa que a coleta ainda não produziu evidência suficiente. Não significa ausência do recurso no concorrente.

O catálogo público pode combinar versões. Por isso, cada capacidade precisa de produto, módulo, edição, data da fonte, tipos de elemento, hipóteses admitidas, norma e limites. Artigos de ajuda com normas antigas são úteis para compreender procedimentos, mas não determinam a edição normativa aplicável ao Prumo.

O inventário é a **base mínima de abrangência**, não uma declaração de levantamento exaustivo. A primeira etapa de especificação deve completar o catálogo de versões e decompor cada família em capacidades verificáveis. Recursos documentados e pertinentes encontrados depois continuam como pendências da mesma base, sem desaparecer por falta de espaço no planejamento inicial.

## 2. Matriz de capacidades pesquisadas

Os identificadores abaixo são permanentes para permitir vincular requisitos, issues e evidências. Uma família só estará concluída quando todas as suas capacidades aplicáveis estiverem verificadas.

| ID | Capacidade | Evidência funcional nos sistemas de referência | Exigência para o Prumo |
|---|---|---|---|
| PAR-01 | Concepção e modelagem | Lançamento estrutural e modelo BIM no Eberick [EB-01]; entrada por plantas no CYPECAD [CY-01] | Modelar por objetos e conexões compreensíveis, em 2D e 3D |
| PAR-02 | Modelo analítico | Geração a partir dos elementos físicos no TQS [TQ-01] | Rastrear objeto, discretização, vínculo, excentricidade e resultado |
| PAR-03 | Análise global | MEF e modelo integrado Eberick [EB-01]; Modelos IV/VI TQS [TQ-02] | Resolver o conjunto e demonstrar equilíbrio e convergência |
| PAR-04 | Ações e combinações | Combinações e modelos ELU/ELS no TQS [TQ-01] | Identificar origem, simultaneidade, coeficientes e combinação governante |
| PAR-05 | Instabilidade e não linearidade | Estabilidade e não linearidade física/geométrica TQS [TQ-01] | Tratar efeitos globais/locais e registrar limites do método |
| PAR-06 | Concreto armado | Modelagem, dimensionamento e detalhamento Eberick [EB-01] | Verificações de resistência, serviço, durabilidade e execução |
| PAR-07 | Lajes e pavimentos | Maciças/nervuradas/treliçadas Eberick [EB-02]; alveolares e mistas CYPECAD [CY-01] | Preservar particularidades de cada tipologia |
| PAR-08 | Elementos especiais | Vigas curvas/variáveis Eberick [EB-02]; pilares-parede/transições TQS [TQ-01] | Análise e dimensionamento coerentes com geometria e transferência |
| PAR-09 | Escadas e rampas | Elementos inclinados Eberick [EB-02]; escadas por MEF CYPECAD [CY-01] | Conferir transferência das reações e armadura executável |
| PAR-10 | Fundações | Sapatas/blocos/radiers/estacas Eberick [EB-02]; SISEs TQS [TQ-03] | Separar verificações estruturais e geotécnicas |
| PAR-11 | Interação solo-estrutura | Integração de fundações e molas pelo SISEs [TQ-03] | Rastrear sondagens, propriedades, recalques e efeitos no edifício |
| PAR-12 | Contenções e reservatórios | Muros e reservatórios Eberick [EB-02] | Ações próprias, situações de uso, fissuração e estabilidade |
| PAR-13 | Protensão | Modelo e processamento Eberick [EB-03]; Protensão LV/VPRO TQS [TQ-06] | Cabos, perdas, fases, armaduras e efeitos primários/secundários |
| PAR-14 | Pré-moldados | Etapas e ligações Eberick [EB-01]; PREO TQS [TQ-06], [TQ-09] | Fases completas, pi/TT/telhas, painéis, insertos, apoios e montagem |
| PAR-15 | Aço | Perfis Eberick [EB-04]; barras CYPE 3D [CY-02] | Resistência, estabilidade, serviço e detalhamento por família |
| PAR-16 | Estruturas mistas | Vigas/pilares mistos no MetalCheck [TQ-04] | Interação entre materiais, conectores e etapas |
| PAR-17 | Ligações metálicas | Ligações e bases no CYPE 3D; integração com aplicativos específicos [CY-02] | Ligações, chapas, fixadores, ancoragens e rigidez associada |
| PAR-18 | Madeira | Barras de madeira no CYPE 3D [CY-02] | Caracterização, anisotropia, estabilidade, ligações e serviço |
| PAR-19 | Alumínio | Elementos de alumínio no CYPECAD/CYPE 3D [CY-01], [CY-02] | Modelagem e verificação com base normativa própria |
| PAR-20 | Material genérico | CYPECAD distingue análise de esforços de dimensionamento [CY-01] | Análise com propriedades declaradas, sem aprovação normativa implícita |
| PAR-21 | Alvenaria e paredes | Alvenaria estrutural Eberick [EB-01] | Modulação, cargas, ligações, dimensionamento e detalhes específicos |
| PAR-22 | Vento, dinâmica e sismo | Vento/túnel Eberick [EB-02]; dinâmica/sismo TQS [TQ-01] | Ações, modos, estabilidade e conforto com hipóteses identificadas |
| PAR-23 | Incêndio | Recurso Eberick [EB-02]; verificações por material CYPECAD [CY-01] | Resistência e proteção segundo norma aplicável |
| PAR-24 | Desenhos e armaduras | Detalhamento Eberick [EB-02]; editor AGC TQS [TQ-08] | Pranchas, editor 2D, detalhes reutilizáveis e edição rastreável |
| PAR-25 | Memoriais e quantitativos | Memoriais/XLSX Eberick [EB-02]; relatórios TQS [TQ-01] | Resultados explicáveis e quantidades reconciliadas |
| PAR-26 | Interoperabilidade BIM | IFC/BCF Eberick [EB-02]; BIM e plug-ins TQS [TQ-05] | Identidades, unidades, propriedades e perdas de conversão controladas |
| PAR-27 | Coordenação | Colisões/furos Eberick [EB-02]; tubulações TQS [TQ-05] | Resolver interferências com repercussão no cálculo |
| PAR-28 | Programação e agentes | Axis Nodes/Python Eberick [EB-01] | Programação visual, rotinas e MCP desde o primeiro incremento funcional |
| PAR-29 | Formatos e ecossistemas | DWG/DXF e BIMserver.center CYPECAD [CY-01]; plug-ins TQS [TQ-05] | Compatibilidade especificada e testada, com dependências declaradas |
| PAR-30 | Operação simples e confiável | Objetivo próprio do Prumo | Medir aprendizagem, recuperação, acessibilidade e erros críticos |
| PAR-31 | Paredes de concreto | Sistema dedicado TQS com referência à NBR 16055 [TQ-07] | Regras próprias, juntas, encontros, aberturas, telas e transições |
| PAR-32 | Barras unilaterais e tirantes | Módulos e comportamento CYPE 3D [CY-03], [CY-04], [CY-05] | Pré-tensão, perda/reengate de rigidez e solução por combinação |
| PAR-33 | Cálculo isolado | Fundações CYPECAD [CY-01]; calculadora MetalCheck [TQ-04] | Esforços externos rastreáveis, sem superestrutura fictícia |
| PAR-34 | Perfis de alma aligeirada | Perfis Boyd no CYPE 3D [CY-05] | Modelagem e verificações específicas com domínio declarado |
| PAR-35 | Apresentação móvel | Realidade aumentada Eberick/Augin [EB-05] | Exportação e visualização com escala conferível |
| PAR-36 | Intercâmbio analítico | Exportação Eberick/SAP2000 [EB-06] | Conectores qualificados, hipóteses preservadas e perdas explícitas |

## 3. Diferenças entre produto, módulo e versão

O CYPE 3D é um aplicativo distinto do CYPECAD. A documentação descreve exportação para CYPE Connect e StruBIM Steel para trabalho adicional em ligações [CY-02]. Compatibilidade com um aplicativo não equivale a implementar suas funções.

No TQS, PREO, Protensão LV/VPRO, SISEs e MetalCheck correspondem a funções específicas [TQ-03], [TQ-04], [TQ-06]. No Eberick, o comportamento de recursos depende dos módulos e critérios adotados [EB-01], [EB-03]. O catálogo do Prumo deve registrar essa distinção para não comparar uma licença básica com um ecossistema completo.

O artigo metálico do Eberick descreve limitações para perfis compostos e necessidade de conferir comprimentos de flambagem [EB-04]. A documentação MetalCheck consultada cita NBR 8800:2008 [TQ-04]. Esses fatos devem preservar contexto de versão; não sustentam conclusões sobre todas as versões atuais.

O manual PREO consultado trata os painéis de fechamento como objetos de modelagem/documentação, sem calcular sua armadura e sem incluir automaticamente seu peso próprio na análise [TQ-09]. O Prumo precisa distinguir essas operações; eventual dimensionamento adicional exige validação própria.

## 4. Abrangência que o lançamento precisa demonstrar

O núcleo brasileiro inclui NBR 6118, NBR 8800, a série NBR 7190 e normas correlatas aplicáveis, com edições confirmadas no registro normativo. A cobertura inclui os materiais, elementos e situações acima, cada qual vinculado ao seu domínio de validade.

**Alumínio exige critérios próprios.** Não pode ser dimensionado por simples troca de módulo de elasticidade em uma rotina NBR 8800. Material genérico pode participar da análise mediante propriedades declaradas; resultados de esforços não autorizam emitir conclusão automática de resistência.

A existência de uma rotina de concreto não fecha automaticamente punção, paredes, reservatórios ou fundações. Uma rotina de barras metálicas não fecha ligações, perfis formados a frio nem elementos mistos. Cada subdomínio exige entradas, hipóteses, critérios, resultados e contraprovas próprios.

Recursos de normas estrangeiras, formatos nativos, conectores e ecossistemas proprietários devem ser inventariados por especificação pública, versão suportada e licença necessária. A prioridade brasileira não permite anunciar paridade mundial sem evidência. Uma dependência indisponível fica identificada como lacuna; não pode ser substituída por exportação aproximada apresentada como integração integral.

A implementação deve usar conhecimento público e componentes com licença compatível. O levantamento funcional não autoriza copiar código, manuais, bibliotecas de detalhes ou bases proprietárias. Os direitos de leitura e escrita de formatos e de redistribuição de componentes precisam ser resolvidos antes de fixar a solução técnica.

## 5. Evidência mínima por capacidade

Para cada capacidade, o registro de validação precisa apresentar:

1. Definição do comportamento esperado, entradas, saídas e limites de aplicabilidade.
2. Fontes técnicas e normas com edição, item pertinente e responsável pela interpretação.
3. Exemplos independentes, casos adversos e tolerâncias justificadas antes da comparação.
4. Identificação do modelo, versão do motor, parâmetros e resultado reproduzível.
5. Cobertura equivalente na interface e no MCP quando houver operação correspondente.
6. Aprovação técnica e tratamento das divergências, incluindo evidência de que nenhuma verificação relevante foi omitida.

Análise numérica, dimensionamento, interoperabilidade e usabilidade têm critérios diferentes. Não se aceita uma captura de tela como prova de cálculo, nem um teste unitário como prova de facilidade de uso. Concordância com outro programa é evidência complementar: erros comuns ou hipóteses diferentes podem produzir concordâncias e divergências enganosas.

Mudanças relevantes no modelo, material, ação, norma, armadura ou vínculo devem invalidar os resultados dependentes. A edição por agente segue a mesma regra. Resultados não calculados, desatualizados, inconclusivos ou fora do domínio precisam permanecer distinguíveis de resultados aprovados.

## 6. Regra de preservação do escopo

**É proibido reduzir a abrangência, renomear uma pendência ou ampliar tolerâncias apenas para encerrar a última issue.** Cada capacidade precisa estar implementada e validada, ou permanecer aberta como impedimento da alegação de paridade correspondente.

Um recurso pode ser dividido em novas issues para viabilizar o trabalho, mas sua obrigação permanece no critério de lançamento. Recursos de versões posteriores ao recorte podem ser tratados como evolução, desde que não sejam usados na comunicação de equivalência da versão lançada.

Qualquer mudança material do objetivo depende de decisão explícita do proprietário do produto, registrada com impactos nas funcionalidades e na comunicação comercial. Uma versão limitada pode existir para experimentação, mas não substitui por iniciativa da equipe o lançamento completo solicitado.

O encerramento da sequência depende da matriz de cobertura concluída, das evidências técnicas aceitas e da operação integral demonstrada em projetos representativos. A quantidade de issues fechadas, isoladamente, não prova que o sistema está pronto.

## 7. Fontes e rastreabilidade

O arquivo [fontes-manuais.json](fontes-manuais.json) registra editor, título, URL, data de acesso e ressalvas. As referências abaixo apontam ao conteúdo original. Não foram reproduzidos manuais ou textos normativos.

O mapa [coverage.json](../planning/coverage.json) relaciona cada família às issues de implementação e qualificação. Todos os registros começam como não implementados. O vínculo serve ao planejamento e à conferência de cobertura, sem representar evidência de correção já obtida.

[EB-01]: https://www.altoqi.com.br/produtos/eberick
[EB-02]: https://www.altoqi.com.br/produtos/eberick/recursos
[EB-03]: https://suporte.altoqi.com.br/hc/pt-br/articles/4406429041943
[EB-04]: https://suporte.altoqi.com.br/hc/pt-br/articles/360055652454
[CY-01]: https://info.cype.com/br/software/cypecad/
[CY-02]: https://info.cype.com/br/software/cype-3d/
[TQ-01]: https://docs.tqs.com.br/Docs/Details?id=343804092&language=pt-BR
[TQ-02]: https://docs.tqs.com.br/Docs/Details?id=3229&language=pt-BR
[TQ-03]: https://docs.tqs.com.br/Docs/Details?id=1054843974&language=pt-BR
[TQ-04]: https://docs.tqs.com.br/Docs/Details?id=990689351&language=pt-BR
[TQ-05]: https://docs.tqs.com.br/Docs/Details?id=428874089&language=pt-BR
[TQ-06]: https://www.tqs.com.br/%C3%82/systems/tqs-preo/366i9tyvyu
[TQ-07]: https://docs.tqs.com.br/Docs/Details?id=311117186&language=pt-BR
[TQ-08]: https://docs.tqs.com.br/Docs/Details?id=3748&language=pt-BR
[TQ-09]: https://docs.tqs.com.br/Docs/Details?id=2146481243&language=pt-BR
[CY-03]: https://info.cype.com/br/tema/cype-3d-tirantes/
[CY-04]: https://info.cype.com/br/novidades/versao-2024-d/
[CY-05]: https://info.cype.com/br/produto/cype-3d-geometria/
[EB-05]: https://www.altoqi.com.br/recursos/exportacao-do-modelo-3d-em-realidade-aumentada
[EB-06]: https://www.altoqi.com.br/recursos/exportacao-para-o-sap-2000

## Ampliação para instalações na revisão 2.0

A [matriz de instalações](INSTALACOES.md) acrescenta 36 famílias obrigatórias, MEP-01 a MEP-36, às 36 famílias estruturais acima. Cobre oito disciplinas, recursos avançados, catálogos, montagem, cálculos, documentos, coordenação e integração. O [registro rastreável](../planning/coverage.json) vincula todas as 72 famílias à liberação final PRU-250.

O recorte inclui as capacidades documentadas até 14/11/2026. Documentos antigos e novos são reconciliados por função, sem excluir recursos recentes nem inferir cálculos a partir de modelagem. Produtos externos de gestão não são atribuídos automaticamente ao produto de instalações. A pesquisa mantém nomes e fontes para permitir conferência; README e LEIAME apresentam o Prumo por suas próprias capacidades.
