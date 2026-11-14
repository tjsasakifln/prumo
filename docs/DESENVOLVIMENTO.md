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

## Servidor MCP local

```sh
cargo run -p prumo-mcp-server --locked -- --project exemplo.prumo
```

O nome exato do pacote e os argumentos do servidor também aparecem em `cargo run -p prumo-mcp-server -- --help`. O lançamento seleciona um único arquivo. Escrita exige `--allow-write`; o padrão permite somente leitura. O cliente não escolhe caminhos nem executa comandos do sistema. O protocolo usa stdio e a revisão MCP 2026-07-28. A documentação específica em `docs/evidencias/PRU-001-mcp.md` descreve descoberta, ferramentas e testes.

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
