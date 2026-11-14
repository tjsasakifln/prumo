# PRU-001: domínio, comandos e persistência inicial

## Comportamento entregue

`prumo-domain` define o projeto, a unidade de apresentação, identidades físicas e analíticas distintas, disciplina, ambientes, redes, portas tipadas, conexões explícitas, autoria e revisão. O estado disponível dos objetos é `modelled`. Não existe resultado estrutural ou aprovação normativa nesta entrega.

`prumo-app::ProjectService` seleciona um arquivo `.prumo` no host e executa os comandos compartilhados `create`, `add_member`, `rename_project` e `add_environment`. O comando não recebe caminho. Os parâmetros e campos desconhecidos são rejeitados, inclusive tentativas de acrescentar `path` ou de chamar `shell`.

As coordenadas de entrada usam a unidade declarada do projeto. A persistência registra metros, sem arredondamento de apresentação. Cada alteração aceita acrescenta uma revisão e registra a identidade de autoria fornecida pelo host. A criação não sobrescreve um projeto existente. Alterações com revisão obsoleta são recusadas.

Uma sessão `ReadOnly` pode consultar o projeto e não pode executar nenhum comando de escrita, inclusive a criação inicial. Consultas abrem um snapshot completo e não precisam criar um arquivo de bloqueio. Isso permite abrir um projeto em pasta sem permissão de escrita.

## Integridade do arquivo inicial

O formato inicial é JSON com `schema_version = 1`. O arquivo é limitado a 16 MiB antes da leitura completa. A revisão, o histórico, as identidades, as referências e a geometria são validados ao ler e antes de salvar. Nomes vazios, caracteres de controle, coordenadas não finitas, peças de comprimento nulo, identidades repetidas e portas incompatíveis são recusados.

Toda alteração adquire um bloqueio exclusivo em arquivo fixo ao lado do projeto, relê a revisão e só então escreve. A nova revisão é gravada em arquivo temporário na mesma pasta, sincronizada e publicada por substituição atômica. Em Unix, a pasta também é sincronizada. Leituras concorrentes observam o arquivo completo anterior ou o novo. Não existe janela de truncamento do arquivo final.

Esse contrato inicial não substitui o pacote versionado, o banco local, as migrações e a recuperação ampliada previstos na PRU-005. O modelo preserva dados de extensão em mapas explícitos. Campos desconhecidos fora desses mapas e versões futuras são recusados, preservando o arquivo original.

## Ensaios executados

Em Rust 1.99.1, Linux x86_64, foram aprovados 16 testes específicos destas duas crates: 13 de serviço e 3 de domínio. O comando de reprodução é:

```sh
cargo test -p prumo-domain -p prumo-app --locked
```

Os ensaios verificam:

- Criação, gravação, encerramento do serviço, reabertura por outra sessão, alteração e nova reabertura, preservando UUIDs, unidade, geometria e autoria.
- Equivalência física de coordenadas expressas em metros, centímetros e milímetros, com valores esperados independentes do conversor.
- Rejeição de nomes e coordenadas inválidos sem qualquer mudança nos bytes da última revisão.
- Autorização desde a criação e em todas as variantes do comando comum.
- Duas sessões abertas independentemente disputando a mesma revisão: exatamente uma alteração é aceita e a outra recebe conflito.
- Leitura em arquivo 0444 e pasta 0555, sem criação de bloqueio, no ensaio específico de Unix.
- Recusa de arquivo corrompido, versão futura, identidade nula, arquivos excessivos e links simbólicos selecionados no host.
- Persistência de ambientes e do contrato de redes, portas, conexões e extensões. O caso de rede é explicitamente uma fixture de modelagem, sem motor de cálculo.
- Ausência de conexão automática por coincidência geométrica, rejeição de referências órfãs e incompatibilidade de portas.

A equivalência completa com interface e cliente MCP é verificada na integração da PRU-001. Estes 16 testes não são apresentados como execução de uma interface ou de um cliente MCP.

## Triagem de reuso desta parcela

Foram inspecionados os manifests e os arquivos `LICENSE-MIT` e `LICENSE-APACHE` das distribuições oficiais obtidas pelo Cargo. As versões são fixadas no workspace e a resolução transitiva consta do `Cargo.lock`.

| Componente | Versão | Função adotada | Licença declarada e decisão |
|---|---|---|---|
| [Serde](https://github.com/serde-rs/serde) | 1.0.229 | Contratos de serialização Rust | MIT ou Apache-2.0. Reutilizado sem copiar ou modificar seu código. Evita manter um protocolo próprio de serialização. |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | JSON versionado e dados explícitos de extensão | MIT ou Apache-2.0. Reutilizado com limites de arquivo, rejeição de campos inesperados e validação semântica própria. |
| [uuid](https://github.com/uuid-rs/uuid) | 1.29.2 | UUID v4 e serialização de identidade | Apache-2.0 ou MIT. Reutilizado para identidades; a separação físico/analítico e a recusa de duplicação pertencem ao domínio Prumo. |
| [thiserror](https://github.com/dtolnay/thiserror) | 2.0.21 | Erros tipados compartilhados | MIT ou Apache-2.0. Reutilizado para derivação de erros, sem incluir lógica própria em outra linguagem. |
| [tempfile](https://github.com/Stebalien/tempfile) | 3.27.0 | Arquivo temporário na mesma pasta e publicação atômica | MIT ou Apache-2.0. Reutilizado em vez de implementar nomes temporários e substituição por plataforma. |

As obrigações de avisos das dependências permanecem aplicáveis à distribuição. A triagem desta parcela não amplia a aprovação para cálculo, BIM externo ou interpretação normativa. O inventário de distribuição da PRU-001 cobre as dependências transitivas e os avisos efetivamente empacotados. A qualificação ampla de componentes pertence à PRU-003.

## Revisão aplicada

A revisão independente identificou que uma consulta inicialmente exigia criar um bloqueio em pasta gravável. A consulta passou a ler o arquivo completo publicado atomicamente, e foi acrescentado o teste específico de pasta somente leitura. As mutações continuam a exigir bloqueio e comparação de revisão. O domínio e o serviço proíbem `unsafe`.
