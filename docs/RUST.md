# Implementação do Prumo em Rust

**Decisão:** TEC-RUST-01. **Requisito:** REQ-15. **Origem:** instrução do proprietário USR-05, de 14/11/2026. **Estado:** obrigatória para as 250 issues. O produto continua em planejamento; esta decisão não declara código ou cálculos implementados.

## 1. Alcance da decisão

Todo código próprio do produto será escrito em **Rust**: interface, domínio BIM e redes, comandos, regras normativas, cálculos desenvolvidos pelo Prumo, persistência, CLI, servidor MCP, serviços, adaptadores e orquestração. O destino pode ser binário nativo ou WebAssembly, conforme a capacidade e o ambiente qualificados.

As decisões de arquitetura das issues 1 e 3 escolhem frameworks, bibliotecas e fronteiras dentro dessa exigência. Alterar a linguagem do produto exige nova decisão do proprietário com impacto apresentado. A política estruturada está em [technology.json](../planning/technology.json); o requisito permanente está em [requirements.json](../planning/requirements.json).

O reuso de componentes abertos pertinentes e validados permanece obrigatório. Componentes externos podem conservar sua linguagem original, com procedência, versão, licença, função e integração verificáveis. A camada desenvolvida pelo Prumo para coordená-los será Rust. Essa permissão não autoriza novos módulos próprios de interface, engenharia ou negócio em outra linguagem.

## 2. Organização e compilação

A issue 1 deverá criar um **workspace Cargo funcional**, com pacotes separados pelas responsabilidades efetivamente necessárias. Aplicação, domínio, comandos e integrações compartilham contratos; a divisão em crates deve ajudar os testes e a manutenção. A documentação oficial descreve o compartilhamento de lockfile, dependências e políticas no workspace. [RS-006](https://doc.rust-lang.org/cargo/reference/workspaces.html).

Exemplos de responsabilidades futuras: `apps/prumo-desktop`, `apps/prumo-cli`, `apps/prumo-mcp`, `crates/prumo-domain`, `crates/prumo-commands`, `crates/prumo-contracts`, `crates/prumo-persistence`, `crates/prumo-numerics`, `crates/prumo-norms`, `crates/prumo-ui`, `crates/prumo-render` e `crates/prumo-adapters`. Criar cada pacote quando houver comportamento real. Uma árvore vazia não satisfaz a primeira entrega.

A base deverá versionar:

- `Cargo.toml` do workspace, com `resolver`, edition, versão suportada e políticas explícitas.
- `Cargo.lock`, preservado nos testes e na compilação de distribuição.
- `rust-toolchain.toml`, com versão estável exata, componentes e targets necessários.
- Matriz de sistemas, arquiteturas, features, bibliotecas nativas, SDKs, flags e configuração de compilação distribuídos.

A versão será escolhida e conferida durante a implementação. `rust-version` declara compatibilidade; a seleção do compilador efetivo também precisa ser fixada e verificada. [RS-007](https://rust-lang.github.io/rustup/overrides.html) e [RS-008](https://doc.rust-lang.org/cargo/reference/rust-version.html).

A integração contínua deverá executar formatação, análise estática, testes e compilação, em Windows e Linux, para a matriz adotada. Comandos de base, a concretizar na issue 1:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

O pipeline deve acrescentar as combinações de features e targets efetivamente suportadas. Features mutuamente exclusivas são testadas em trabalhos separados; `--all-features` não deve ativar combinações inválidas. `--locked` preserva a resolução Cargo, mas não fixa sozinho bibliotecas nativas, SDKs, flags, ambiente de linkedição nem resultados numéricos. A distribuição registra também esses componentes. [RS-009](https://doc.rust-lang.org/cargo/commands/cargo-build.html).

## 3. Interface e experiência

A **interface própria também terá autoria Rust**, com comportamento e estado de interação no código Rust. Recursos visuais declarativos e código gerado pelas ferramentas não transferem para outra linguagem as regras de interface, autorização ou engenharia.

`egui`/`eframe`, Dioxus e `wgpu` são candidatos pesquisados, sujeitos a decisão e ensaios. `egui`/`eframe` documentam destinos nativos e web; Dioxus oferece autoria Rust e distingue seus renderizadores experimentais; `wgpu` fornece gráficos e não resolve sozinho edição BIM, documentos ou acessibilidade. [RS-003](https://github.com/emilk/egui), [RS-012](https://docs.rs/eframe/latest/eframe/), [RS-004](https://github.com/DioxusLabs/dioxus) e [RS-005](https://github.com/gfx-rs/wgpu).

A comparação usará o mesmo caso BIM: plantas, cortes e 3D, grandes coordenadas, seleção precisa, foco e teclado, leitor de tela, escala de interface, múltiplas vistas, impressão, operação local, memória e recuperação de falha gráfica. Um contêiner como Tauri só será pertinente se hospedar autoria Rust, por exemplo Rust/WASM, com integração qualificada. A seleção do framework não está concluída nesta revisão.

## 4. MCP desde a primeira entrega

O servidor MCP próprio será Rust e usará o mesmo serviço de comandos da interface. O **SDK Rust oficial `rmcp`** é o candidato prioritário de reuso. Seu repositório declara a revisão MCP 2026-07-28, e a documentação da crate descreve cliente, servidor e transportes. A implementação precisa fixar versão/commit, features e perfis, com ensaios reais; a declaração da branch principal não comprova os recursos de toda versão publicada. [RS-001](https://github.com/modelcontextprotocol/rust-sdk) e [RS-002](https://docs.rs/rmcp/latest/rmcp/).

A issue 1 demonstra criação, persistência, reabertura e consulta de um projeto BIM real por interface Rust e cliente MCP. O transporte inicial permanece stdio. Revisão esperada, idempotência, autorização, unidades, auditoria e invalidação de resultados continuam no contrato de [MCP.md](MCP.md).

SDKs de outras linguagens podem servir a clientes externos de interoperabilidade ou referências de teste. IfcMCP e outros componentes externos não substituem o núcleo Rust nem recebem acesso irrestrito ao modelo. A licença atual do SDK Rust registra transição entre Apache-2.0 e trechos MIT; conferir o recorte adotado e seus avisos. [RS-013](https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE).

## 5. Integração de bibliotecas e motores externos

Cada componente deve ter um papel identificado: produção, ferramenta de apoio, referência independente ou cliente de teste. A adoção exige os critérios de [REUSO.md](REUSO.md), inclusive licença, manutenção, testes e domínio de aplicação. Um motor de produção não pode gerar sua própria referência de validação independente.

| Fronteira | Obrigações do adaptador Rust |
|---|---|
| FFI | ABI, tipos, alinhamento, comprimentos, ownership, liberação, concorrência, callbacks e tradução de erros; impedir propagação de panic ou exceção por fronteira incompatível |
| Processo isolado | Protocolo versionado, unidades, revisão/snapshot, binário e argumentos controlados, limites de recursos, cancelamento, reinício e resultado inválido identificado |
| WebAssembly | Imports e capacidades autorizados, limites de memória/tempo, conversões, contrato versionado e comportamento de falha |

Bindings mecânicos gerados e patches em componentes externos ficam identificados por origem, escopo e revisão. Devem ser restritos à integração ou à manutenção do componente, sem abrigar um módulo próprio do Prumo em outra linguagem. Formatos de dados, configuração, estilos e migrações declarativas, como JSON, TOML, CSS e SQL, são recursos do produto; não substituem regras de negócio ou de engenharia Rust.

O pacote instala os binários e runtimes externos necessários. O usuário final não precisa instalar compilador, Python ou ferramentas de desenvolvimento. Atualizações de componentes exigem análise de impacto e nova execução das evidências afetadas.

## 6. Segurança de memória e erros

As camadas puras de domínio, comandos e regras próprias deverão proibir `unsafe`. Fronteiras inevitáveis ficam em crates de integração pequenos, com invariantes documentadas, interface segura e revisão independente. O inventário também considera dependências transitivas. A ausência de `unsafe` no domínio não equivale à ausência de código inseguro em toda a distribuição.

O contrato de FFI e as condições que evitam comportamento indefinido precisam ser demonstrados. Rust mantém essas obrigações mesmo em blocos `unsafe`. [RS-010](https://doc.rust-lang.org/nomicon/ffi.html) e [RS-011](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).

Testar entradas inválidas, falhas, vazamento ou uso incorreto de recursos, cancelamento e concorrência nas fronteiras aplicáveis. Instrumentação e ferramentas de análise ajudam onde forem compatíveis; não substituem a revisão do contrato nem validam automaticamente uma chamada externa.

## 7. Extensões e ferramentas de planejamento

O host de extensões, a API e o executor próprio de rotinas visuais serão Rust. A capacidade prevista nas issues 108 e 236 de criar rotinas e scripts restritos permanece: extensões de terceiros são conteúdo isolado, com origem, permissões e limites definidos, e só usam comandos autorizados. Não podem substituir módulos próprios obrigatórios por código de outra linguagem.

Os scripts Python que geram e conferem o planejamento são ferramentas administrativas e permanecem fora do runtime do produto. O CI atual verifica os documentos e registros. Os comandos Cargo acima serão executáveis quando a issue 1 entregar o workspace real. Arquivos Rust vazios ou verificações Cargo sem aplicação funcional não serão apresentados como implementação.

## 8. Aceite e lançamento

Cada issue deve comprovar a aderência ao REQ-15 no código que entregar, além de seus critérios funcionais. A issue 1 estabelece a base; a 3 qualifica os componentes; as etapas de integração mantêm a fronteira; as issues 246 a 250 reconciliam origem, linguagem, dependências e artefatos do candidato final.

Rust não comprova formulação física, correção numérica ou atendimento a normas. Permanecem integrais as referências independentes, tolerâncias pré-registradas, testes de conservação e convergência, estudos de usabilidade e revisão profissional descritos em [VALIDACAO.md](VALIDACAO.md) e [LANCAMENTO.md](LANCAMENTO.md).

Fontes primárias, data de consulta e limites: [fontes-rust.json](fontes-rust.json). Todas as bibliotecas citadas continuam candidatas até sua qualificação.
