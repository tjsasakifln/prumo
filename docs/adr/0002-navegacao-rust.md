# ADR 0002: navegação compartilhada em planta, corte e 3D

**Data:** 19/11/2026. **Entrega:** PRU-011. **Decisão:** manter egui/eframe 0.37.0 com glow, usar projeção ortográfica em Rust e compartilhar comandos de vista entre aplicação nativa, MCP e visualizador web experimental. A decisão não libera cálculo nem encerra a campanha de acessibilidade.

## Caso avaliado

Um mesmo projeto contém uma viga térrea, um pilar inclinado, uma viga superior e uma travessa. A seleção deve preservar o UUID quando o usuário muda de planta para corte ou 3D, oculta uma peça, seleciona a peça oculta pela lista e a localiza novamente. A navegação não pode alterar revisão, autoria ou evidência técnica. Coordenadas remotas precisam conservar a separação entre peças antes de qualquer conversão gráfica para precisão simples.

## Alternativas e decisão

| Candidato | Avaliação para o caso | Decisão desta entrega |
|---|---|---|
| egui/eframe 0.37.0 e glow | Já fazem parte do runtime qualificado da base. Widgets e controladores são Rust; WebRunner permite reutilizar a mesma representação em WASM. Acessibilidade do canvas web tem limites documentados. | Reutilizar o contrato existente e acrescentar controles DOM de navegação escritos em Rust para a consulta web. Sem fork nem código de fornecedor copiado. |
| Dioxus, documentação da família 0.7 | Oferece autoria Rust e semântica DOM. O destino desktop usa WebView; o renderizador nativo alternativo é apresentado como experimental. Ainda exigiria integrar o desenho BIM e qualificar seleção, precisão e recuperação. | Não incorporar nesta etapa. A troca acrescentaria outro runtime e invalidaria a evidência inicial sem resolver sozinha os critérios geométricos. |
| wgpu | API gráfica com destinos nativos e web; não fornece por si só árvore de objetos, formulários ou semântica assistiva. | Não substituir glow nesta etapa. Continua candidato quando houver um limite medido no renderizador atual. Não há comparação de desempenho entre motores nesta entrega. |
| AccessKit de egui e controles HTML semânticos | AccessKit fornece a árvore nativa exercitada pelos testes. O HTML usa botões, lista, rótulos e regiões de estado; callbacks e validações permanecem em Rust/WASM. | Adaptar por destino. Uma árvore de acessibilidade automatizada não é um ensaio humano com leitor de tela. |

## Fronteiras

`prumo-domain::view` recebe um `Project` imutável e mantém `ViewState` separado. `ViewCommand` é serializável e `apply` valida em uma cópia antes de publicar o novo estado. `ViewReport` inclui UUID do projeto, revisão e SHA-256 canônico do modelo consultado. O estado de vista não integra o modelo técnico nem a revisão persistida.

Planta usa XY; corte usa XZ; a vista 3D usa câmera ortográfica ajustável. O recorte é um intervalo em X, Y ou Z. Todos os cálculos de projeção e seleção usam `f64` relativo ao centro da câmera. Somente posições locais de tela são convertidas para `f32` no painter. A cena usa segmentos e contornos das peças reais das etapas 12 a 15. Esta entrega não oferece renderização sólida com ocultação de faces, perspectiva ou modelo de análise adicional criado pela vista.

UUIDs ligam a seleção à árvore, às propriedades e às três vistas. O usuário pode selecionar ocultos pela lista, alternar peças sobrepostas por cliques sucessivos e navegar por Anterior/Próximo. Selecionar e localizar revela o objeto e remove um recorte que o escondia. Ocultar e isolar um conjunto também considera seus componentes, sem transformar as faces em material adicional.

O serviço local aplica autorização e a base exata do arquivo apresentado. A exportação web gera uma cópia JSON no diretório escolhido pelo host, com nome derivado de revisão e hash; as ferramentas não recebem caminhos arbitrários. O navegador importa essa cópia, valida a representação JSON e o projeto, preserva identidades e revisão e opera somente uma sessão de consulta em memória. Recursos HTML/CSS são declarativos; o JavaScript distribuído é bootstrap e binding gerado por wasm-bindgen 0.2.130.

## Recuperação e acessibilidade

O estado gráfico de falha mantém o projeto e a seleção disponíveis. A aplicação oferece comandos explícitos de restaurar vista, lista textual e propriedades. O destino web registra perda de contexto WebGL, mantém os controles DOM e tenta reiniciar WebRunner usando o mesmo estado. O teste de injeção de estado de falha não prova recuperação de um driver real. A perda real do contexto e a abertura do mesmo projeto em browser precisam da execução do ensaio de CI.

Tab, Enter, setas e botões substituem arrastes obrigatórios. Seleção também aparece em texto; foco tem contorno visível; a escala e o alto contraste são preferências de sessão. O teste egui_kittest verifica ações e nomes pela árvore AccessKit. O visualizador oferece uma lista DOM para tecnologias assistivas compatíveis. A aceitação com leitor de tela humano permanece pendente por plataforma.

## Evidência e condições para rever

Os ensaios e seus limites estão em [PRU-011](../evidencias/PRU-011.md). A decisão será revista se uma cena representativa exceder os limites medidos de seleção, memória, tempo, recuperação ou acessibilidade. Aprovação do código não substitui execução dos backends, publicação dos artefatos ou a dependência sequencial da issue.

## Fontes primárias

- [eframe 0.37.0: WebRunner, glow e features](https://docs.rs/eframe/0.37.0/eframe/).
- [eframe: manifesto publicado com features de acessibilidade](https://docs.rs/crate/eframe/0.37.0/source/Cargo.toml.orig).
- [egui: código e limites de acessibilidade](https://github.com/emilk/egui).
- [Dioxus: destino desktop da família 0.7](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop/) e [estrutura de destinos](https://dioxuslabs.com/learn/0.7/beyond/project_structure/).
- [wgpu: API oficial](https://wgpu.rs/doc/wgpu/).
