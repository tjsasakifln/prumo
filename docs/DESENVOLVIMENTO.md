# Executar o Prumo

Esta é uma versão de desenvolvimento para modelar, persistir e consultar projetos. Não calcula nem dimensiona estruturas ou instalações.

## Instalação de desenvolvimento

Instale Rust por [rustup](https://rustup.rs/). No Windows, instale também as ferramentas C++ do Visual Studio para o target MSVC. Em Linux, use um ambiente gráfico X11 ou Wayland com OpenGL; as bibliotecas de desenvolvimento usadas no CI estão em `.github/workflows/rust.yml`.

Na raiz do repositório:

```sh
cargo build --workspace --locked
cargo run -p prumo-desktop --locked
```

O `rust-toolchain.toml` instala a versão exata 1.99.1. Não há configuração manual obrigatória. A tela permite escolher o arquivo, criar projeto e unidade, adicionar uma peça, renomear, cadastrar ambiente e reabrir. Cada comando grava sua revisão. Coordenadas de entrada respeitam a unidade escolhida e o modelo conserva metros internamente.

## Referências normativas e capacidades

Abra **Normas e capacidades** no projeto. A tela mostra a cobertura do inventário, as pendências técnicas e o pacote de referências disponível. **Fixar pacote de referências** preserva o conteúdo e sua identidade SHA-256 no arquivo do projeto.

Em **Cadastrar ou atualizar referência normativa**, selecione uma referência ou crie uma nova. O cadastro gera outra versão e apresenta o impacto previsto. O pacote ativo continua preservado até marcar a confirmação e escolher **Migrar pacote do projeto**. A comparação mostra partes, edições, emendas, erratas e capacidades afetadas antes da decisão. O histórico conserva todos os pacotes anteriores.

Selecionar uma capacidade apresenta os motivos que impedem um resultado final. O catálogo distribuído contém metadados públicos, sem textos normativos licenciados ou aprovações profissionais. Nenhuma de suas capacidades de engenharia está liberada. Informar uma edição, um responsável funcional ou uma fonte pública não supre a consulta autorizada, a interpretação e a qualificação exigidas.

## Servidor MCP local

```sh
cargo run -p prumo-mcp-server --locked -- --project exemplo.prumo
```

O nome exato do pacote e os argumentos do servidor também aparecem em `cargo run -p prumo-mcp-server -- --help`. O lançamento seleciona um único arquivo. Escrita exige `--allow-write`; o padrão permite somente leitura. O cliente não escolhe caminhos nem executa comandos do sistema. O protocolo usa stdio e a revisão MCP 2026-07-28. A documentação específica em `docs/evidencias/PRU-001-mcp.md` descreve descoberta, ferramentas e testes.

As ferramentas `prumo.registry.read`, `prumo.registry.audit`, `prumo.norms.package` e `prumo.results.gate` consultam o registro. `prumo.norms.register`, `prumo.norms.pin` e `prumo.norms.migrate` usam o mesmo serviço da interface e exigem escrita autorizada e revisão atual. Fixação e migração também conferem os hashes do pacote.

Uma segunda instância deve reabrir o projeto para ver a última revisão. Uma alteração com revisão antiga é recusada e pede atualização, preservando a alteração já salva pela outra instância.

## Verificar

```sh
python3 scripts/validate_plan.py
cargo fmt --all -- --check
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check -p prumo-ui --target wasm32-unknown-unknown --locked
```

Os testes de interface usam os widgets e a árvore de acessibilidade reais. Os testes de MCP iniciam um processo separado e utilizam o cliente do SDK oficial. O reaproveitamento WASM compila a camada de interface e domínio; um aplicativo web com acesso ao armazenamento não é distribuído nesta etapa.

## Matriz distribuída

| Destino | Compilador | Interface | Persistência |
|---|---|---|---|
| x86_64-pc-windows-msvc | Rust 1.99.1, edition 2024 | eframe, OpenGL e AccessKit | Arquivo local |
| x86_64-unknown-linux-gnu | Rust 1.99.1, edition 2024 | eframe, X11/Wayland, OpenGL e AccessKit | Arquivo local |
| wasm32-unknown-unknown | Rust 1.99.1 | Componente UI reutilizável, experimental | Sem aplicativo web distribuído |

Features, dependências exatas e resolução transitiva são registradas em Cargo.toml e Cargo.lock. A primeira etapa distribui executáveis de desenvolvimento; instaladores e atualização pertencem às entregas posteriores.
