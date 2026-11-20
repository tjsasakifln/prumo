# Executar o Prumo

Esta é uma versão de desenvolvimento para modelar, persistir e consultar projetos. Não calcula nem dimensiona estruturas ou instalações.

## Instalação de desenvolvimento

Instale Rust por [rustup](https://rustup.rs/). No Windows, instale também as ferramentas C++ do Visual Studio para o target MSVC. Em Linux, use um ambiente gráfico X11 ou Wayland com OpenGL; as bibliotecas de desenvolvimento usadas no CI estão em `.github/workflows/rust.yml`.

Na raiz do repositório:

```sh
cargo build --workspace --locked
cargo run -p prumo-desktop --locked
```

O `rust-toolchain.toml` instala a versão exata 1.99.1. Não há configuração manual obrigatória. A tela permite escolher o arquivo, criar projeto e unidade, adicionar uma peça, renomear, cadastrar ambiente e reabrir. Cada comando grava sua revisão no banco transacional de trabalho e publica o pacote `.prumo`, com autosave e cópia anterior quando aplicáveis. Coordenadas de entrada respeitam a unidade escolhida e o modelo conserva metros internamente.

## Arquivo, backup e recuperação

Abra **Arquivo e recuperação** para consultar formato, integridade, revisão e cópias disponíveis. **Inspecionar arquivo** atualiza o diagnóstico sem modificar o conteúdo. O painel continua disponível quando o projeto não pode ser aberto. **Identidades do pacote** mostra versões, hashes e conteúdos verificados; os detalhes ficam recolhidos durante a edição normal.

Arquivos JSON legados podem ser abertos sem conversão automática na leitura. Para convertê-los antes de editar, confira o diagnóstico, marque **Confirmo migrar o arquivo e manter a cópia original** e escolha **Migrar arquivo Prumo**. A primeira alteração válida também faz essa migração e conserva os bytes originais. O modelo mantém identidades, propriedades e extensões desconhecidas aceitas pelo domínio. A conversão do schema de modelo 1 para 2 preserva os identificadores e cria os nós e vínculos antes representados por um único identificador analítico.

**Criar backup verificado** conserva uma cópia da revisão publicada. O arquivo `.prumo` é o pacote portátil; o banco e as fontes locais de recuperação ficam na pasta de trabalho administrada pelo serviço, junto ao projeto. Preserve essa pasta ao transferir também o histórico de recuperação local. Ela não precisa ser editada nem aberta manualmente.

Se o principal estiver corrompido, ausente ou houver uma revisão pendente, confira **Cópias disponíveis**. Escolha pelo nome, revisão e identidade, verifique o projeto e o hash do candidato e marque **Confirmo restaurar este candidato e preservar o arquivo atual**. **Recuperar candidato selecionado** usa o mesmo serviço das demais operações e reabre o modelo restaurado. O conteúdo substituído é preservado; uma cópia incompatível ou um diagnóstico desatualizado é recusado.

Alterar o caminho digitado, trocar o candidato ou inspecionar novamente descarta a confirmação anterior. Se a inspeção detectar que o arquivo mudou desde a abertura, o editor é retirado até **Reabrir / atualizar** carregar o conteúdo atual. As edições feitas pela interface também levam ao serviço a identidade e o hash do arquivo exibido; uma substituição externa é recusada mesmo antes de inspecionar e mesmo que conserve o número da revisão. As ações de armazenamento conferem seu próprio hash de diagnóstico. Uma versão futura no principal, nas cópias ou no banco de trabalho bloqueia escrita e informa a incompatibilidade. O aplicativo não oferece criar um projeto sobre esse conteúdo. Selecionar outro arquivo continua disponível.

O funcionamento do projeto é local e não exige conexão. As garantias de pacote, transação e interrupção constam da [matriz PRU-005](evidencias/PRU-005.md); os ensaios de uso da interface e MCP estão em [PRU-005-ui-mcp.md](evidencias/PRU-005-ui-mcp.md).

## Modelo físico, analítico e instalações

Abra **Modelo físico e analítico**. As oito áreas reúnem peças, modelo analítico, materiais, ações, cópia e transformação, redes e portas, origem e unidades, medidas e dependências. Os seletores usam entidades do projeto. Os campos numéricos aceitam vírgula ou ponto decimal, sem separadores de milhares; um texto inválido permanece visível e impede a gravação.

Uma peça **Somente visual** não cria elemento analítico. Para relacioná-la ao modelo analítico, altere sua função, crie nós e barra e estabeleça os vínculos. Um físico pode ter vários vínculos e um analítico pode ser compartilhado. Materiais e ações têm unidades declaradas; N ou kN não são aceitos em um campo de momento. Os eixos locais, o movimento rígido e a mudança de referencial de engenharia são operações explícitas. O mapa georreferenciado permanece separado do referencial de cálculo.

Em **Redes e portas**, crie a rede ou circuito, as portas e suas atribuições. Conecte as portas pelo comando correspondente. Duas peças ou portas coincidentes não ficam conectadas pela aparência do desenho. O contrato valida a disciplina, o tipo de porta e a rede.

Em **Medidas e dependências**, **Atualizar medidas do modelo** salva comprimentos e contagens reais da geometria, com revisão e identidade da base. Modificar uma dependência marca as medidas afetadas como obsoletas e preserva valor anterior, snapshot e causa. Essas medidas não são resultados de dimensionamento. Renomear o projeto ou alterar a unidade de apresentação preserva seus valores físicos.

As ferramentas MCP equivalentes são `prumo.model.edit`, `prumo.model.inspect`, `prumo.model.snapshot` e `prumo.results.trace`. As duas primeiras exigem escrita e revisão atual. As consultas funcionam em leitura. Consulte [MCP](MCP.md) para o contrato e [evidências da PRU-004](evidencias/PRU-004.md) para invariantes e limites.

## Lotes, revisão e desfazer

Em **Alterações em lote e histórico**, marque **Preparar alterações em lote antes de salvar** e use os formulários usuais. As operações ficam pendentes até **Conferir prévia do lote**. Confira efeitos e invalidações antes de **Aplicar lote conferido**. A aplicação usa o mesmo plano e os mesmos identificadores preparados. Se outro cliente alterar o projeto, reabra e prepare novamente.

**Desfazer última alteração** e **Refazer alteração** gravam revisões auditadas. O histórico não é apagado nem resultados antigos são revalidados por terem voltado os mesmos valores. Os recibos idempotentes sobrevivem à reabertura. A interface e o MCP utilizam o mesmo serviço transacional.

## Vistas e modelagem paramétrica

O painel de vistas compartilha a seleção entre árvore, propriedades, planta, corte e 3D pelo UUID do objeto. Use os botões para enquadrar, localizar, ocultar, isolar, escolher vista e recortar por faixa. Os comandos também admitem teclado, escala de interface e alto contraste. A navegação conserva a revisão técnica; falha gráfica mantém a árvore textual disponível. Um recorte visual não altera a geometria salva.

Em **Eixos, níveis e encaixes**, cadastre cotas, eixos inclinados e referenciais locais. Medidas aceitam `3 m + 25 cm`, `(2,5 m + 50 cm) / 2` e deslocamentos negativos. A unidade é obrigatória; separadores de milhares ambíguos são recusados. A edição de um nível prepara o impacto no painel de alterações antes de mover suas dependências. Encaixes mostram o ponto proposto e não criam uma ligação estrutural.

Em **Pilares, vigas, barras e treliças**, escolha pontos por coordenadas, eixos, níveis, interseções ou nós; selecione material e seções do catálogo. A peça pode ter alinhamento, excentricidade, rotação, seção constante ou variável e, para viga curva, arco por três pontos. A prévia mostra a geometria física e analítica, comprimento e erro de aproximação do arco. Conexões compartilhadas exigem a escolha explícita dos nós. Barras de treliça carregam comportamento somente axial.

Os formulários de superfícies admitem contorno, aberturas, espessuras locais, rebaixos, planos inclinados, nervuras e grelhas. A prévia discrimina componentes, área, volume e peso próprio usando a densidade cadastrada. Aberturas são retiradas das regiões trianguladas. Apoios e condições de borda identificam os objetos candidatos e conservam a necessidade de análise posterior.

Os conjuntos paramétricos reúnem paredes, núcleos abertos ou fechados, vigas de acoplamento, escadas, rampas e patamares. Cada parte conserva identidade e região analítica própria. Continuidade e apoios são declarados, sem ligação automática pela proximidade visual. Peças arquitetônicas podem ser promovidas explicitamente; a alteração de um conjunto não transforma silenciosamente a sua referência de engenharia.

**Exportar projeto para vista web** salva uma cópia `.view.json` ao lado do projeto; a tela informa o nome. O visualizador WASM importa essa cópia e reutiliza o domínio de seleção e projeção em Rust. Não edita o pacote `.prumo`, não acessa o banco local e não altera a revisão. Consulte [PRU-011](evidencias/PRU-011.md) para construção, execução e limites verificados.

## Geometria, materiais e seções

Em **Geometria e conexões**, as tolerâncias de seleção, união topológica, malha e análise são independentes. Inspecione por disciplina, ambiente ou rede. O programa distingue duplicidade, vão curto, aproximação, cruzamento e conexão explícita. Para corrigir uma extremidade ou conectar portas, prepare a alteração, leia o impacto e aplique a prévia. Uma geometria semelhante não cria ligação resistente.

Em **Catálogos de materiais e seções**, crie uma seção retangular, circular, I ou caixão informando dimensões e fonte. Selecione uma entrada para calcular propriedades, consultar versões ou associá-la a uma peça. O editor de entrada completa admite polígonos, seções compostas, materiais isotrópicos e ortotrópicos, estados por condição e propriedades com unidades. O exemplo de material é explicitamente ilustrativo e deve ser preenchido com os dados e a origem usados no projeto.

**Preparar próxima versão** conserva a identidade e avança a revisão. **Preparar customização** cria uma identidade diferente e exige motivo, mantendo a versão e o SHA da base. Salvar passa pelas validações comuns. Valores nominais, medidos, característicos e de projeto permanecem distintos; um valor de projeto exige sua base declarada. A torção apresenta método e convergência; estimativa sem convergência não é apresentada como propriedade liberada.

Em **Verificação com casos de referência**, execute os casos analíticos de viga, treliça, pórtico e rejeição de mecanismo. A bateria tem referências e tolerâncias fixadas antes da execução e conserva relatório por SHA. Ela verifica o software e não calcula o projeto aberto. Mudar a revisão limpa a evidência exibida.

## Referências normativas e capacidades

Abra **Normas e capacidades** no projeto. A tela mostra a cobertura do inventário, as pendências técnicas e o pacote de referências disponível. **Fixar pacote de referências** preserva o conteúdo e sua identidade SHA-256 no arquivo do projeto.

Em **Cadastrar ou atualizar referência normativa**, selecione uma referência ou crie uma nova. O cadastro gera outra versão e apresenta o impacto previsto. O pacote ativo continua preservado até marcar a confirmação e escolher **Migrar pacote do projeto**. A comparação mostra partes, edições, emendas, erratas e capacidades afetadas antes da decisão. O histórico conserva todos os pacotes anteriores.

Selecionar uma capacidade apresenta os motivos que impedem um resultado final. O catálogo distribuído contém metadados públicos, sem textos normativos licenciados ou aprovações profissionais. Nenhuma de suas capacidades de engenharia está liberada. Informar uma edição, um responsável funcional ou uma fonte pública não supre a consulta autorizada, a interpretação e a qualificação exigidas.

## Verificação dos componentes

Após compilar o workspace, abra **Verificação de componentes** e escolha **Executar ensaios de componentes**. O processo separado executa os casos de referência de BIM, geometria, sistema linear, seção, desenho SVG e fronteira WASM. É possível cancelar a execução. A tela apresenta versão, licença, domínio, tolerância, resultado, tempo e identidade do executável.

O relatório registra a identidade, a revisão e o hash do projeto como contexto. Os casos são fixtures independentes, não cálculos do modelo aberto. Alterar ou reabrir outra revisão limpa a evidência da tela. Nenhuma operação muda o projeto, libera uma capacidade normativa ou permite executar código arbitrário. Os três executáveis devem permanecer juntos na instalação.

As mesmas operações estão disponíveis em MCP por `prumo.components.list` e `prumo.components.qualify`, inclusive no modo de leitura. A ficha [PRU-003](componentes/PRU-003.md) descreve as alternativas, licenças e decisões. A [matriz de evidências](evidencias/PRU-003.md) registra os contratos e medições.

## Servidor MCP local

```sh
cargo run -p prumo-mcp-server --locked -- --project exemplo.prumo
```

O nome exato do pacote e os argumentos do servidor também aparecem em `cargo run -p prumo-mcp-server -- --help`. O lançamento seleciona um único arquivo. O padrão permite somente leitura. `--allow-write` mantém compatibilidade para modelagem; use `--scopes read,model,calculation` para modelagem, casos de referência e propriedades de seção. `--disciplines` limita o modelo por disciplina e `--project-id` fixa a identidade esperada. O ator é definido no lançamento, nunca pelo conteúdo importado. O cliente não escolhe caminhos nem executa comandos do sistema. O protocolo usa stdio e a revisão MCP 2026-07-28. O perfil legado é explícito: `--protocol 2025-11-25`. A descoberta moderna e a inicialização legada são ensaiadas separadamente; dados de cache modernos não são enviados no perfil legado. A documentação específica em `docs/evidencias/PRU-001-mcp.md` descreve descoberta, ferramentas e testes.

As ferramentas `prumo.registry.read`, `prumo.registry.audit`, `prumo.norms.package` e `prumo.results.gate` consultam o registro. `prumo.norms.register`, `prumo.norms.pin` e `prumo.norms.migrate` usam o mesmo serviço da interface e exigem escrita autorizada e revisão atual. Fixação e migração também conferem os hashes do pacote.

`prumo.storage.status` consulta o arquivo e as fontes de recuperação mesmo quando `prumo.project.read` falha. A consulta não cria arquivos auxiliares. `prumo.storage.migrate`, `prumo.storage.backup` e `prumo.storage.recover` recebem a identidade do diagnóstico obtida nessa mesma autorização de lançamento; migração e recuperação também exigem confirmação explícita. Os candidatos usam identificadores devolvidos pelo serviço. Não há ferramenta para escolher caminho, executar SQL ou gravar diretamente no banco. O catálogo anunciado depende dos escopos e disciplinas concedidos pelo host; consulte `tools/list` para as funções permitidas.

Uma segunda instância deve reabrir o projeto para ver a última revisão. Uma alteração com revisão antiga é recusada e pede atualização, preservando a alteração já salva pela outra instância.

## Verificar

```sh
python3 scripts/validate_plan.py
cargo fmt --all -- --check
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check -p prumo-ui --target wasm32-unknown-unknown --locked
cargo build -p prumo-web --target wasm32-unknown-unknown --locked
python3 scripts/build_wasm_probe.py --check
python3 scripts/package_dev.py
```

Os testes de interface usam os widgets e a árvore de acessibilidade reais. Os testes de MCP iniciam um processo separado e utilizam o cliente do SDK oficial e um cliente de mensagens independente. O visualizador WASM reutiliza o domínio e a interface; sua entrada é uma cópia exportada, sem acesso ao armazenamento nativo.

## Matriz distribuída

| Destino | Compilador | Interface | Persistência |
|---|---|---|---|
| x86_64-pc-windows-msvc | Rust 1.99.1, edition 2024 | eframe, OpenGL e AccessKit | SQLite local e pacote `.prumo` |
| x86_64-unknown-linux-gnu | Rust 1.99.1, edition 2024 | eframe, X11/Wayland, OpenGL e AccessKit | SQLite local e pacote `.prumo` |
| wasm32-unknown-unknown | Rust 1.99.1 | Visualizador eframe com projeção e seleção comuns | Importação de cópia `.view.json`; sem escrita no projeto |

Features, dependências exatas e resolução transitiva são registradas em Cargo.toml e Cargo.lock. A primeira etapa distribui executáveis de desenvolvimento; instaladores e atualização pertencem às entregas posteriores.

O pacote contém `prumo-desktop`, `prumo-mcp` e `prumo-component-worker`, avisos de terceiros e manifesto de integridade. O empacotamento extrai em uma pasta temporária, inicia os três executáveis e executa a bateria de componentes a partir dessa instalação.

A medição da PRU-003 pode ser reproduzida com `python3 scripts/measure_components.py`. Ela exige que `target/qualification-clean-release` ainda não exista, compila todo o workspace em perfil release e gera cinco amostras, hashes, tamanho do pacote e a duração do build. O cache de downloads pode existir; os artefatos de compilação começam em diretório vazio. A otimização release não altera o estágio de desenvolvimento do produto.
