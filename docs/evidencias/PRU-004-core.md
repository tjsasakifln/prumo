# PRU-004: domínio físico, analítico, unidades e identidades

## Entrega e limites

O schema nativo 2 representa objetos físicos, nós e membros analíticos, materiais declarados, ações nodais, vínculos físicos/analíticos, ambientes, redes, circuitos, portas e conexões. Os contratos e as edições ficam em `prumo-domain`, compartilhados pelo serviço da interface e do MCP.

`PhysicalObject` tem identidade própria e papel explícito: `visual_only` ou `analytical_source`. Todos os objetos continuam no estado `modelled`. Um objeto apenas visual não aceita vínculo analítico. Nenhuma dessas entidades é apresentada como elemento dimensionado, material certificado ou resultado normativo.

Esta parcela fornece os dados e operações do modelo. O snapshot canônico, grafo de resultados e inspeção geométrica são implementados em `canonical.rs`, `dependencies.rs` e no serviço de aplicação, com evidência específica. A inspeção de comprimentos não constitui análise estrutural.

## Uma autoridade por informação

As coleções existentes de objetos, ambientes, redes, portas e conexões continuam sendo as autoridades desses dados. O novo `SemanticModel` contém nós/membros analíticos, vínculos, materiais, ações, identidades, histórico de derivação, referencial de engenharia e georreferência. Não há uma segunda lista de objetos físicos escondida em extensões.

O campo singular `PhysicalObject.analytical_id` deixou de representar a relação no schema 2. A coleção explícita `PhysicalAnalyticalLink` permite um físico ligado a vários analíticos e vários físicos ligados ao mesmo analítico. Referências inexistentes, pares repetidos e vínculos com objetos apenas visuais são recusados. Um membro analítico pode ser preparado antes de receber vínculo; essa preparação não o torna calculado ou aprovado.

Materiais guardam família, módulo elástico, densidade e coeficiente de Poisson fornecidos. O domínio exige módulo e densidade positivos e `-1 < ν < 0,5`. Os dois extremos de ν são recusados. Os dados não recebem automaticamente uma classe ou conformidade normativa.

## Grandezas e reuso

O domínio reutiliza [uom 0.38.0](https://docs.rs/crate/uom/0.38.0) com `std`, `si` e `f64`, sem features padrão, Serde ou conversão automática entre sistemas de base. A pesquisa, licença, alternativas e auditoria estão em [PRU-004.md](../componentes/PRU-004.md). Os textos Apache-2.0 e MIT da publicação integram os avisos do produto. Não houve patch no componente.

Os conversores usam uom para aplicar unidades e offsets; tipos privados do Prumo impõem significado e limites. A implementação reaproveita ainda nalgebra 0.35.0, qualificada na PRU-003, para matrizes e vetores dos referenciais. Serde/serde_json e UUID mantêm os papéis já qualificados.

| Tipo próprio | Semântica e representação |
|---|---|
| `Point3`, `Length` | Coordenada tridimensional em m e comprimento escalar em m, com construtores validados |
| `Force3` | Componentes de força em N; entrada aceita `newton` ou `kilonewton` |
| `Moment3` | Componentes de um casal livre em N·m; entrada aceita `newton_metre` ou `kilonewton_metre` |
| `Pressure` | Quantidade positiva em Pa utilizada para módulo elástico; não é a guarda de uma pressão manométrica futura |
| `Stress` | Quantidade assinada em Pa, separada da guarda positiva do módulo |
| `Density` | Massa específica positiva em kg/m³ |
| `Mass` | Massa em kg, com conversão de tonelada |
| `AbsoluteTemperature` | Temperatura absoluta em K, inclusive conversão de °C; não aceita valor abaixo de 0 K |
| `TemperatureInterval` | Intervalo de temperatura, sem aplicar o offset de temperatura absoluta |
| `Time` | Tempo em s, com conversões de minuto e hora |
| `Angle` | Rotação em rad, com conversão de graus |
| `Stiffness` | Rigidez linear em N/m, encapsulando a razão dimensional SI força/comprimento em tipo próprio |

`Stiffness` não é exposta como tensão superficial ou exposição radiante apenas porque essas grandezas compartilham dimensão. A classificação semântica continua no tipo próprio. Nenhuma biblioteca de fluidos ou radiação é adotada por esse detalhe dimensional.

As entradas de ponto, força, momento, módulo e densidade têm enums de unidade fechados. Força em um campo de momento falha na desserialização. Um doctest comprova que atribuir `Force3` a `Moment3` também não compila. As representações SI internas de pontos e escalares usam campos cuja unidade é declarada pelo nome ou contrato, como `position_m` e `rotation_radians`; não inferem unidade pela magnitude do número.

NaN, infinitos e valores fora dos limites declarados são recusados após a conversão para SI. Zero negativo é normalizado para zero positivo em pontos, forças, momentos e escalares, sem arredondar outros valores. A troca de unidade de apresentação não altera os valores SI das entidades.

A revisão independente encontrou perda de um bit no parser JSON sem a feature `float_roundtrip`: o valor `1.9109059804556539` era reaberto como `1.910905980455654`. O teste anterior por `serde_json::Value` não atravessava texto e não detectava esse defeito. A feature foi habilitada no workspace e um teste adicional escreve bytes JSON, lê esses bytes e compara os bits de posição, força e casal, além do modelo completo. A contraprova falhou antes da mudança e passou depois dela, sem arredondar dados ou alargar tolerância.

## Identidade e compatibilidade com v1

UUID interno, GUID IFC e referência externa de origem são dados separados. O codec IFC próprio é pequeno, baseado na codificação pública de 128 bits, sem extração de código do IfcOpenShell. O [exemplo oficial da buildingSMART](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/lexical/IfcGloballyUniqueId.htm) foi usado como contraprova nas duas direções:

```text
UUID:     f70dd363-bfe3-495d-84a0-2c02dcb7d4d2
GUID IFC: 3t3TDZl_D9NOIWB0BSjzJI
```

São conferidos 22 caracteres, alfabeto permitido e primeiro caractere entre 0 e 3. O modelo também exige unicidade e rejeita identidade nula por regra própria. A validade do código não significa que a entidade já tenha representação IFC exportável.

`decode_project` aceita o DTO v1 completo e fechado, ou o schema 2. A conversão de v1 ocorre em memória e preserva UUID do projeto, objetos, identificadores analíticos, entidades MEP, extensões, unidades, pacote normativo, revisão e histórico. Cada antigo `analytical_id` torna-se o UUID de um membro analítico real, com nós e vínculo explícitos.

Os novos nós e vínculos da migração recebem UUIDs derivados de SHA-256, com namespace e papel definidos. Duas leituras do mesmo arquivo legado produzem os mesmos identificadores e dados. Os IDs que realmente existiam ficam como `Imported`; nós e vínculos sintetizados ficam como `Derived`, com a identidade de origem indicada. A migração não atribui a uma entidade derivada um identificador externo que nunca existiu.

O formato antigo não registrava autoria e revisão de criação por entidade. Por isso esses campos ficam ausentes na migração. Os GUIDs IFC locais atribuídos aos legados são determinísticos, derivados dos UUIDs preservados; não são apresentados como GUIDs recuperados de um arquivo IFC anterior. Novas criações recebem identidade nova e procedência explícita. Abrir o modelo não regrava o arquivo; o serviço cuida da preservação do original v1 antes da primeira mutação persistida.

## Cópia, fusão e divisão

Cada operação cria novos UUIDs e GUIDs para os novos objetos. O histórico conserva pais, filhos, revisão e ator. Fusão e divisão mantêm os objetos e vínculos retirados, com suas identidades e geometria, permitindo reabertura sem perder o significado da derivação.

A cópia é explicitamente física: copia geometria e atributos, aplica a translação declarada e registra a origem. Não cria automaticamente nós, ações, portas ou conexões. Os vínculos do objeto original permanecem intactos.

A fusão atual aceita segmentos colineares e contíguos, com mesma disciplina, papel, material, ambiente e extensões. Diferenças não são apagadas silenciosamente. Os vínculos analíticos dos pais são reunidos no novo físico. A divisão separa um segmento na fração declarada e liga os dois novos físicos aos mesmos membros analíticos de origem. Ela não é uma discretização de elementos finitos.

Operações de fusão ou divisão sobre objetos com portas são recusadas, exigindo tratamento explícito da conectividade. O recorte evita escolher silenciosamente a qual filho pertence uma porta ou substituir relações de engenharia.

Na reabertura, a validação confere unicidade e cardinalidade dos pais/filhos, ordem das revisões, nascimento dos filhos na revisão/autoria do evento, aposentadoria dos pais de fusão/divisão, preservação dos pais de cópia naquele momento e presença dos objetos retirados. Uma aposentadoria anterior ou igual à criação é recusada.

## Referenciais e invariância física

O modelo distingue movimento rígido ativo de troca passiva de referencial. Matrizes de rotação devem ser finitas, ortonormais e ter determinante +1, dentro de 10⁻¹². Escala e reflexão são recusadas como movimento rígido.

O referencial de engenharia declara origem e rotação para um espaço de referência abstrato. Trocar esse referencial converte coordenadas, eixos e ações mantendo as posições e vetores físicos nesse espaço. Georreferência é metadado separado: CRS declarado, origem cartográfica, rotação e escala. Ela não executa projeção geodésica nem altera silenciosamente comprimentos e ações.

Uma ação nodal referencia seu nó de aplicação, guarda força e casal livre separadamente e informa se suas componentes usam o referencial de engenharia ou os eixos de um membro incidente. O casal livre apenas gira. A resultante de momento em relação a um ponto é calculada explicitamente como:

\[
\mathbf M_O=(\mathbf r_P-\mathbf r_O)\times\mathbf F+\mathbf C.
\]

O teste independente usa força `(0,10,0)` N aplicada em `(1,2,0)` m e casal `(0,0,3)` N·m. A resultante sobre a origem original é 13 N·m. Com nova origem física `(5,-1,0)` m, a referência independente passa a -37 N·m. Uma rotação de 90° do referencial conserva a resultante sobre o mesmo ponto físico, enquanto o casal livre permanece 3 N·m no eixo correspondente. O teste distingue o efeito de mudar o ponto de referência do efeito de girar um casal.

Mover uma seleção que compartilha membro ou nó analítico com objetos/membros não selecionados é recusado. A seleção completa transforma cada nó uma vez. Reordenar coleções por UUID não altera geometria nem ação física; inverter a orientação de um membro exige converter as componentes locais correspondentes, pois a ordem dos nós tem significado.

## Topologia MEP

Portas pertencem a objetos e têm disciplina, tipo e posição. Redes/circuitos referenciam portas explicitamente. Uma porta não pertence simultaneamente a duas redes. Conexões exigem duas portas distintas da mesma rede, sem pares repetidos. Circuitos são limitados às disciplinas elétricas e de sinal previstas no contrato.

Um cruzamento geométrico, mesmo com portas coincidentes, não cria conexão. O teste representa dois tubos cruzados, cria portas e rede e comprova que a conexão só surge depois de `ConnectPorts`. Portas incompatíveis com a disciplina são recusadas sem alterar o projeto.

## Atomicidade e fronteiras de entrada

`apply_model_edit` trabalha em candidato separado e só substitui o projeto em memória depois da validação do modelo. A validação considera a próxima revisão para proveniência, sem antecipar a validação do grafo. O serviço compara os índices de fonte, invalida dependências, acrescenta revisão/histórico e executa a validação completa antes de persistir.

O cliente MCP revelou um caso relevante de Serde: variantes vazias representadas como unit variants podiam ignorar campos extras mesmo com o enum fechado. `SourceOrigin::Native {}` e `ActionFrame::Engineering {}` passaram a ser variantes struct vazias. A forma JSON permanece a mesma e agora recusa campos adicionais, como um GUID forjado em uma origem nativa. Há prova direta no domínio e prova por cliente MCP.

Os campos opcionais de comandos de atualização também distinguem ausência de remoção explícita. `SetPhysicalMaterial` exige a chave `material_id`, e `SetGeoreference` exige `georeference`; remover requer `null` explícito. Um desserializador dedicado sem valor padrão mantém `Option` no tipo Rust e rejeita chave ausente. Campos opcionais na criação continuam opcionais. O teste focal verifica as duas recusas, a remoção explícita e a compatibilidade das criações.

## Verificação executada

Os comandos abaixo passaram com a toolchain e o lock do repositório:

```sh
cargo check -p prumo-domain --offline --locked
cargo test -p prumo-domain --offline --locked
cargo clippy -p prumo-domain --all-targets --offline --locked -- -D warnings
```

Foram executados 17 testes novos na bateria integrada de `crates/prumo-domain/tests/model.rs`, três testes anteriores e um doctest de compilação negativa. Um 18º teste focal passou após a correção de presença dos campos opcionais de atualização. A bateria cobre conversões, temperaturas, sinal, grandezas inválidas, GUID oficial, ida e volta nativa também por texto com preservação dos bits, n:m, órfãos, visualização sem vínculo, extremos de ν, movimentos/referenciais, torque, permutação, seleção compartilhada, cópia/fusão/divisão, história adulterada, conexão MEP explícita, mudança de unidade de apresentação, migração determinística, campos extras em variantes vazias e remoção somente por null explícito.

A revisão independente apontou e motivou correções nos extremos de Poisson, na coerência de linhagem e na leitura de números JSON; a integração MCP revelou a recusa incompleta de campos extras. As correções têm testes correspondentes. Serviço, snapshots, invalidação, UI/MCP, pacote e CI são conferidos na evidência integrada da PRU-004. Esta ficha não substitui esse fechamento nem declara cálculo de engenharia disponível.
