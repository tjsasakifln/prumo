# Reuso de componentes para instalações

**Estado: triagem documental de 13 candidatos adicionais. Nenhum foi incorporado ou validado para o Prumo nesta sessão.** A política geral permanece em [REUSO.md](REUSO.md). O [catálogo estruturado](candidatos-reuso.json) reúne os 15 candidatos iniciais e os 13 acrescentados.

A escolha depende do módulo, do domínio, das hipóteses e da forma de distribuição. A issue 3 prepara a estratégia e a 129 qualifica os componentes usados pelas instalações. Cada adoção posterior fixa versão, conserva procedência e avisos e executa os testes específicos.

| Candidato | Uso possível | Condição de qualificação |
|---|---|---|
| [EPANET](https://github.com/USEPA/EPANET2.2) | Redes de água, bombas, válvulas e equilíbrio hidráulico | Qualificar regimes, adaptação predial, unidades e convergência; não aplicar automaticamente a gás |
| [WNTR](https://github.com/USEPA/WNTR) | Preparação de casos, cenários e análise de redes de água | EpanetSimulator compartilha EPANET; não é referência independente do mesmo núcleo produtivo |
| [SWMM](https://github.com/USEPA/Stormwater-Management-Model) | Hidrologia e escoamento, comparação delimitada de pluvial | Não substitui por si só as regras prediais de esgoto, ventilação ou captação |
| [fluids](https://github.com/CalebBell/fluids) | Atrito, perdas localizadas, bombas e correlações de fluidos | Verificar correlação, condições de referência, fluido, regime e domínio por função |
| [pandapipes](https://github.com/e2nIEE/pandapipes) | Redes de água, gás e calor | Qualificar cada fluido e regime; nenhuma conformidade NBR 15526 automática |
| [pandapower](https://github.com/e2nIEE/pandapower) | Redes elétricas e comparação de métodos de curto | Adaptar e provar o domínio predial, fatores, proteções e regras brasileiras |
| [pvlib-python](https://github.com/pvlib/pvlib-python) | Posição solar, geração fotovoltaica e parâmetros de equipamentos | Não substitui proteção elétrica, conexão à concessionária nem regra econômica local |
| [PsychroLib](https://github.com/psychrometrics/psychrolib) | Propriedades psicrométricas e parcelas pertinentes de climatização | Conferir unidades, pressão, temperatura, umidade e hipóteses do método normativo |
| [CoolProp](https://github.com/CoolProp/CoolProp) | Propriedades termofísicas de fluidos | Não é dimensionador de VRF nem certificação de equipamento; qualificar fluido e faixa |
| [petgraph](https://github.com/petgraph/petgraph) | Grafos de redes e caminhos no domínio Rust | Testar multigrafos, portas, identidade, caminhos e escala; não contém regras de engenharia |
| [NetworkX](https://github.com/networkx/networkx) | Referência de topologia e algoritmos de grafo | Independência exige implementação e preparação distintas; não é solver hidráulico/elétrico |
| [D3](https://github.com/d3/d3) | Diagramas e apresentação de resultados | Manter valores, conexões e identidades; verificar acessibilidade e atualização |
| [Dagre](https://github.com/dagrejs/dagre) | Disposição automática de diagramas | Algoritmo de layout não pode mudar rede, ligação ou cálculo |

## Independência e adaptação

Um motor adotado em produção não passa a ser referência independente por usar outra interface. A preparação do caso também precisa de conferência: duas entradas geradas pelo mesmo adaptador podem repetir o mesmo erro. EPANET e ferramentas apoiadas nele devem ser classificados pela implementação efetivamente executada.

O reuso de funções de perda de carga não autoriza aplicar água incompressível a qualquer gás. Uma biblioteca elétrica não implementa automaticamente demanda, proteção e práticas de concessionárias brasileiras. Propriedades térmicas não demonstram adequação de equipamento; energia solar estimada não comprova atendimento elétrico ou retorno financeiro. Cada uma dessas interfaces recebe critérios próprios no backlog.

## Licenças, dados e formatos

O catálogo registra a licença observada ou a pendência concreta. A decisão de distribuição usa os arquivos, dependências e versões que serão incorporados. Não presumir que todo o repositório possui a mesma licença ou que processo separado elimina obrigações.

Para RVT/RTE e DWG, avaliar SDK/conversor autorizado e alternativas com desempenho e compatibilidade medidos. A capacidade só será aceita com importação real das versões declaradas. Catálogos de fabricantes, símbolos, curvas e bancos meteorológicos também exigem origem e direitos de uso.

Nomes genéricos como biblioteca térmica, gerador de documentos ou motor de regras nas issues indicam uma função a pesquisar. Eles não representam um componente aprovado. As pendências nominais anteriores continuam no [registro de pesquisa pendente](../planning/reuse-pending.json).

## Implementação própria em Rust

REQ-15 aplica-se a todas as disciplinas. As bibliotecas Python, C/C++, JavaScript ou de outras linguagens desta pesquisa continuam candidatas a funções externas qualificadas ou referências independentes. O domínio de redes, os cálculos próprios, a interface, os comandos, o MCP e os adaptadores desenvolvidos pelo Prumo serão Rust. A escolha entre FFI, processo isolado e WASM deve comprovar unidades, conversões, falhas, limites, licença e empacotamento, conforme [RUST.md](RUST.md). O componente externo não recebe autoridade direta sobre o modelo do projeto.
