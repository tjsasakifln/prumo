# ADR 0001: aplicação Rust local e domínio compartilhado

**Data:** 14/11/2026. **Entrega:** PRU-001. **Decisão:** interface nativa `egui/eframe`, serviço comum de comandos e servidor MCP `rmcp` em processo local. A decisão arquitetural não libera cálculo estrutural, dimensionamento nem emissão técnica.

## Contexto e caso de comparação

O Prumo começa com um projeto BIM pequeno, editável pela interface e por MCP. O caso usado para comparar as opções é o mesmo: criar projeto, escolher metro/centímetro/milímetro, inserir uma barra entre dois pontos, visualizar sua representação, salvar, encerrar, reabrir, consultar e alterar pelo outro canal. As identidades física e analítica e a revisão devem sobreviver. Uma coordenada inválida deve ser rejeitada sem alterar o projeto. A autorização de escrita deve valer antes da primeira operação.

Rust é uma decisão do produto, não uma alternativa avaliada neste ADR. A comparação considera autoria de interface e regras em Rust. O quadro abaixo contém avaliação documental dos candidatos. Somente a opção selecionada recebe implementação e ensaios nesta entrega; o quadro não apresenta um teste de desempenho ou acessibilidade dos outros frameworks.

## Alternativas no mesmo caso BIM

| Critério do caso | egui/eframe nativo, escolhido | Dioxus com WebView | Interface Rust/WASM em Tauri |
|---|---|---|---|
| Criar e editar a barra | Widgets Rust chamam diretamente o serviço comum; desenho derivado dos pontos do domínio | Componentes RSX em Rust podem chamar o mesmo serviço; estado de interação e DOM não podem virar outro modelo | Comandos tipados atravessam a fronteira WASM/host; o host Rust valida autorização e revisão |
| Salvar, encerrar e reabrir | Serviço Rust local independente do renderizador e de internet | Serviço Rust local, acrescido do runtime WebView do sistema | Serviço Rust no host; acesso a arquivo restrito pelas capacidades do contêiner |
| Consultar e alterar pelo MCP | Processo Rust local recebe um único projeto autorizado; usa o mesmo serviço | Mesmo servidor MCP externo; nenhum benefício de protocolo vem do framework | Mesmo servidor MCP; não expor comandos genéricos de filesystem do contêiner a agentes |
| Teclado e operação por uma mão | Campos numéricos, foco e ações explícitas; habilitar AccessKit e dar nomes aos controles. Testar a tarefa sem arrastes obrigatórios | Semântica DOM e foco são vantagens possíveis; widgets e canvas personalizados ainda exigem teste real | Depende da interface Rust/WASM escolhida e do WebView; capacidades Tauri não provam acessibilidade |
| Leitor de tela | Integração AccessKit documentada; habilitá-la não equivale a uma avaliação completa com tecnologias assistivas | Semântica do WebView precisa ser exercitada em cada sistema e controle | Mesmo limite do WebView e da interface embutida; uma tela em canvas não herda acessibilidade DOM |
| Planta, corte e 3D | `egui` desenha UI 2D e permite integração gráfica. A barra mínima não requer um motor de cena. Cenas 3D grandes serão qualificadas separadamente | 2D e 3D demandam canvas ou componente gráfico e integração de eventos. O renderizador nativo WGPU de Dioxus é documentado como experimental | Rust/WASM pode usar desenho e GPU no navegador; exige coordenadas, seleção e conversões explícitas pela fronteira |
| Coordenadas grandes e seleção | Modelo mantém números em precisão dupla; converter para coordenadas locais de tela antes de desenhar. O desenho não é autoridade geométrica | A mesma regra deve valer no caminho Rust/DOM/canvas | A mesma regra vale no WASM e no host; não reduzir precisão do projeto para atender a tela |
| Impressão e documentos | Não há emissão técnica automática fornecida pelo framework. Futuro documento deve nascer de snapshot e motor de desenho próprios | Impressão WebView pode servir a interface, mas não demonstra escala e rastreabilidade de uma prancha | Impressão do contêiner não substitui qualificação do documento técnico |
| Empacotamento | Binário Rust, recursos embutidos e bibliotecas gráficas do sistema; sem WebView adicional | Distribuir aplicativo e tratar dependência de WebView e bibliotecas da plataforma | Acrescenta host, ativos WASM, WebView e permissões por comando à distribuição |
| Reutilização web | `eframe::WebRunner` documenta destino web; domínio e representação ficam separados da persistência nativa | Dioxus documenta autoria para DOM/WASM, com reutilização de componentes | É a finalidade desta alternativa; a experiência offline e acesso a arquivos continuam a exigir qualificação |
| Custo desta primeira entrega | Menos fronteiras para a fatia BIM local; uma biblioteca de UI, um serviço e um transporte MCP | Introduz WebView sem benefício necessário para os quatro comandos iniciais | Acrescenta ponte e empacotamento antes de haver necessidade demonstrada do contêiner |

## Decisão e fronteiras

Adotar `eframe 0.37.0`, com `egui` da mesma família, e renderização `glow` na distribuição inicial. A opção reduz a quantidade de fronteiras para criar e verificar o primeiro projeto local. `wgpu` permanece alternativa de renderização a comparar quando as cenas BIM exigirem suas funções; não é tratado como parser BIM, solver ou framework de documentos. Nenhum fork ou trecho extraído desses candidatos é necessário nesta etapa.

O código se organiza pelas responsabilidades existentes:

- `prumo-domain` define projeto, identidades, unidades, objetos físicos/analíticos e contratos mínimos de instalações. Não acessa janela, transporte ou filesystem.
- `prumo-app` concentra comandos, validações, autorização, revisão e persistência. As operações devolvem o projeto realmente salvo.
- A interface Rust apresenta os dados e solicita os mesmos comandos. Não altera diretamente arquivos nem decide uma regra de engenharia.
- `prumo-mcp` adapta ferramentas tipadas para o serviço comum. O host seleciona o arquivo e concede escrita no lançamento. Ferramentas não aceitam caminho arbitrário, shell ou SQL.

A autorização local limita o processo e seus comandos ao projeto selecionado. Ela não pretende isolar um invasor que já controla a mesma conta do sistema operacional. Diretório e arquivo continuam sujeitos às permissões locais do usuário.

O futuro processo de cálculo será separado da interface. Receberá snapshot imutável, unidades, versões e configuração; devolverá resultado associado ao snapshot, diagnósticos e estado. Cancelamento ou falha não poderá escrever um resultado como concluído. A PRU-001 não contém um solver fictício nem um processo que produza números de exemplo como cálculo.

## Plataforma e reuso web

As plataformas nativas iniciais são `x86_64-pc-windows-msvc` e `x86_64-unknown-linux-gnu`, com toolchain `1.99.1` fixada. O workspace e seu lockfile identificam os pacotes. O pipeline registra o compilador e testa as features efetivamente distribuídas. As bibliotecas gráficas e de janela do sistema são dependências externas e devem constar das instruções de instalação e da evidência de build.

Separar o domínio, os dados de visualização e a persistência permite reutilização em `wasm32-unknown-unknown`. Isso não declara a existência de um produto web completo nem de persistência nativa no navegador. O target web só pode ser anunciado como distribuído após seu próprio build, interface e armazenamento terem sido testados. Tauri não integra esta versão e nenhuma permissão dele é presumida configurada.

## Limites e critérios para rever a decisão

A primeira entrega verifica o fluxo de projeto mínimo, entrada inválida, preservação de identidade/revisão e operação equivalente por MCP. Ela não qualifica ainda impressão técnica, modelos grandes, navegação 3D completa, recuperação da GPU, uso com leitores de tela de todas as plataformas ou a campanha de acessibilidade prevista no produto.

Reavaliar o renderizador quando uma cena representativa revelar limite mensurado de desempenho, precisão visual, seleção, memória ou recuperação. Reavaliar o contêiner somente se distribuição, integração de sistema ou interface web demonstrarem benefício que compense uma nova fronteira. Uma mudança de UI preserva os comandos e o formato de domínio; não cria um núcleo paralelo.

As evidências reproduzíveis da implementação pertencem aos testes do workspace, ao CI e ao registro `docs/evidencias/PRU-001.md`. Relatos de capacidade do fornecedor não são marcados como testes do Prumo. A triagem de versões e licenças está em [PRU-001](../componentes/PRU-001.md).

## Fontes primárias consultadas em 14/11/2026

- [eframe 0.37.0: uso nativo, WebRunner e features](https://docs.rs/eframe/0.37.0/eframe/).
- [egui: integração e acessibilidade](https://github.com/emilk/egui), incluindo a distinção entre UI 2D e renderização externa.
- [Dioxus: destinos e renderizadores experimentais](https://github.com/DioxusLabs/dioxus) e [documentação da família 0.7](https://dioxuslabs.com/learn/0.7/getting_started/). Candidato documental, sem versão incorporada.
- [Tauri: capacidades e escopos](https://tauri.app/security/capabilities/). Candidato documental, sem versão incorporada.
- [wgpu](https://github.com/gfx-rs/wgpu). Candidato de gráficos, sem qualificação como motor de engenharia.
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) e [rmcp 3.10.0](https://docs.rs/rmcp/3.10.0/rmcp/).
- [IfcOpenShell: introdução, componentes e licenças](https://docs.ifcopenshell.org/introduction) e [IfcMCP](https://docs.ifcopenshell.org/ifcmcp.html). Sem incorporação na fatia inicial de autoria nativa.
