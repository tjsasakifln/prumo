# PRU-005: banco transacional e pacote nativo

**Data:** 14/11/2026. **Parcela:** `crates/prumo-storage`, seus testes e a fixture
de versão futura. A integração do serviço, autosave, cópias originais, interface,
MCP e quedas de processo possui evidências próprias. Esta ficha não declara
conclusão da issue nem qualificação de cálculo de engenharia.

## 1. Comportamento entregue

O banco SQLite é a autoridade do estado persistido. Cada campo superior do
`Project` é serializado em um objeto identificado por SHA-256. Um envelope
imutável relaciona os nomes dos campos aos hashes e registra identidade,
revisão e versão do modelo. Campos estáveis, como o catálogo normativo, são
reutilizados nas revisões seguintes. O identificador de uma inspeção e seu
snapshot não são reatribuídos na leitura.

O banco mantém quatro tabelas de contrato fixo:

| Tabela | Função e regra |
|---|---|
| `content_objects` | Bytes imutáveis e seus hashes/tamanhos. A API insere ou reutiliza o conteúdo exato; não atualiza nem remove objetos |
| `revisions` | Ordinal do registro, revisão de domínio e referências ao envelope e ao comando |
| `head` | Identidade, revisão e envelope vigentes, publicados na mesma transação dos objetos e do registro |
| `extensions` | Caminho lógico, tipo, versão e hash de dados opacos de domínios futuros, preservados junto do banco |

O registro de comando contém identidade própria, ator fornecido pelo serviço,
operação, motivo opcional, data e payload recebido. O armazenamento deriva a
revisão base e os hashes anterior/seguinte. Esse registro documenta o que foi
executado; não implementa replay ou desfazer da etapa seguinte. Na importação de
uma baseline anterior, o histórico existente é preservado e o payload de
comando ausente permanece `null`. Não são inventados comandos anteriores à
implantação do banco.

Uma operação administrativa `storage.recover` pode registrar a recuperação na
mesma revisão somente se o projeto for idêntico, inclusive seus bytes
serializados. Ela cria outro registro no diário, com identidade nova, e conserva
o histórico do domínio. Uma alteração disfarçada na mesma revisão é recusada.

### API nativa

| Operação | Resultado |
|---|---|
| `SQLiteWorkingStore::open(path)` | Cria banco novo ou abre esquema reconhecido. Arquivo existente vazio, estranho, corrompido ou futuro não é inicializado silenciosamente |
| `open_with_project(path)` | Devolve a conexão e o projeto da mesma validação inicial, sem repetir `load` imediatamente |
| `open_read_only(path)` e `load()` | Leitura validada sem criar sidecars na pasta do projeto |
| `snapshot_read_only(path)` | Faz backup SQLite consistente e valida essa imagem uma vez; projeto e bytes devolvidos pertencem à mesma imagem |
| `backup_read_only(path)` | Devolve apenas uma imagem SQLite consistente, sem afirmar que objetos, projeto ou diário estejam válidos |
| `commit(expected_revision, project, record)` | Compara revisão e identidade, preserva o prefixo de histórico e confirma toda a transação |
| `commit_with_extensions(...)` | Inicializa extensões atomicamente com a primeira baseline. Nas revisões seguintes exige o mesmo conjunto |
| `extensions()` | Devolve o conjunto opaco validado, sem interpretá-lo |
| `commit_with_hook(..., hook)` | Ponto de teste/controlador nativo com fases tipadas, sem parâmetro de ambiente, SQL, CLI ou MCP |
| `backup_bytes()` | Imagem consistente pela API de backup SQLite, nunca cópia crua de arquivo ativo |
| `restore_database(path, bytes)` | Valida em arquivo temporário privado e publica atomicamente; exige que o host feche conexões e mantenha seu lock |
| `encode_package(store, extensions, origin)` | Monta pacote a partir de uma única imagem consistente e de projeções dessa imagem |
| `encode_validated_package(store, extensions, origin)` | Devolve `EncodedPackage`, cujos bytes privados só podem ser construídos pelo encoder e acessados por empréstimo imutável ou consumo |
| `decode_package(bytes)` | Devolve projeto, manifesto, imagem do banco e extensões somente após confrontar todo o conteúdo |

O argumento de extensões do encoder, quando não vazio, precisa ser igual ao
índice do banco. Vazio significa usar o conjunto já persistido. Por isso a
recuperação de um banco isolado conserva também dados opacos que não estejam no
pacote principal disponível.

## 2. Formato e ausência de autoridade concorrente

São versões separadas: **contêiner 1**, **banco 1** e **modelo 2**. O decodificador
JSON legado v1/v2 pertence ao domínio; o serviço prepara a baseline e conserva
o original antes de migrar. A crate de armazenamento não transforma um banco
futuro em banco atual.

O arquivo `.prumo` contém:

| Entrada | Conteúdo |
|---|---|
| `manifest.json` | Versões, identidade, revisão, origem, data do último registro incluído, hashes e relação de entradas com tamanho/tipo/versão |
| `state.sqlite` | Banco consistente e validado, incluindo objetos, comandos e extensões |
| `project.json` | Projeção completa do modelo físico, analítico, materiais, ações, redes, identidades, proveniência, perfil normativo, snapshots, resultados e extensões do domínio |
| `commands.json` | Projeção legível do diário de comandos do banco |
| `objects/<sha256>.blob` | Projeção exata dos objetos por conteúdo. O sufixo binário admite tanto campos JSON quanto dados opacos |
| `extensions/...` | Projeções das extensões identificadas no próprio banco |

Cada entrada, exceto o próprio manifesto, tem SHA-256 e tamanho no manifesto.
O banco possui verificações internas dos hashes de todos os objetos. O leitor
compara ainda `project.json`, `commands.json` e cada objeto/extensão com a
autoridade reconstruída do banco. Alterar uma projeção e recalcular seu hash no
manifesto não torna a alteração aceita. Entradas ausentes, sobressalentes ou de
papel incompatível são recusadas.

O hash da projeção completa é distinto do hash de snapshot semântico da
PRU-004. A projeção usa a representação JSON do DTO, com mapas ordenados e o
roundtrip exato de `f64` e `arbitrary_precision` configurados no workspace.
Números JSON desconhecidos preservam seu valor mesmo fora do intervalo de
`f64`/`u64`. A forma lexical pode ser normalizada pela biblioteca, por exemplo
`1e9999` para `1e+9999`; isso não autoriza perda de valor ou precisão. O contrato
não se apresenta como JSON canônico RFC 8785. O hash identifica conteúdo; não
autentica autoria nem comprova revisão profissional.

## 3. Transação, cópia consistente e leitura

Conexões de escrita usam `journal_mode=DELETE` e `synchronous=EXTRA`, com consulta
da configuração efetiva. A transação `IMMEDIATE` grava objetos, diário e
cabeçalho como uma unidade. O bloqueio de SQLite tem espera limitada a 500 ms.
O hook tipado permite interromper após inserir objetos, inserir revisão,
atualizar cabeçalho e imediatamente antes de `COMMIT`. Retornar erro reverte o
conjunto. O serviço também usa esses pontos em testes com processos encerrados.

O backup usa uma chamada limitada a `Backup::step(-1)`. `Busy`, `Locked` ou uma
cópia não concluída retornam erro; o adaptador não utiliza o laço de repetição
ilimitada de `run_to_completion`. O limite de páginas restringe o tamanho da
cópia. O destino é sincronizado e fechado antes da leitura dos bytes.

A leitura pública valida cabeçalho, esquema, integridade SQLite e conteúdo.
Para a imagem privada recebida por bytes, o caminho interno faz essa validação
de conteúdo uma única vez antes de devolver dados. O arquivo temporário é
privado e congelado durante a operação. Essa redução de trabalho não se aplica
como cache a um banco vivo ou a consultas de arquivos mutáveis.

Na inspeção do host, uma imagem de `backup_read_only` só permite reutilizar o
projeto se seus bytes forem exatamente iguais aos do banco do pacote que já
passou pela validação completa naquela operação. Qualquer diferença exige
snapshot e validação completos. A API de backup devolve um `Vec<u8>` comum,
sem tipo ou sinalizador que declare aprovação do conteúdo.

Dentro de um commit, o `Project` fornecido passa pela validação integral antes
da transação. A reconstrução do cabeçalho novo precisa produzir exatamente os
mesmos bytes desse projeto. Somente depois dessa igualdade a segunda validação
semântica idêntica é dispensada. Esquema, objetos, hashes, diário e histórico
continuam conferidos. Não existe sinalizador público de confiança nem reuso de
validação entre requisições.

`EncodedPackage` conserva os bytes privados produzidos a partir da imagem
validada pelo encoder. Não implementa desserialização, construtor de bytes
recebidos ou acesso mutável. O host pode publicar essa saída sem decodificá-la
novamente, conferindo os bytes do arquivo temporário antes da publicação.
Qualquer arquivo recebido continua passando pelo decodificador completo,
inclusive quando foi criado anteriormente por esse encoder e depois alterado.

O modo somente leitura não usa `immutable=1`, não depende de WAL e não cria
`-shm`. Um rollback journal quente precisa de recuperação pelo host em caminho
de escrita. A restauração recusa `-journal`, `-wal` ou `-shm` presentes no destino;
o serviço deve preservar/quarentenar a sessão antes de substituir o banco. No
Unix a publicação também sincroniza o diretório. O teste de encerramento de
processo não equivale a ensaio de perda elétrica, de hardware ou de todas as
implementações de sistema de arquivos.

## 4. Entradas externas e limites

O adaptador ativa modo defensivo, desliga confiança no esquema, views e
gatilhos, restringe colunas, expressões, instruções, parâmetros, funções,
conexões anexadas e quantidade de objetos do esquema. Um autorizador admite
somente as operações SQL estáticas necessárias: inserção nas tabelas próprias,
atualização de `head`, consultas e transações. Não há API de SQL, URI de conexão
ou caminho de arquivo proveniente de MCP. Uma contagem de instruções SQLite
interrompe trabalho que exceda o orçamento estabelecido.

O esquema recebido deve corresponder às definições e índices da versão. Views,
triggers, tabelas extras, tabelas virtuais e definições diferentes são recusados.
Também são conferidos `quick_check`, referências estrangeiras, hashes,
continuidade do diário, unicidade dos comandos, identidade do projeto,
proveniência histórica e ausência de objetos órfãos. O projeto atual passa por
todas as validações do domínio; no commit, vale a comparação exata dentro da
mesma operação descrita acima.

| Limite do formato 1 | Valor |
|---|---:|
| Pacote recebido ou produzido | 128 MiB |
| Banco SQLite | 64 MiB |
| Entrada individual | 64 MiB |
| Total expandido de ZIP | 256 MiB |
| Total expandido das extensões | 64 MiB |
| Manifesto | 2 MiB |
| Registro de comando | 4 MiB |
| Entradas ZIP | 4.096 |
| Registros no diário | 2.048 |
| Bytes dos campos históricos conferidos por leitura | 512 MiB |
| Razão expandido/compactado | 200 |

Os limites se combinam. Antes de confirmar uma transação, o adaptador calcula
o tamanho projetado do pacote Stored completo, incluindo cópias auditáveis,
extensões, manifesto, origem no maior tamanho permitido e margem de cabeçalhos.
Assim, uma extensão de 44 MiB, embora caiba isoladamente no banco, é recusada
com rollback porque suas representações somadas ultrapassariam 128 MiB. O mesmo
critério se aplica a bancos recebidos, para não anunciar como recuperável um
estado que o formato não consiga exportar.

O leitor ZIP verifica EOCD e diretório antes de criar `ZipArchive`, sem reservar
capacidade a partir do tamanho declarado de uma entrada. Confere contagem,
comprimento de metadados, tamanhos, total e razão. ZIP64, múltiplos discos,
comentários e caudas fora do perfil são recusados. O menor offset de cabeçalho
local deve ser zero; isso recusa prefixos mesmo quando os offsets foram
recalculados e `ZipArchive::offset()` retorna zero.

Nomes usam um perfil ASCII portátil. Não se aceitam caminhos absolutos,
componentes vazios, `.` ou `..`, barras invertidas, unidade de Windows, ADS,
NUL, nomes reservados, ponto final ou colisões por caixa ou por hierarquia de
arquivo/diretório. Entradas repetidas, sobrepostas, cifradas, diretórios,
symlinks e outros tipos de arquivo são recusados. Os cabeçalhos local e central
devem concordar. Nenhum caminho recebido é extraído para o sistema de arquivos.

Stored é usado em todas as entradas produzidas. A leitura também aceita Deflate
com as mesmas guardas. Isso evita que zeros ou outro conteúdo nativo muito
repetitivo produzam um pacote que exceda a própria razão máxima admitida. A
leitura limitada contabiliza bytes efetivos e verifica término, CRC e SHA-256.

Versões de manifesto e de envelope do banco são lidas antes do DTO estrito,
incluindo valores acima de `u32`, acima de `u64`, expoentes como `1e9999` e
campos futuros. Números que não representem a versão inteira suportada são
incompatíveis, sem conversão intermediária para `f64`. A versão incompatível
resulta em erro específico, preservando os bytes de origem.

Antes de desserializar JSON externo para `Value`, a guarda compartilhada do
domínio recusa chaves do namespace interno `$serde_json::private::`, inclusive
quando escritas com escapes. Essa recusa é classificada como representação
incompatível, para impedir recuperação automática sobre um formato que não foi
interpretado. Texto JSON truncado continua sendo corrupção. A guarda não se
aplica a blobs binários opacos, cujo conteúdo permanece byte a byte. Extensões
declaradas têm tipo e versão próprios, conservados sem atribuir interpretação
ou capacidade instalada.

## 5. Verificação executada nesta parcela

Comandos, com toolchain 1.99.1 e `Cargo.lock` preservado:

```sh
cargo fmt -p prumo-storage
cargo test -p prumo-storage --offline --locked
cargo clippy -p prumo-storage -p prumo-domain --all-targets --offline --locked -- -D warnings
```

**30 testes passaram:** 16 de integração do banco, 11 de integração do pacote,
2 unitários e 1 teste de compilação recusada. As regressões direcionadas também
foram reexecutadas depois das correções de revisão.

| Grupo | Evidência |
|---|---|
| Transação | Rollback nas quatro fases mantém objeto, diário e cabeçalho anteriores; conflito entre dois escritores, repetição de identidade de comando, alteração de identidade/histórico e modo somente leitura são recusados |
| Recuperação | Backup ocupado termina com erro limitado; restauração inválida ou com sidecar preserva destino; comando administrativo idêntico conserva revisão, hash do projeto e hash do envelope |
| Legado | Baseline na revisão 2 conserva seu histórico real, sem fabricar comandos prévios; a revisão seguinte exige payload |
| Imutabilidade | Conteúdo estável de aproximadamente 440 kB permanece como um único objeto ao renomear, e cada objeto antigo conserva os bytes |
| Dados opacos | Extensão binária, metadados e campo JSON de domínio futuro sobrevivem à gravação, pacote, reabertura e restauração só do banco |
| Precisão e representação | Inteiro acima de `u64`, decimal extenso, `1e-9999` e `1e9999` sobrevivem ao snapshot e à reabertura; o namespace JSON reservado é incompatível, enquanto conteúdo binário com o mesmo texto é preservado |
| Limites compostos | Extensão 44 MiB provoca rollback completo por excesso do pacote final; 1.000 aliases de um blob 1 MiB são recusados antes das cópias ultrapassarem 64 MiB |
| Offline | O seed real de 171 referências e 368 capacidades, todas ainda sem liberação normativa, é fixado no projeto; catálogo, pacote normativo, snapshot e resultado de inspeção geométrica permanecem iguais após reabrir |
| Banco hostil | Aplicação estrangeira, versão futura, esquema com trigger, objeto adulterado e referência órfã são recusados sem reinicialização |
| Versão futura | Contêiner, banco, modelo, envelope atual e histórico são detectados como incompatíveis antes de descartar campos novos |
| ZIP hostil | Contagens/tamanhos/razão, caminhos, nomes Windows, colisões, sobreposição, cabeçalhos divergentes, prefixos com offsets ajustados, caudas, truncamento, links, diretórios e CRC adulterado são recusados |
| Autoridade | Projeções adulteradas continuam recusadas mesmo com novo hash correto no manifesto; campos e nomes repetidos no manifesto também são recusados |
| Reuso dentro da operação | Bytes de outro projeto não satisfazem a validação reutilizada do commit; o compilador recusa construir `EncodedPackage` externamente e o leitor recusa bytes alterados depois de consumi-lo |
| Reconstrução e backup | Cada campo exige exatamente um valor JSON completo; fragmentos com propriedades adicionais são recusados. Um backup consistente de conteúdo adulterado não é tratado como validado, e o snapshot integral o recusa |

A fixture `tests/fixtures/future-format.prumo` foi produzida por `zipfile` da
biblioteca padrão do Python, independente do escritor Rust do produto. Possui
221 bytes, apenas um manifesto de formato 2 e um campo futuro. SHA-256:
`14d0d4c4a43015b0c0723d6431b43ef64b80eb413c705cced691c30287576f64`.
Interface e MCP reutilizam esses bytes; não é um projeto futuro implementado.

A revisão independente encontrou e motivou correções de versão interna antes
da desserialização, alocação acumulada de aliases, limite composto do pacote,
prefixo ZIP com offsets ajustados e testes que poderiam aceitar uma recusa pela
razão errada. A otimização do leitor privado conserva a validação antes de
devolver qualquer conteúdo. O revisor também conferiu o tipo de saída privada,
o confronto exato dentro do commit e a classificação de números/representações
incompatíveis. A revisão de integração e os checks Windows/Linux do commit final
permanecem responsabilidade da entrega consolidada.

No domínio normativo, `compare_packages` continua validando seus dois argumentos.
O método privado de comparação é reutilizado dentro de `NormativeState::validate`
somente depois de validar o pacote ativo e cada pacote anterior da mesma chamada.
O teste `normative_history` altera separadamente os dois pacotes anteriores, o
ativo e o impacto de duas migrações, e mantém a recusa de cada adulteração.
Esse ajuste foi revisado independentemente e não cria cache entre operações.

## 6. Diagnóstico de desempenho e decisão

O fluxo normativo do ensaio fixa o catálogo de 171 referências e 368 capacidades,
cadastra a edição sintética `fixture-next` e exige migração explícita com os
hashes de origem/destino. A referência usa `example.invalid`; não representa
edição publicada ou aprovação profissional. O programa de observação está em
[PRU-005-latencia.rs](PRU-005-latencia.rs), e os tempos completos estão em
[PRU-005-latencia.json](PRU-005-latencia.json).

Cada etapa abaixo possui uma observação local em debug, com Rust 1.99.1. São
medições para localizar trabalho repetido durante a implementação, não uma
comparação estatística ou qualificação de desempenho da distribuição.

| Etapa observada | Cadastro da referência | Migração normativa |
|---|---:|---:|
| Antes das APIs de snapshot pareado | 8,764 s | Não medida |
| Após as APIs de snapshot pareado | 7,402 s | Não medida |
| Validação normativa única dentro da operação | 7,451 s | 11,681 s |
| Saída privada do encoder e reuso exato no commit | 7,295 s | 11,015 s |
| Comparação exata do backup com a imagem já validada | 6,794 s | 10,867 s |

As duas primeiras observações terminam na revisão 3, com pacote de 7.463.086
bytes. As três seguintes incluem migração e terminam na revisão 4, com pacote
de 12.660.812 bytes, preservando o perfil normativo anterior. IDs e datas variam
por execução; o conteúdo sintético e o catálogo mantêm a mesma composição.
O tempo de migração permanece acima de 10 s neste ensaio em debug. Os dados não
declaram aprovação desse prazo. O orçamento do teste MCP e a execução integrada
devem considerar protocolo, serialização e concorrência de testes separadamente.

Foi ensaiada e recusada outra otimização: montar JSON dos campos e desserializar
`Project` diretamente. Cada fragmento era validado como um valor completo,
porém `arbitrary_precision` e o buffer de enums internamente tagueados do Serde
fizeram `InspectionPayload` receber o número como mapa interno, incompatível
com seu campo `f64`. A prova real de catálogo, snapshot e inspeção falhou com
`invalid type: map, expected f64`. A reconstrução via `Value`, já qualificada,
foi restaurada, e a mesma prova passou novamente. A decisão preserva a fronteira
de desserialização já testada e evita ampliar o domínio para tratar apenas os
dois campos que revelaram o problema.

## 7. Reuso e manutenção

Foram reutilizados rusqlite 0.42.0, SQLite 3.54.1 e zip 9.5.1 com as features
mínimas documentadas na [ficha de componentes](../componentes/PRU-005.md).
`serde`, `serde_json`, `sha2`, `uuid` e `tempfile` já estavam qualificados no
workspace. Não há `unsafe`, SQL dinâmico fornecido pelo usuário, parser de
compressão próprio ou patch em dependência.

Fontes primárias utilizadas: [API de backup SQLite](https://sqlite.org/backup.html),
[sincronização SQLite](https://sqlite.org/pragma.html#pragma_synchronous),
[segurança SQLite](https://sqlite.org/security.html),
[rusqlite 0.42.0](https://docs.rs/rusqlite/0.42.0/rusqlite/),
[ZIP 9.5.1](https://docs.rs/zip/9.5.1/zip/) e os fontes publicados das duas crates.
A pesquisa de alternativas, licenças de binding/núcleo, vulnerabilidades,
avisos de distribuição e lock está na ficha do revisor.

Uma atualização dessas dependências precisa repetir as provas de esquema,
backup, concorrência, bytes/CRC, prefixos, sobreposição, expansão, nomes e
igualdade com o banco. Aumentar limites ou versões requer decisão explícita;
nenhuma recuperação remove dados desconhecidos para fazer o arquivo caber.
