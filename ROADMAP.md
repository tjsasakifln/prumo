# Prumo: sequência de implementação

**250 issues, implementadas em ordem numérica.** Estado inicial: planejamento. A conclusão da última issue exige estruturas e instalações completas, evidências aceitas e distribuição conferida.

A fila ampliada preserva a correspondência entre número GitHub e sequência PRU. A etapa 120 é um marco estrutural; a liberação comercial pertence à etapa 250. Novas PRs não alteram os identificadores abaixo.

**Decisão de linguagem: Rust.** O requisito REQ-15 e a política TEC-RUST-01 abrangem as 250 entregas. Todo código próprio do produto, incluindo interface, BIM, regras de engenharia, motores, CLI e MCP, será escrito em Rust, com workspace Cargo e toolchain fixada. Integrações de componentes externos e ferramentas do planejamento seguem [RUST.md](docs/RUST.md) e [technology.json](planning/technology.json). Esta decisão registra uma exigência de implementação, sem declarar código já concluído.

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
| #111 a #120 | Qualificação estrutural e preparação das instalações |
| #121 a #130 | Fundação de instalações e coordenação BIM |
| #131 a #145 | Água fria, água quente, reservas e piscinas |
| #146 a #155 | Esgoto, águas pluviais e tratamento |
| #156 a #161 | Instalações prediais de gás |
| #162 a #175 | Hidrantes, sprinklers e sistemas preventivos |
| #176 a #190 | Instalações elétricas e infraestrutura |
| #191 a #200 | Energia fotovoltaica |
| #201 a #208 | Proteção contra descargas atmosféricas |
| #209 a #216 | Cabeamento estruturado e telefonia |
| #217 a #226 | Climatização e redes associadas |
| #227 a #238 | Integração BIM, documentos e operação por agentes |
| #239 a #250 | Qualificação completa e lançamento |

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

## Fase 12: Qualificação estrutural e preparação das instalações

| Issue | Entrega | Especificação local |
|---|---|---|
| [#111](https://github.com/tjsasakifln/prumo/issues/111) | Desempenho e escala do incremento estrutural | [PRU-111](planning/issues/111.md) |
| [#112](https://github.com/tjsasakifln/prumo/issues/112) | Fluxos completos e regressão do incremento estrutural | [PRU-112](planning/issues/112.md) |
| [#113](https://github.com/tjsasakifln/prumo/issues/113) | Campanha independente de verificação de concreto | [PRU-113](planning/issues/113.md) |
| [#114](https://github.com/tjsasakifln/prumo/issues/114) | Campanha independente de aço, madeira, alvenaria, alumínio e geotecnia | [PRU-114](planning/issues/114.md) |
| [#115](https://github.com/tjsasakifln/prumo/issues/115) | Pilotos profissionais do incremento estrutural | [PRU-115](planning/issues/115.md) |
| [#116](https://github.com/tjsasakifln/prumo/issues/116) | Auditoria de segurança, cadeia de dependências e MCP | [PRU-116](planning/issues/116.md) |
| [#117](https://github.com/tjsasakifln/prumo/issues/117) | Estudo de usabilidade e acessibilidade do incremento estrutural | [PRU-117](planning/issues/117.md) |
| [#118](https://github.com/tjsasakifln/prumo/issues/118) | Base comercial, licenças e manutenção normativa estrutural | [PRU-118](planning/issues/118.md) |
| [#119](https://github.com/tjsasakifln/prumo/issues/119) | Candidato interno estrutural, distribuição e recuperação | [PRU-119](planning/issues/119.md) |
| [#120](https://github.com/tjsasakifln/prumo/issues/120) | Consolidação estrutural e preparação das instalações | [PRU-120](planning/issues/120.md) |

## Fase 13: Fundação de instalações e coordenação BIM

| Issue | Entrega | Especificação local |
|---|---|---|
| [#121](https://github.com/tjsasakifln/prumo/issues/121) | Modelo nativo de instalações, ambientes, redes e portas | [PRU-121](planning/issues/121.md) |
| [#122](https://github.com/tjsasakifln/prumo/issues/122) | Catálogo técnico de peças, símbolos e dados de cálculo | [PRU-122](planning/issues/122.md) |
| [#123](https://github.com/tjsasakifln/prumo/issues/123) | Configuração multidisciplinar, níveis e modelos de projeto | [PRU-123](planning/issues/123.md) |
| [#124](https://github.com/tjsasakifln/prumo/issues/124) | Edição sincronizada de instalações em planta, corte e 3D | [PRU-124](planning/issues/124.md) |
| [#125](https://github.com/tjsasakifln/prumo/issues/125) | Topologia, prumadas, continuidade e diagnóstico das redes | [PRU-125](planning/issues/125.md) |
| [#126](https://github.com/tjsasakifln/prumo/issues/126) | Kits reutilizáveis e blocos dinâmicos de instalações | [PRU-126](planning/issues/126.md) |
| [#127](https://github.com/tjsasakifln/prumo/issues/127) | Traçado assistido, encaixes e inserção automática de conexões | [PRU-127](planning/issues/127.md) |
| [#128](https://github.com/tjsasakifln/prumo/issues/128) | Vistas técnicas, indicações e esquemas de rede associados | [PRU-128](planning/issues/128.md) |
| [#129](https://github.com/tjsasakifln/prumo/issues/129) | Base de verificação e qualificação de componentes de instalações | [PRU-129](planning/issues/129.md) |
| [#130](https://github.com/tjsasakifln/prumo/issues/130) | Qualificação da base BIM e MCP para instalações | [PRU-130](planning/issues/130.md) |

## Fase 14: Água fria, água quente, reservas e piscinas

| Issue | Entrega | Especificação local |
|---|---|---|
| [#131](https://github.com/tjsasakifln/prumo/issues/131) | Redes de água fria e quente com conexões verificáveis | [PRU-131](planning/issues/131.md) |
| [#132](https://github.com/tjsasakifln/prumo/issues/132) | Consumo de água e demanda por métodos reconhecidos | [PRU-132](planning/issues/132.md) |
| [#133](https://github.com/tjsasakifln/prumo/issues/133) | Perdas de carga e balanço hidráulico em redes de água | [PRU-133](planning/issues/133.md) |
| [#134](https://github.com/tjsasakifln/prumo/issues/134) | Dimensionamento de diâmetros hidráulicos comerciais | [PRU-134](planning/issues/134.md) |
| [#135](https://github.com/tjsasakifln/prumo/issues/135) | Pontos críticos e simulador de pressões hidráulicas | [PRU-135](planning/issues/135.md) |
| [#136](https://github.com/tjsasakifln/prumo/issues/136) | Reservação de água e interligação de reservatórios | [PRU-136](planning/issues/136.md) |
| [#137](https://github.com/tjsasakifln/prumo/issues/137) | Alimentação predial e medição coletiva e individual | [PRU-137](planning/issues/137.md) |
| [#138](https://github.com/tjsasakifln/prumo/issues/138) | Bombas de recalque e curva do sistema de água | [PRU-138](planning/issues/138.md) |
| [#139](https://github.com/tjsasakifln/prumo/issues/139) | Pressurizadores e válvulas redutoras de pressão | [PRU-139](planning/issues/139.md) |
| [#140](https://github.com/tjsasakifln/prumo/issues/140) | Água quente e atendimento de pontos mistos | [PRU-140](planning/issues/140.md) |
| [#141](https://github.com/tjsasakifln/prumo/issues/141) | Aquecedores de passagem e reservatórios térmicos | [PRU-141](planning/issues/141.md) |
| [#142](https://github.com/tjsasakifln/prumo/issues/142) | Aquecimento solar de água | [PRU-142](planning/issues/142.md) |
| [#143](https://github.com/tjsasakifln/prumo/issues/143) | Distribuidores PEX e condutos flexíveis | [PRU-143](planning/issues/143.md) |
| [#144](https://github.com/tjsasakifln/prumo/issues/144) | Montagem automática de ambientes hidráulicos | [PRU-144](planning/issues/144.md) |
| [#145](https://github.com/tjsasakifln/prumo/issues/145) | Piscinas e qualificação do módulo hidráulico | [PRU-145](planning/issues/145.md) |

## Fase 15: Esgoto, águas pluviais e tratamento

| Issue | Entrega | Especificação local |
|---|---|---|
| [#146](https://github.com/tjsasakifln/prumo/issues/146) | Esgoto predial e dimensionamento de ramais e coletores | [PRU-146](planning/issues/146.md) |
| [#147](https://github.com/tjsasakifln/prumo/issues/147) | Ventilação sanitária e proteção dos desconectores | [PRU-147](planning/issues/147.md) |
| [#148](https://github.com/tjsasakifln/prumo/issues/148) | Declividades, caixas e continuidade da rede sanitária | [PRU-148](planning/issues/148.md) |
| [#149](https://github.com/tjsasakifln/prumo/issues/149) | Estações elevatórias sanitárias e pluviais | [PRU-149](planning/issues/149.md) |
| [#150](https://github.com/tjsasakifln/prumo/issues/150) | Contribuições pluviais e dados de precipitação | [PRU-150](planning/issues/150.md) |
| [#151](https://github.com/tjsasakifln/prumo/issues/151) | Calhas, condutores e coletores de águas pluviais | [PRU-151](planning/issues/151.md) |
| [#152](https://github.com/tjsasakifln/prumo/issues/152) | Aproveitamento de água de chuva e reservação não potável | [PRU-152](planning/issues/152.md) |
| [#153](https://github.com/tjsasakifln/prumo/issues/153) | Tanques sépticos e contribuição de tratamento local | [PRU-153](planning/issues/153.md) |
| [#154](https://github.com/tjsasakifln/prumo/issues/154) | Tratamento complementar e disposição no solo | [PRU-154](planning/issues/154.md) |
| [#155](https://github.com/tjsasakifln/prumo/issues/155) | Qualificação sanitária, pluvial e de tratamento | [PRU-155](planning/issues/155.md) |

## Fase 16: Instalações prediais de gás

| Issue | Entrega | Especificação local |
|---|---|---|
| [#156](https://github.com/tjsasakifln/prumo/issues/156) | Modelagem de gás GN, GLP e redes intercambiáveis | [PRU-156](planning/issues/156.md) |
| [#157](https://github.com/tjsasakifln/prumo/issues/157) | Potências, vazões e simultaneidade de gás | [PRU-157](planning/issues/157.md) |
| [#158](https://github.com/tjsasakifln/prumo/issues/158) | Dimensionamento e verificação de pressões de gás | [PRU-158](planning/issues/158.md) |
| [#159](https://github.com/tjsasakifln/prumo/issues/159) | Reguladores e medidores de gás | [PRU-159](planning/issues/159.md) |
| [#160](https://github.com/tjsasakifln/prumo/issues/160) | Central GLP e seleção de recipientes | [PRU-160](planning/issues/160.md) |
| [#161](https://github.com/tjsasakifln/prumo/issues/161) | Qualificação do módulo de gás | [PRU-161](planning/issues/161.md) |

## Fase 17: Hidrantes, sprinklers e sistemas preventivos

| Issue | Entrega | Especificação local |
|---|---|---|
| [#162](https://github.com/tjsasakifln/prumo/issues/162) | Critérios de incêndio por ocupação e jurisdição | [PRU-162](planning/issues/162.md) |
| [#163](https://github.com/tjsasakifln/prumo/issues/163) | Redes de hidrantes e mangotinhos | [PRU-163](planning/issues/163.md) |
| [#164](https://github.com/tjsasakifln/prumo/issues/164) | Simultaneidade e hidrantes favoráveis e desfavoráveis | [PRU-164](planning/issues/164.md) |
| [#165](https://github.com/tjsasakifln/prumo/issues/165) | Alcance de mangueiras e cobertura de hidrantes | [PRU-165](planning/issues/165.md) |
| [#166](https://github.com/tjsasakifln/prumo/issues/166) | Reserva técnica de incêndio e alimentação | [PRU-166](planning/issues/166.md) |
| [#167](https://github.com/tjsasakifln/prumo/issues/167) | Bombas principais de incêndio e verificação de operação | [PRU-167](planning/issues/167.md) |
| [#168](https://github.com/tjsasakifln/prumo/issues/168) | Bomba jockey e manutenção de pressão | [PRU-168](planning/issues/168.md) |
| [#169](https://github.com/tjsasakifln/prumo/issues/169) | Lançamento de sprinklers e componentes da rede | [PRU-169](planning/issues/169.md) |
| [#170](https://github.com/tjsasakifln/prumo/issues/170) | Risco e áreas de operação de sprinklers | [PRU-170](planning/issues/170.md) |
| [#171](https://github.com/tjsasakifln/prumo/issues/171) | Dimensionamento hidráulico e por tabelas de sprinklers | [PRU-171](planning/issues/171.md) |
| [#172](https://github.com/tjsasakifln/prumo/issues/172) | Redes de incêndio em anel, malha e grelha | [PRU-172](planning/issues/172.md) |
| [#173](https://github.com/tjsasakifln/prumo/issues/173) | Sistemas integrados de sprinklers e hidrantes | [PRU-173](planning/issues/173.md) |
| [#174](https://github.com/tjsasakifln/prumo/issues/174) | Preventivos: extintores, detecção, alarme e emergência | [PRU-174](planning/issues/174.md) |
| [#175](https://github.com/tjsasakifln/prumo/issues/175) | Qualificação de hidrantes, sprinklers e preventivos | [PRU-175](planning/issues/175.md) |

## Fase 18: Instalações elétricas e infraestrutura

| Issue | Entrega | Especificação local |
|---|---|---|
| [#176](https://github.com/tjsasakifln/prumo/issues/176) | Modelo elétrico, circuitos e esquemas de ligação | [PRU-176](planning/issues/176.md) |
| [#177](https://github.com/tjsasakifln/prumo/issues/177) | Pontos elétricos, comandos e distribuição de tomadas | [PRU-177](planning/issues/177.md) |
| [#178](https://github.com/tjsasakifln/prumo/issues/178) | Encaminhamento automático de condutos e redes múltiplas | [PRU-178](planning/issues/178.md) |
| [#179](https://github.com/tjsasakifln/prumo/issues/179) | Fiação automática, edição e verificação de conectividade | [PRU-179](planning/issues/179.md) |
| [#180](https://github.com/tjsasakifln/prumo/issues/180) | Quadros, entrada de serviço e padrões de concessionárias | [PRU-180](planning/issues/180.md) |
| [#181](https://github.com/tjsasakifln/prumo/issues/181) | Demandas, correntes e balanceamento de fases | [PRU-181](planning/issues/181.md) |
| [#182](https://github.com/tjsasakifln/prumo/issues/182) | Seções de condutores e fatores de correção | [PRU-182](planning/issues/182.md) |
| [#183](https://github.com/tjsasakifln/prumo/issues/183) | Queda de tensão por trecho, circuito e alimentador | [PRU-183](planning/issues/183.md) |
| [#184](https://github.com/tjsasakifln/prumo/issues/184) | Proteções contra sobrecarga e curto-circuito | [PRU-184](planning/issues/184.md) |
| [#185](https://github.com/tjsasakifln/prumo/issues/185) | DR, IDR, DPS e desconexão associada | [PRU-185](planning/issues/185.md) |
| [#186](https://github.com/tjsasakifln/prumo/issues/186) | Dimensionamento de condutos e ocupação elétrica | [PRU-186](planning/issues/186.md) |
| [#187](https://github.com/tjsasakifln/prumo/issues/187) | Barramento blindado e derivações | [PRU-187](planning/issues/187.md) |
| [#188](https://github.com/tjsasakifln/prumo/issues/188) | Geradores e quadros de transferência automática | [PRU-188](planning/issues/188.md) |
| [#189](https://github.com/tjsasakifln/prumo/issues/189) | Transformadores, nobreaks, recarga veicular e subestações modeladas | [PRU-189](planning/issues/189.md) |
| [#190](https://github.com/tjsasakifln/prumo/issues/190) | Documentação elétrica e verificação integrada do módulo | [PRU-190](planning/issues/190.md) |

## Fase 19: Energia fotovoltaica

| Issue | Entrega | Especificação local |
|---|---|---|
| [#191](https://github.com/tjsasakifln/prumo/issues/191) | Componentes e modelagem fotovoltaica | [PRU-191](planning/issues/191.md) |
| [#192](https://github.com/tjsasakifln/prumo/issues/192) | Séries, arranjos e associação fotovoltaica | [PRU-192](planning/issues/192.md) |
| [#193](https://github.com/tjsasakifln/prumo/issues/193) | Compatibilidade e dimensionamento de inversores | [PRU-193](planning/issues/193.md) |
| [#194](https://github.com/tjsasakifln/prumo/issues/194) | Cabos e proteções do lado CC fotovoltaico | [PRU-194](planning/issues/194.md) |
| [#195](https://github.com/tjsasakifln/prumo/issues/195) | Integração fotovoltaica com a rede CA | [PRU-195](planning/issues/195.md) |
| [#196](https://github.com/tjsasakifln/prumo/issues/196) | Sistemas isolados, controladores e baterias | [PRU-196](planning/issues/196.md) |
| [#197](https://github.com/tjsasakifln/prumo/issues/197) | Perfis fotovoltaicos e projetos de usinas | [PRU-197](planning/issues/197.md) |
| [#198](https://github.com/tjsasakifln/prumo/issues/198) | Estimativa de geração e desempenho fotovoltaico | [PRU-198](planning/issues/198.md) |
| [#199](https://github.com/tjsasakifln/prumo/issues/199) | Retorno econômico fotovoltaico e cenários | [PRU-199](planning/issues/199.md) |
| [#200](https://github.com/tjsasakifln/prumo/issues/200) | Documentação e verificação integrada fotovoltaica | [PRU-200](planning/issues/200.md) |

## Fase 20: Proteção contra descargas atmosféricas

| Issue | Entrega | Especificação local |
|---|---|---|
| [#201](https://github.com/tjsasakifln/prumo/issues/201) | Avaliação de risco de descargas atmosféricas | [PRU-201](planning/issues/201.md) |
| [#202](https://github.com/tjsasakifln/prumo/issues/202) | Geometria de exposição e proteção por ângulo | [PRU-202](planning/issues/202.md) |
| [#203](https://github.com/tjsasakifln/prumo/issues/203) | Malhas de captação e distribuição automática | [PRU-203](planning/issues/203.md) |
| [#204](https://github.com/tjsasakifln/prumo/issues/204) | Método eletrogeométrico e combinação de proteções | [PRU-204](planning/issues/204.md) |
| [#205](https://github.com/tjsasakifln/prumo/issues/205) | Descidas, cordoalhas e anéis intermediários | [PRU-205](planning/issues/205.md) |
| [#206](https://github.com/tjsasakifln/prumo/issues/206) | SPDA estrutural e captores naturais | [PRU-206](planning/issues/206.md) |
| [#207](https://github.com/tjsasakifln/prumo/issues/207) | Aterramento, equipotencialização e proteção interna | [PRU-207](planning/issues/207.md) |
| [#208](https://github.com/tjsasakifln/prumo/issues/208) | Memoriais e verificação integrada de SPDA | [PRU-208](planning/issues/208.md) |

## Fase 21: Cabeamento estruturado e telefonia

| Issue | Entrega | Especificação local |
|---|---|---|
| [#209](https://github.com/tjsasakifln/prumo/issues/209) | Redes de dados, voz, telefonia, TV e interfone | [PRU-209](planning/issues/209.md) |
| [#210](https://github.com/tjsasakifln/prumo/issues/210) | Pontos de comunicação e identificação automática | [PRU-210](planning/issues/210.md) |
| [#211](https://github.com/tjsasakifln/prumo/issues/211) | Encaminhamento e passagem de cabos de comunicação | [PRU-211](planning/issues/211.md) |
| [#212](https://github.com/tjsasakifln/prumo/issues/212) | Dimensionamento de cabos, pares e enlaces | [PRU-212](planning/issues/212.md) |
| [#213](https://github.com/tjsasakifln/prumo/issues/213) | Dimensionamento e organização de racks | [PRU-213](planning/issues/213.md) |
| [#214](https://github.com/tjsasakifln/prumo/issues/214) | Associação de portas e conexões lógicas | [PRU-214](planning/issues/214.md) |
| [#215](https://github.com/tjsasakifln/prumo/issues/215) | Mapa de cabos e documentação de comunicação | [PRU-215](planning/issues/215.md) |
| [#216](https://github.com/tjsasakifln/prumo/issues/216) | Qualificação de cabeamento e redes complementares | [PRU-216](planning/issues/216.md) |

## Fase 22: Climatização e redes associadas

| Issue | Entrega | Especificação local |
|---|---|---|
| [#217](https://github.com/tjsasakifln/prumo/issues/217) | Ambientes e entradas de cálculo térmico | [PRU-217](planning/issues/217.md) |
| [#218](https://github.com/tjsasakifln/prumo/issues/218) | Carga térmica sensível por ambiente | [PRU-218](planning/issues/218.md) |
| [#219](https://github.com/tjsasakifln/prumo/issues/219) | Carga latente, psicrometria e carga total | [PRU-219](planning/issues/219.md) |
| [#220](https://github.com/tjsasakifln/prumo/issues/220) | Equipamentos e acessórios de climatização | [PRU-220](planning/issues/220.md) |
| [#221](https://github.com/tjsasakifln/prumo/issues/221) | Linhas frigogênicas e sistemas Split e VRF | [PRU-221](planning/issues/221.md) |
| [#222](https://github.com/tjsasakifln/prumo/issues/222) | Dutos e distribuição de ar | [PRU-222](planning/issues/222.md) |
| [#223](https://github.com/tjsasakifln/prumo/issues/223) | Tubulação de água gelada e circuitos de climatização | [PRU-223](planning/issues/223.md) |
| [#224](https://github.com/tjsasakifln/prumo/issues/224) | Drenagem de condensado e bombas de dreno | [PRU-224](planning/issues/224.md) |
| [#225](https://github.com/tjsasakifln/prumo/issues/225) | Detalhamento e quantitativos de climatização | [PRU-225](planning/issues/225.md) |
| [#226](https://github.com/tjsasakifln/prumo/issues/226) | Qualificação de climatização e carga térmica | [PRU-226](planning/issues/226.md) |

## Fase 23: Integração BIM, documentos e operação por agentes

| Issue | Entrega | Especificação local |
|---|---|---|
| [#227](https://github.com/tjsasakifln/prumo/issues/227) | Integração de cargas, equipamentos e resultados entre disciplinas | [PRU-227](planning/issues/227.md) |
| [#228](https://github.com/tjsasakifln/prumo/issues/228) | Coordenação de redes, folgas, passagens e autorização estrutural | [PRU-228](planning/issues/228.md) |
| [#229](https://github.com/tjsasakifln/prumo/issues/229) | Referências CAD e BIM, RVT/RTE e planificação seletiva | [PRU-229](planning/issues/229.md) |
| [#230](https://github.com/tjsasakifln/prumo/issues/230) | Intercâmbio IFC de sistemas e quantitativos de instalações | [PRU-230](planning/issues/230.md) |
| [#231](https://github.com/tjsasakifln/prumo/issues/231) | Revisões colaborativas, BCF, IDS e mesclagem multidisciplinar | [PRU-231](planning/issues/231.md) |
| [#232](https://github.com/tjsasakifln/prumo/issues/232) | Quantitativos completos, composições e entrega para orçamento | [PRU-232](planning/issues/232.md) |
| [#233](https://github.com/tjsasakifln/prumo/issues/233) | Pranchas e detalhes executivos de todas as instalações | [PRU-233](planning/issues/233.md) |
| [#234](https://github.com/tjsasakifln/prumo/issues/234) | Memoriais multidisciplinares e pacote técnico coerente | [PRU-234](planning/issues/234.md) |
| [#235](https://github.com/tjsasakifln/prumo/issues/235) | Bibliotecas de fabricantes, atualização e portabilidade do acervo | [PRU-235](planning/issues/235.md) |
| [#236](https://github.com/tjsasakifln/prumo/issues/236) | Paridade completa da interface, MCP e rotinas multidisciplinares | [PRU-236](planning/issues/236.md) |
| [#237](https://github.com/tjsasakifln/prumo/issues/237) | Desempenho e recuperação em projetos multidisciplinares grandes | [PRU-237](planning/issues/237.md) |
| [#238](https://github.com/tjsasakifln/prumo/issues/238) | Aceite funcional integrado de estruturas e oito disciplinas | [PRU-238](planning/issues/238.md) |

## Fase 24: Qualificação completa e lançamento

| Issue | Entrega | Especificação local |
|---|---|---|
| [#239](https://github.com/tjsasakifln/prumo/issues/239) | Campanha independente hidrossanitária, piscinas e tratamento | [PRU-239](planning/issues/239.md) |
| [#240](https://github.com/tjsasakifln/prumo/issues/240) | Campanha independente de gás e proteção contra incêndio | [PRU-240](planning/issues/240.md) |
| [#241](https://github.com/tjsasakifln/prumo/issues/241) | Campanha independente de instalações elétricas | [PRU-241](planning/issues/241.md) |
| [#242](https://github.com/tjsasakifln/prumo/issues/242) | Campanha independente de fotovoltaico e SPDA | [PRU-242](planning/issues/242.md) |
| [#243](https://github.com/tjsasakifln/prumo/issues/243) | Campanha independente de cabeamento e climatização | [PRU-243](planning/issues/243.md) |
| [#244](https://github.com/tjsasakifln/prumo/issues/244) | Pilotos profissionais integrados de estruturas e instalações | [PRU-244](planning/issues/244.md) |
| [#245](https://github.com/tjsasakifln/prumo/issues/245) | Estudo final de usabilidade nas onze famílias de tarefas | [PRU-245](planning/issues/245.md) |
| [#246](https://github.com/tjsasakifln/prumo/issues/246) | Auditoria final de segurança, agentes e cadeia de componentes | [PRU-246](planning/issues/246.md) |
| [#247](https://github.com/tjsasakifln/prumo/issues/247) | Consolidação normativa, licenças e oferta do produto ampliado | [PRU-247](planning/issues/247.md) |
| [#248](https://github.com/tjsasakifln/prumo/issues/248) | Distribuição, desempenho e recuperação do conjunto completo | [PRU-248](planning/issues/248.md) |
| [#249](https://github.com/tjsasakifln/prumo/issues/249) | Candidato final e aceite técnico de todas as disciplinas | [PRU-249](planning/issues/249.md) |
| [#250](https://github.com/tjsasakifln/prumo/issues/250) | Lançamento do Prumo completo para estruturas e instalações | [PRU-250](planning/issues/250.md) |
