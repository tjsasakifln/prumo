# Prumo: sequência de implementação

**120 issues, implementadas em ordem numérica.** Estado inicial: planejamento. A conclusão da última issue exige produto completo, evidências aceitas e distribuição conferida.

A fila foi criada antes de qualquer PR para preservar a correspondência entre número GitHub e sequência PRU. Novas PRs não alteram os identificadores abaixo.

## Fases

| Issues | Fase |
|---|---|
| #1 a #10 | Base BIM, MCP e verificação |
| #11 a #20 | Modelagem intuitiva |
| #21 a #30 | Motor de análise estrutural |
| #31 a #40 | Ações, combinações e resultados |
| #41 a #50 | Dimensionamento de concreto |
| #51 a #60 | Detalhamento e concreto avançado |
| #61 a #70 | Fundações e interação com o solo |
| #71 a #80 | Aço, mistos e ligações |
| #81 a #90 | Madeira e ligações |
| #91 a #100 | Sistemas complementares e agentes |
| #101 a #110 | BIM, documentos e distribuição |
| #111 a #120 | Qualificação e lançamento |

## Como avançar

Selecionar a menor issue da sequência ainda não concluída, confirmar as anteriores na branch principal e executar seus critérios completos. Uma issue pode precisar de várias PRs. A quantidade de issues não é uma estimativa de prazo.

As etapas intermediárias entregam capacidades de desenvolvimento. Estado analisado, verificado por domínio e liberado para emissão são distintos. O lançamento depende de [LANCAMENTO.md](docs/LANCAMENTO.md) e da matriz de evidências.

A [matriz de cobertura](planning/coverage.json) vincula as capacidades pesquisadas às implementações e campanhas de qualificação. Os dados estruturados da fila estão em [backlog.json](planning/backlog.json).

## Fase 01: Base BIM, MCP e verificação

| Issue | Entrega | Especificação local |
|---|---|---|
| [#1](https://github.com/tjsasakifln/prumo/issues/1) | Fundação executável com projeto BIM e MCP desde a origem | [PRU-001](planning/issues/001.md) |
| [#2](https://github.com/tjsasakifln/prumo/issues/2) | Registro normativo, inventário de paridade e estados de capacidade | [PRU-002](planning/issues/002.md) |
| [#3](https://github.com/tjsasakifln/prumo/issues/3) | Qualificação de componentes abertos e arquitetura de adaptadores | [PRU-003](planning/issues/003.md) |
| [#4](https://github.com/tjsasakifln/prumo/issues/4) | Modelo físico e analítico com identidades e unidades tipadas | [PRU-004](planning/issues/004.md) |
| [#5](https://github.com/tjsasakifln/prumo/issues/5) | Arquivo Prumo, salvamento atômico e recuperação | [PRU-005](planning/issues/005.md) |
| [#6](https://github.com/tjsasakifln/prumo/issues/6) | Comandos transacionais, desfazer e concorrência UI/MCP | [PRU-006](planning/issues/006.md) |
| [#7](https://github.com/tjsasakifln/prumo/issues/7) | MCP local, recursos e controle de acesso por projeto | [PRU-007](planning/issues/007.md) |
| [#8](https://github.com/tjsasakifln/prumo/issues/8) | Infraestrutura de verificação e casos de referência independentes | [PRU-008](planning/issues/008.md) |
| [#9](https://github.com/tjsasakifln/prumo/issues/9) | Geometria robusta, topologia e tolerâncias distintas | [PRU-009](planning/issues/009.md) |
| [#10](https://github.com/tjsasakifln/prumo/issues/10) | Catálogos de materiais, seções e propriedades rastreáveis | [PRU-010](planning/issues/010.md) |

## Fase 02: Modelagem intuitiva

| Issue | Entrega | Especificação local |
|---|---|---|
| [#11](https://github.com/tjsasakifln/prumo/issues/11) | Ambiente 2D/3D, navegação e acessibilidade desde a base | [PRU-011](planning/issues/011.md) |
| [#12](https://github.com/tjsasakifln/prumo/issues/12) | Eixos, níveis, cotas e encaixes com entrada numérica | [PRU-012](planning/issues/012.md) |
| [#13](https://github.com/tjsasakifln/prumo/issues/13) | Peças paramétricas: pilares, vigas, barras e treliças | [PRU-013](planning/issues/013.md) |
| [#14](https://github.com/tjsasakifln/prumo/issues/14) | Lajes, painéis, aberturas e regiões de espessura variável | [PRU-014](planning/issues/014.md) |
| [#15](https://github.com/tjsasakifln/prumo/issues/15) | Paredes, núcleos, escadas e rampas | [PRU-015](planning/issues/015.md) |
| [#16](https://github.com/tjsasakifln/prumo/issues/16) | Apoios, liberações, molas e ligações semirrígidas | [PRU-016](planning/issues/016.md) |
| [#17](https://github.com/tjsasakifln/prumo/issues/17) | Referências CAD, PDF e imagens com escala conferível | [PRU-017](planning/issues/017.md) |
| [#18](https://github.com/tjsasakifln/prumo/issues/18) | Importação IFC e promoção controlada para o modelo estrutural | [PRU-018](planning/issues/018.md) |
| [#19](https://github.com/tjsasakifln/prumo/issues/19) | Conjuntos paramétricos, repetição e edição em lote | [PRU-019](planning/issues/019.md) |
| [#20](https://github.com/tjsasakifln/prumo/issues/20) | Revisão de modelo e fluxo guiado antes do cálculo | [PRU-020](planning/issues/020.md) |

## Fase 03: Motor de análise estrutural

| Issue | Entrega | Especificação local |
|---|---|---|
| [#21](https://github.com/tjsasakifln/prumo/issues/21) | Contrato do solver e solução linear esparsa verificável | [PRU-021](planning/issues/021.md) |
| [#22](https://github.com/tjsasakifln/prumo/issues/22) | Elementos de barras, pórticos e torção em 2D e 3D | [PRU-022](planning/issues/022.md) |
| [#23](https://github.com/tjsasakifln/prumo/issues/23) | Placas, cascas e malha com convergência documentada | [PRU-023](planning/issues/023.md) |
| [#24](https://github.com/tjsasakifln/prumo/issues/24) | Diafragmas, vínculos multiponto e condensação | [PRU-024](planning/issues/024.md) |
| [#25](https://github.com/tjsasakifln/prumo/issues/25) | Transferência de cargas e conservação entre componentes | [PRU-025](planning/issues/025.md) |
| [#26](https://github.com/tjsasakifln/prumo/issues/26) | Segunda ordem geométrica, instabilidade e imperfeições | [PRU-026](planning/issues/026.md) |
| [#27](https://github.com/tjsasakifln/prumo/issues/27) | Não linearidade física e rigidez dependente do estado | [PRU-027](planning/issues/027.md) |
| [#28](https://github.com/tjsasakifln/prumo/issues/28) | Modos próprios, massas e participação modal | [PRU-028](planning/issues/028.md) |
| [#29](https://github.com/tjsasakifln/prumo/issues/29) | Resposta dinâmica, espectral e histórico no tempo | [PRU-029](planning/issues/029.md) |
| [#30](https://github.com/tjsasakifln/prumo/issues/30) | Etapas construtivas, fluência e retração no motor | [PRU-030](planning/issues/030.md) |

## Fase 04: Ações, combinações e resultados

| Issue | Entrega | Especificação local |
|---|---|---|
| [#31](https://github.com/tjsasakifln/prumo/issues/31) | Ações permanentes, variáveis e situações especiais | [PRU-031](planning/issues/031.md) |
| [#32](https://github.com/tjsasakifln/prumo/issues/32) | Combinações ELU e ELS conforme pacote normativo | [PRU-032](planning/issues/032.md) |
| [#33](https://github.com/tjsasakifln/prumo/issues/33) | Vento estático, direções e pressões externas/internas | [PRU-033](planning/issues/033.md) |
| [#34](https://github.com/tjsasakifln/prumo/issues/34) | Efeitos dinâmicos do vento e dados de túnel | [PRU-034](planning/issues/034.md) |
| [#35](https://github.com/tjsasakifln/prumo/issues/35) | Ação sísmica e dimensionamento sismorresistente | [PRU-035](planning/issues/035.md) |
| [#36](https://github.com/tjsasakifln/prumo/issues/36) | Estabilidade global, deslocabilidade e deriva | [PRU-036](planning/issues/036.md) |
| [#37](https://github.com/tjsasakifln/prumo/issues/37) | Deslocamentos imediatos, diferidos e recalques impostos | [PRU-037](planning/issues/037.md) |
| [#38](https://github.com/tjsasakifln/prumo/issues/38) | Vibração em serviço, fadiga e ações repetidas | [PRU-038](planning/issues/038.md) |
| [#39](https://github.com/tjsasakifln/prumo/issues/39) | Ações térmicas, incêndio e situações excepcionais | [PRU-039](planning/issues/039.md) |
| [#40](https://github.com/tjsasakifln/prumo/issues/40) | Exploração de resultados e explicação verificável | [PRU-040](planning/issues/040.md) |

## Fase 05: Dimensionamento de concreto

| Issue | Entrega | Especificação local |
|---|---|---|
| [#41](https://github.com/tjsasakifln/prumo/issues/41) | Concreto: materiais, durabilidade e hipóteses normativas | [PRU-041](planning/issues/041.md) |
| [#42](https://github.com/tjsasakifln/prumo/issues/42) | Concreto: seções e interação N-Mx-My | [PRU-042](planning/issues/042.md) |
| [#43](https://github.com/tjsasakifln/prumo/issues/43) | Concreto: vigas à flexão, força normal e ductilidade | [PRU-043](planning/issues/043.md) |
| [#44](https://github.com/tjsasakifln/prumo/issues/44) | Concreto: cisalhamento, torção e interação em vigas | [PRU-044](planning/issues/044.md) |
| [#45](https://github.com/tjsasakifln/prumo/issues/45) | Concreto: pilares, esbeltez e segunda ordem local | [PRU-045](planning/issues/045.md) |
| [#46](https://github.com/tjsasakifln/prumo/issues/46) | Concreto: lajes maciças, nervuradas e faixas de dimensionamento | [PRU-046](planning/issues/046.md) |
| [#47](https://github.com/tjsasakifln/prumo/issues/47) | Concreto: punção, pilares de borda e capitéis | [PRU-047](planning/issues/047.md) |
| [#48](https://github.com/tjsasakifln/prumo/issues/48) | Concreto: fissuração e serviço com armadura efetiva | [PRU-048](planning/issues/048.md) |
| [#49](https://github.com/tjsasakifln/prumo/issues/49) | Concreto: paredes, núcleos e sistema de paredes moldadas no local | [PRU-049](planning/issues/049.md) |
| [#50](https://github.com/tjsasakifln/prumo/issues/50) | Concreto: regiões descontínuas, consolos e elementos inclinados | [PRU-050](planning/issues/050.md) |

## Fase 06: Detalhamento e concreto avançado

| Issue | Entrega | Especificação local |
|---|---|---|
| [#51](https://github.com/tjsasakifln/prumo/issues/51) | Armaduras de vigas, ancoragens e emendas construtíveis | [PRU-051](planning/issues/051.md) |
| [#52](https://github.com/tjsasakifln/prumo/issues/52) | Armaduras de pilares, encontros e nós congestionados | [PRU-052](planning/issues/052.md) |
| [#53](https://github.com/tjsasakifln/prumo/issues/53) | Armaduras de lajes, paredes, escadas e regiões especiais | [PRU-053](planning/issues/053.md) |
| [#54](https://github.com/tjsasakifln/prumo/issues/54) | Protensão: cabos, traçados e ações equivalentes | [PRU-054](planning/issues/054.md) |
| [#55](https://github.com/tjsasakifln/prumo/issues/55) | Protensão: perdas, ELU, ELS, fadiga e ancoragens | [PRU-055](planning/issues/055.md) |
| [#56](https://github.com/tjsasakifln/prumo/issues/56) | Concreto em incêndio e detalhamento de proteção | [PRU-056](planning/issues/056.md) |
| [#57](https://github.com/tjsasakifln/prumo/issues/57) | Pré-moldados: fases, ligações, apoios e tolerâncias | [PRU-057](planning/issues/057.md) |
| [#58](https://github.com/tjsasakifln/prumo/issues/58) | Lajes pré-fabricadas, alveolares e seções compostas | [PRU-058](planning/issues/058.md) |
| [#59](https://github.com/tjsasakifln/prumo/issues/59) | Reservatórios, piscinas e estruturas enterradas de concreto | [PRU-059](planning/issues/059.md) |
| [#60](https://github.com/tjsasakifln/prumo/issues/60) | Qualificação integrada de concreto e detalhamento | [PRU-060](planning/issues/060.md) |

## Fase 07: Fundações e interação com o solo

| Issue | Entrega | Especificação local |
|---|---|---|
| [#61](https://github.com/tjsasakifln/prumo/issues/61) | Dados geotécnicos, investigação e perfis de solo | [PRU-061](planning/issues/061.md) |
| [#62](https://github.com/tjsasakifln/prumo/issues/62) | Sapatas isoladas e verificações geotécnicas/estruturais | [PRU-062](planning/issues/062.md) |
| [#63](https://github.com/tjsasakifln/prumo/issues/63) | Sapatas associadas, corridas e vigas de equilíbrio | [PRU-063](planning/issues/063.md) |
| [#64](https://github.com/tjsasakifln/prumo/issues/64) | Estacas, tubulões, grupos e esforços laterais | [PRU-064](planning/issues/064.md) |
| [#65](https://github.com/tjsasakifln/prumo/issues/65) | Blocos sobre estacas, bielas, tirantes e arranques | [PRU-065](planning/issues/065.md) |
| [#66](https://github.com/tjsasakifln/prumo/issues/66) | Radiers, pisos estruturais e contato unilateral | [PRU-066](planning/issues/066.md) |
| [#67](https://github.com/tjsasakifln/prumo/issues/67) | Interação integrada solo, fundação e superestrutura | [PRU-067](planning/issues/067.md) |
| [#68](https://github.com/tjsasakifln/prumo/issues/68) | Contenções, empuxos, drenagem e estabilidade | [PRU-068](planning/issues/068.md) |
| [#69](https://github.com/tjsasakifln/prumo/issues/69) | Detalhamento de fundações e compatibilidade com execução | [PRU-069](planning/issues/069.md) |
| [#70](https://github.com/tjsasakifln/prumo/issues/70) | Qualificação independente de fundações e contenções | [PRU-070](planning/issues/070.md) |

## Fase 08: Aço, mistos e ligações

| Issue | Entrega | Especificação local |
|---|---|---|
| [#71](https://github.com/tjsasakifln/prumo/issues/71) | Aço: materiais, perfis e classificação das seções | [PRU-071](planning/issues/071.md) |
| [#72](https://github.com/tjsasakifln/prumo/issues/72) | Aço: tração, compressão, flexão, cisalhamento e interação | [PRU-072](planning/issues/072.md) |
| [#73](https://github.com/tjsasakifln/prumo/issues/73) | Aço: estabilidade, flambagem lateral e análise de segunda ordem | [PRU-073](planning/issues/073.md) |
| [#74](https://github.com/tjsasakifln/prumo/issues/74) | Estruturas mistas: vigas, pilares, lajes e conectores | [PRU-074](planning/issues/074.md) |
| [#75](https://github.com/tjsasakifln/prumo/issues/75) | Ligações metálicas soldadas, parafusadas e semirrígidas | [PRU-075](planning/issues/075.md) |
| [#76](https://github.com/tjsasakifln/prumo/issues/76) | Chapas de base, chumbadores e interface aço-concreto | [PRU-076](planning/issues/076.md) |
| [#77](https://github.com/tjsasakifln/prumo/issues/77) | Perfis tubulares, treliças e nós específicos | [PRU-077](planning/issues/077.md) |
| [#78](https://github.com/tjsasakifln/prumo/issues/78) | Perfis formados a frio e instabilidade local/distorcional | [PRU-078](planning/issues/078.md) |
| [#79](https://github.com/tjsasakifln/prumo/issues/79) | Aço e mistos em incêndio | [PRU-079](planning/issues/079.md) |
| [#80](https://github.com/tjsasakifln/prumo/issues/80) | Detalhamento metálico, listas e qualificação do módulo | [PRU-080](planning/issues/080.md) |

## Fase 09: Madeira e ligações

| Issue | Entrega | Especificação local |
|---|---|---|
| [#81](https://github.com/tjsasakifln/prumo/issues/81) | Madeira: classificação, propriedades e condições de uso | [PRU-081](planning/issues/081.md) |
| [#82](https://github.com/tjsasakifln/prumo/issues/82) | Madeira: barras sob esforços simples e combinados | [PRU-082](planning/issues/082.md) |
| [#83](https://github.com/tjsasakifln/prumo/issues/83) | Madeira: instabilidade, travamentos e segunda ordem | [PRU-083](planning/issues/083.md) |
| [#84](https://github.com/tjsasakifln/prumo/issues/84) | Ligações de madeira e interfaces com aço/concreto | [PRU-084](planning/issues/084.md) |
| [#85](https://github.com/tjsasakifln/prumo/issues/85) | Coberturas, treliças e pórticos de madeira | [PRU-085](planning/issues/085.md) |
| [#86](https://github.com/tjsasakifln/prumo/issues/86) | Madeira laminada colada, peças curvas e seções variáveis | [PRU-086](planning/issues/086.md) |
| [#87](https://github.com/tjsasakifln/prumo/issues/87) | Pisos de madeira, diafragmas, fluência e vibração | [PRU-087](planning/issues/087.md) |
| [#88](https://github.com/tjsasakifln/prumo/issues/88) | Madeira em incêndio e proteção das ligações | [PRU-088](planning/issues/088.md) |
| [#89](https://github.com/tjsasakifln/prumo/issues/89) | Painéis e produtos de madeira engenheirada | [PRU-089](planning/issues/089.md) |
| [#90](https://github.com/tjsasakifln/prumo/issues/90) | Detalhamento, fabricação e qualificação de madeira | [PRU-090](planning/issues/090.md) |

## Fase 10: Sistemas complementares e agentes

| Issue | Entrega | Especificação local |
|---|---|---|
| [#91](https://github.com/tjsasakifln/prumo/issues/91) | Alvenaria estrutural: modelo, materiais e dimensionamento | [PRU-091](planning/issues/091.md) |
| [#92](https://github.com/tjsasakifln/prumo/issues/92) | Alvenaria: modulação, graute, armaduras e detalhamento | [PRU-092](planning/issues/092.md) |
| [#93](https://github.com/tjsasakifln/prumo/issues/93) | Integração entre concreto, aço, madeira, alvenaria e fundações | [PRU-093](planning/issues/093.md) |
| [#94](https://github.com/tjsasakifln/prumo/issues/94) | Alumínio e materiais genéricos com domínio declarado | [PRU-094](planning/issues/094.md) |
| [#95](https://github.com/tjsasakifln/prumo/issues/95) | Grelhas, modelos de pavimentos e escolha de idealização | [PRU-095](planning/issues/095.md) |
| [#96](https://github.com/tjsasakifln/prumo/issues/96) | Alternativas de projeto, quantitativos e otimização verificável | [PRU-096](planning/issues/096.md) |
| [#97](https://github.com/tjsasakifln/prumo/issues/97) | Revisão técnica, ATP e aprovação por revisão de projeto | [PRU-097](planning/issues/097.md) |
| [#98](https://github.com/tjsasakifln/prumo/issues/98) | Cálculo em lote, filas e estudos paramétricos | [PRU-098](planning/issues/098.md) |
| [#99](https://github.com/tjsasakifln/prumo/issues/99) | MCP completo para modelar, calcular, revisar e preparar entregas | [PRU-099](planning/issues/099.md) |
| [#100](https://github.com/tjsasakifln/prumo/issues/100) | Resistência a instruções maliciosas e abuso de ferramentas por agentes | [PRU-100](planning/issues/100.md) |

## Fase 11: BIM, documentos e distribuição

| Issue | Entrega | Especificação local |
|---|---|---|
| [#101](https://github.com/tjsasakifln/prumo/issues/101) | Pranchas automáticas, cortes, detalhes e revisão visual | [PRU-101](planning/issues/101.md) |
| [#102](https://github.com/tjsasakifln/prumo/issues/102) | Memoriais de cálculo, relatórios e quantitativos reconciliados | [PRU-102](planning/issues/102.md) |
| [#103](https://github.com/tjsasakifln/prumo/issues/103) | Intercâmbio DWG/DXF e formatos de fabricação | [PRU-103](planning/issues/103.md) |
| [#104](https://github.com/tjsasakifln/prumo/issues/104) | Exportação IFC físico, analítico e armaduras | [PRU-104](planning/issues/104.md) |
| [#105](https://github.com/tjsasakifln/prumo/issues/105) | BCF, IDS, colisões e coordenação de revisões | [PRU-105](planning/issues/105.md) |
| [#106](https://github.com/tjsasakifln/prumo/issues/106) | Emissão técnica, pacote imutável e rastreio de revisões | [PRU-106](planning/issues/106.md) |
| [#107](https://github.com/tjsasakifln/prumo/issues/107) | Colaboração, permissões e sincronização com conflitos explícitos | [PRU-107](planning/issues/107.md) |
| [#108](https://github.com/tjsasakifln/prumo/issues/108) | Extensões, API estável e ajuda contextual em português | [PRU-108](planning/issues/108.md) |
| [#109](https://github.com/tjsasakifln/prumo/issues/109) | Diagnóstico, privacidade e suporte com dados mínimos | [PRU-109](planning/issues/109.md) |
| [#110](https://github.com/tjsasakifln/prumo/issues/110) | Instaladores, atualização e recuperação de versão | [PRU-110](planning/issues/110.md) |

## Fase 12: Qualificação e lançamento

| Issue | Entrega | Especificação local |
|---|---|---|
| [#111](https://github.com/tjsasakifln/prumo/issues/111) | Desempenho, escala e resposta sob carga | [PRU-111](planning/issues/111.md) |
| [#112](https://github.com/tjsasakifln/prumo/issues/112) | Fluxos completos e regressão entre disciplinas | [PRU-112](planning/issues/112.md) |
| [#113](https://github.com/tjsasakifln/prumo/issues/113) | Campanha independente de verificação de concreto | [PRU-113](planning/issues/113.md) |
| [#114](https://github.com/tjsasakifln/prumo/issues/114) | Campanha independente de aço, madeira, alvenaria, alumínio e geotecnia | [PRU-114](planning/issues/114.md) |
| [#115](https://github.com/tjsasakifln/prumo/issues/115) | Pilotos profissionais e confronto de projetos completos | [PRU-115](planning/issues/115.md) |
| [#116](https://github.com/tjsasakifln/prumo/issues/116) | Auditoria de segurança, cadeia de dependências e MCP | [PRU-116](planning/issues/116.md) |
| [#117](https://github.com/tjsasakifln/prumo/issues/117) | Estudo independente de usabilidade e acessibilidade | [PRU-117](planning/issues/117.md) |
| [#118](https://github.com/tjsasakifln/prumo/issues/118) | Preparação comercial, licenças, termos e manutenção normativa | [PRU-118](planning/issues/118.md) |
| [#119](https://github.com/tjsasakifln/prumo/issues/119) | Candidato de lançamento, distribuição e operação assistida | [PRU-119](planning/issues/119.md) |
| [#120](https://github.com/tjsasakifln/prumo/issues/120) | Lançamento do Prumo com cobertura e confiabilidade demonstradas | [PRU-120](planning/issues/120.md) |
