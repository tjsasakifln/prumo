# PRU-001: servidor MCP local

## Comportamento entregue

`prumo-mcp` é um executável Rust com transporte stdio. O processo recebe um único arquivo `.prumo` por `--project`. O modo padrão permite leitura. A opção `--allow-write`, fornecida pelo usuário ao iniciar o processo, concede criação e edição desse destino. A identidade de autoria é definida no host como `mcp-local`; parâmetros enviados por clientes não podem mudar autor, projeto ou permissões.

```sh
cargo run --locked -p prumo-mcp-server -- --project meu-projeto.prumo
cargo run --locked -p prumo-mcp-server -- --project meu-projeto.prumo --allow-write
```

O servidor anuncia somente a revisão MCP **2026-07-28**. Usa `server/discover` e os metadados exigidos em cada requisição. Não anuncia suporte ao fluxo legado de inicialização. Stdout contém somente mensagens JSON-RPC; mensagens humanas e falhas de lançamento seguem por stderr.

| Ferramenta | Operação efetiva | Autorização |
|---|---|---|
| `prumo.project.read` | Reabre o estado persistido com identidades, revisão e unidades | Leitura |
| `prumo.project.create` | Cria um projeto novo no arquivo selecionado | Escrita |
| `prumo.model.add_member` | Adiciona uma peça física e sua identidade analítica | Escrita e revisão atual |
| `prumo.project.rename` | Altera o nome do projeto | Escrita e revisão atual |
| `prumo.model.add_environment` | Adiciona um ambiente | Escrita e revisão atual |

Todas as operações chamam `prumo-app::ProjectService`, também usado pela interface. O MCP não contém outra implementação do domínio. Cada resposta de modelagem indica `calculation_available: false`; as peças permanecem no estado `modelled`.

As ferramentas de escrita não aparecem no catálogo de uma sessão somente leitura. Mesmo uma chamada direta, usando o nome de uma dessas ferramentas, é recusada pelo serviço compartilhado. Os schemas e os desserializadores rejeitam campos adicionais. Não existem ferramentas de shell, SQL, execução de código ou seleção de caminhos. Coordenadas seguem a unidade do projeto e são convertidas pelo domínio para metros.

O transporte limita cada mensagem de entrada a 64 KiB. Uma mensagem excessiva é descartada sem mutação e o próximo pedido válido continua utilizável. Até quatro comandos são despachados simultaneamente; operações bloqueantes de arquivo são executadas fora do executor assíncrono. A validação de revisões e o bloqueio do arquivo permanecem no serviço de comandos.

## Verificação reproduzível

Executar:

```sh
cargo test --locked -p prumo-mcp-server
cargo clippy --locked -p prumo-mcp -p prumo-mcp-server --all-targets -- -D warnings
```

Os testes de `apps/prumo-mcp/tests/stdio.rs` iniciam o executável real em processos filhos. Dois clientes exercitam o contrato: o SDK `rmcp` fixado e um cliente JSON-RPC independente, escrito em Rust, que não usa os tipos nem o transporte do SDK para montar e ler mensagens.

1. Cliente SDK negocia a revisão, cria projeto, adiciona peça com entrada em centímetros, encerra o processo e reabre em leitura. IDs físicos e analíticos, revisão e coordenadas em metros são comparados.
2. A interface `Desktop` real é operada por cliques no `egui_kittest`: criar projeto e adicionar peça. Após destruir a instância da interface, o cliente MCP consulta e altera o arquivo. Uma nova instância de `Desktop` reabre o resultado e preserva IDs e revisão. Um clique com peça de comprimento nulo é recusado sem alteração. O ensaio usa a árvore de acessibilidade e eventos da interface; não é uma simulação de respostas do serviço.
3. Uma sessão somente leitura tenta criar, renomear, adicionar peça e adicionar ambiente. Todas as mutações são recusadas, inclusive quando solicitadas diretamente fora do catálogo anunciado.
4. O cliente independente verifica descoberta, esquema, caminho extra malicioso, dimensão errada das coordenadas, revisão inválida, peça nula, revisão vencida e ferramenta inexistente. O projeto permanece igual após os erros.
5. Versão desconhecida retorna erro `-32022` e a lista suportada. Ausência dos metadados obrigatórios retorna `-32602`. Uma requisição válida posterior continua funcionando.
6. Uma mensagem maior que 64 KiB não cria arquivo; a descoberta de ferramentas seguinte funciona.
7. Parâmetros ausentes ou inválidos de lançamento falham com mensagem em stderr. A ajuda não contamina stdout.

Os sete testes foram executados com sucesso em Linux durante esta implementação. A execução nativa da janela e a matriz Windows/Linux são registradas na evidência consolidada da issue. Os ensaios do protocolo e da interface não qualificam cálculo estrutural nem substituem revisão profissional.

## Reuso e fronteira

`rmcp 3.10.0` foi adotado para o protocolo, descoberta e transporte. A versão efetivamente baixada contém o perfil 2026-07-28, validação de metadados por requisição e transporte limitado por `with_max_line_length`. A alternativa de escrever outro servidor de protocolo foi rejeitada por duplicar manutenção e validação. O núcleo de projeto e os contratos próprios continuam Rust, separados do SDK.

Features de produção: `server` e `transport-io`, sem features padrão. Os testes acrescentam `client` e `transport-child-process`. O uso não ativa transportes HTTP, OAuth, shell ou servidor de arquivos. Tokio fornece o executor, stdio e despacho bloqueante. Os pacotes e transitivas são fixados em `Cargo.lock` e constam da triagem consolidada de [componentes da PRU-001](../componentes/PRU-001.md).

O manifesto de `rmcp 3.10.0` declara Apache-2.0. Seu `LICENSE` também preserva contribuições MIT anteriores sem consentimento de relicenciamento. Os avisos completos acompanham a distribuição; não se presume que a transição elimine as obrigações MIT. O SDK é uma dependência, sem código de engenharia nem patches próprios em linguagem estrangeira.

Fontes de implementação verificadas: [SDK Rust oficial](https://github.com/modelcontextprotocol/rust-sdk), [release publicada 3.10.0](https://crates.io/crates/rmcp/3.10.0), [documentação da versão](https://docs.rs/rmcp/3.10.0/rmcp/) e código-fonte correspondente obtido pelo Cargo. A licença e o comportamento foram examinados na versão fixada, não inferidos da branch principal.
