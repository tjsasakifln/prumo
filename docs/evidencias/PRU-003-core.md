# PRU-003: geometria, álgebra linear e propriedades geométricas de seção

## Escopo entregue

Esta parcela implementa três interfaces substituíveis em `prumo-components`: `GeometryAdapter`, `LinearSolverAdapter` e `SectionAdapter`. Cada interface tem uma implementação com reuso real, entradas tipadas, limites, erros e uma bateria pública de qualificação. A mesma bateria pode testar outra implementação, sem mudar suas referências ou tolerâncias.

Os resultados são evidências de experimentos delimitados. Não são resultados de um projeto estrutural, validação normativa, dimensionamento ou aprovação profissional. Não há montagem de elementos finitos, materiais, combinações de ações nem verificação de resistência nesta parcela. A execução integrada mantém `engineering_release = false`.

| Módulo | Implementação | Entrada e saída | Referência independente |
|---|---|---|---|
| `geometry` | `NalgebraGeometry` | Pontos e translação em m; rotação adimensional; pontos transformados e limites por eixo | Rotação positiva de 90°, identidade, translação inteira e triângulo 3-4-5 |
| `solver` | `NalgebraCholesky` | Matriz em N/m, vetor em N, solução em m; resíduo e condição adimensionais | Inversa fechada 2×2, três molas em série e divisão escalar |
| `sections` | `GeoSection` | Coordenadas em m; área em m²; centroide em m; momentos centrais em m⁴ | Retângulo, triângulo retângulo e seção L composta de dois retângulos |

Os arquivos próprios estão em `crates/prumo-components/src/geometry.rs`, `solver.rs` e `sections.rs`. O contrato comum dos relatórios está em `prumo_domain::components`. Interface, MCP e processo de execução usam essas mesmas funções de qualificação.

## Reuso e alternativas

### Geometria e álgebra linear

Foi adotado `nalgebra = 0.35.0`, com `default-features = false` e somente `std`. A origem é [dimforge/nalgebra](https://github.com/dimforge/nalgebra), com licença Apache-2.0. O pacote publicado informa o commit `5f927f6c821d0ced07a0e086e5f37251cab2a8c3` em `.cargo_vcs_info.json`. Versão, licença e requisitos foram conferidos também no manifesto do pacote baixado pelo Cargo.

A biblioteca fornece operações matriciais, transformação de vetores e Cholesky. A [documentação de Cholesky 0.35.0](https://docs.rs/nalgebra/0.35.0/nalgebra/linalg/struct.Cholesky.html) informa que a construção usa o triângulo inferior. Por isso o adaptador verifica a simetria da matriz inteira antes da decomposição. Não usa `new_unchecked` nem `new_with_substitute`; uma matriz fora do domínio produz erro.

| Possibilidade | Decisão neste recorte | Custo ou consequência |
|---|---|---|
| Implementar operações matriciais e decomposição próprias | Rejeitada | Criaria outro núcleo numérico, com manutenção e validação desnecessárias para esta etapa |
| Usar a biblioteca diretamente em UI e MCP | Rejeitada | Espalharia unidades, limites e tratamento de falhas entre consumidores |
| Integrar a biblioteca por interfaces Rust pequenas | Adotada | Reutiliza o mesmo componente em geometria e álgebra linear, com contratos e bateria comuns |
| Criar fork ou contribuir alteração no projeto de origem | Não necessário no experimento | Nenhum defeito do componente exigiu patch; atualizar não depende de manter alterações próprias nele |

O ensaio não compara desempenho de nalgebra com motores estruturais completos. A análise de PyNite, OpenSees e SuiteSparse pertence à ficha geral da PRU-003. A capacidade demonstrada aqui permanece restrita à álgebra linear densa e às transformações declaradas.

### Seções

Foi adotado `geo = 0.33.1`, com `default-features = false`, para validação do polígono, área e centroide. A origem é [georust/geo](https://github.com/georust/geo), com licença MIT OR Apache-2.0. O pacote publicado informa o commit `5ab50b072e34a915a33eaa46c80763d51d61295e`, caminho `geo`. A [interface Validation](https://docs.rs/geo/0.33.1/geo/algorithm/validation/trait.Validation.html) permite recusar a geometria inválida antes do cálculo das propriedades.

Os momentos centrais são uma pequena integração própria pelo contorno. O código desloca o polígono primeiro à origem local e depois ao centroide, evitando subtrair grandes momentos globais para obter uma pequena inércia local. `geo` não foi apresentado como fornecedor da fórmula de inércia. Não houve extração de código de terceiros.

Também foi pesquisado `sectionproperties`, versão 3.10.2 observada no [manifesto oficial](https://github.com/robbievanleeuwen/section-properties/blob/master/pyproject.toml), licença [MIT](https://github.com/robbievanleeuwen/section-properties/blob/master/LICENSE). A [análise geométrica documentada](https://sectionproperties.readthedocs.io/en/stable/examples/analysis/geometric_analysis.html) passa por geometria, malha e objeto de análise. O [guia de malhas](https://sectionproperties.readthedocs.io/en/stable/user_guide/meshing.html) e a [visão das análises disponíveis](https://sectionproperties.readthedocs.io/en/stable/user_guide/overview.html) mostram seu domínio mais amplo.

| Possibilidade | Decisão neste recorte | Custo ou consequência |
|---|---|---|
| Reescrever validação de polígonos, área e centroide | Rejeitada | Duplicaria operações fornecidas por geo, inclusive o tratamento de geometria inválida |
| Integrar sectionproperties por processo externo | Mantida como candidata para funções mais amplas | Exige empacotar Python e dependências numéricas e de malha; não foi executada nem recebeu aprovação nesta etapa |
| Encapsular geo e acrescentar somente os momentos centrais | Adotada para o domínio declarado | Mantém integração Rust e contraprovas independentes para cada propriedade própria |
| Ampliar o domínio para furos, torção ou plasticidade | Fora desta adoção | Exigirá novo estudo de reuso e campanha específica antes de tornar a função disponível |
| Manter fork ou patch upstream | Não necessário | Nenhum patch ou arquivo externo alterado |

A decisão é sobre o esforço de integração observado para este recorte. Não afirma superioridade de velocidade, precisão ou manutenção em relação a sectionproperties, que não participou da medição executável. Os avisos e dependências transitivas dos componentes efetivamente incorporados constam do inventário geral da PRU-003; desativar features padrão não significa ausência de dependências transitivas.

## Domínios, unidades e falhas

### Transformações

O adaptador aceita de 1 a 4096 pontos, em `f64`. Coordenadas de entrada, translação e saída devem ser finitas e ter módulo até 10⁹ m. A transformação aplica primeiro a rotação e depois a translação. A matriz deve satisfazer ortogonalidade e determinante +1, ambos dentro de 10⁻¹². Escala e reflexão são recusadas.

A saída contém os pontos e a caixa alinhada aos eixos. Não qualifica sólidos, operações booleanas, malhas, geodesia ou conversão IFC. Uma entrada inválida falha sem produzir saída parcial.

### Sistema linear

O domínio é uma matriz densa simétrica definida positiva com 1 a 64 incógnitas. Dimensões, finitude e magnitude são verificadas antes da decomposição. Coeficientes e cargas têm módulo até 10²⁰ nas respectivas unidades. A solução deve ser finita e ter módulo até 10⁹ m. A assimetria máxima admitida é 10⁻¹² vezes a norma infinito da matriz.

O diagnóstico de resíduo é calculado a partir da matriz original, fora da rotina de fatoração:

\[
r = \frac{\|Ax-b\|_\infty}{\|A\|_\infty\|x\|_\infty+\|b\|_\infty}.
\]

O denominador nulo, para matriz e solução válidas, corresponde ao caso de carga e solução nulas. O limite do resíduo relativo é 10⁻¹². A condição é estimada em ponto flutuante por `||A||∞ × ||A⁻¹||∞`, limitada a 10¹⁰. Não é apresentada como número de condição espectral exato nem estimada somente pelas diagonais de Cholesky. O ensaio 2×2 verifica separadamente o valor conhecido `25/11` dessa norma.

São recusadas matrizes singulares, indefinidas, assimétricas, mal condicionadas além do limite ou com dados incompatíveis. Não se acrescenta rigidez, vínculo ou regularização para obter uma solução.

### Seção poligonal

O adaptador aceita um polígono simples, convexo ou côncavo, com 3 a 512 vértices e fechamento final opcional. As duas orientações são aceitas. Furos são representados no tipo de entrada e explicitamente recusados, sem desaparecimento silencioso.

Coordenadas finitas têm módulo até 10⁹ m. A dimensão máxima é 10⁶ m, cada aresta mede pelo menos 10⁻⁹ m e a área deve superar `10⁻¹² × dimensão_máxima²`. Autointerseção, repetição consecutiva, colinearidade e propriedades numericamente indefinidas são recusadas.

`Ixx` representa a integral de `(y-yc)²`; `Iyy`, de `(x-xc)²`; e `Ixy`, de `(x-xc)(y-yc)`. O produto de área tem sinal e não é a entrada negativa de um tensor. O triângulo e sua rotação de 90° verificam essa convenção. Não há resistência de material associada às propriedades.

## Referências independentes e tolerâncias

As referências são expressões fechadas preparadas fora da implementação adotada. Nenhuma saída de nalgebra ou geo é reutilizada como resultado esperado. Retângulos e triângulos usam suas fórmulas analíticas; a seção L usa a soma de dois retângulos disjuntos e o teorema dos eixos paralelos. As três molas usam alongamentos `F/k` e soma acumulada.

| Ensaio | Tolerância numérica fixada |
|---|---|
| Coordenadas e comprimentos geométricos | `10⁻¹⁰ m + 10⁻¹² × módulo da referência` |
| Solução linear | `10⁻¹⁴ m + 10⁻¹⁰ × módulo da referência` |
| Resíduo linear relativo | `10⁻¹²` |
| Área da seção | `10⁻²⁰ m² + 10⁻¹⁰ × área de referência` |
| Centroide | `10⁻¹² m + 10⁻¹⁰ × módulo da coordenada local de referência` |
| Ixx e Iyy | `10⁻²⁸ m⁴ + 10⁻¹⁰ × módulo do momento de referência` |
| Ixy | `10⁻²⁸ m⁴ + 10⁻¹⁰ × max(Ixx, Iyy) de referência` |

A tolerância do centroide usa a origem local. Uma coordenada global de 10⁸ m não aumenta a tolerância para centímetros. A parte relativa da tolerância de Ixy usa a escala dos momentos centrais do ensaio para permitir a verificação de produto de área nulo sem uma tolerância absoluta desproporcional em seções pequenas.

As escalas de 0,001, 1 e 1000 participam da bateria. Há referências de zero, sinais negativos, orientação reversa, rotação, translação grande e seção côncava. Testes adicionais verificam a inversa da transformação, permutação de incógnitas, o limite válido de 64 incógnitas, fechamento do polígono e rejeição além dos limites.

Cada interface também tem uma implementação propositalmente errada em teste. Ela é submetida à mesma bateria pública, que detecta a falha. Isso verifica o papel da referência e o contrato de substituição; não é evidência de um segundo backend de produção.

## Medições e manutenção

O relatório reproduzível da execução integrada fica em `docs/evidencias/PRU-003-metricas.json`. Ele identifica perfil de compilação, plataforma, binário, tamanho e tempos medidos. As funções medem a bateria real com `Instant` e execução preservada por `black_box`: geometria faz 128 repetições, sistema linear 64 e seções 64. O número não é controlável pelo chamador MCP.

O tempo inclui a bateria, suas verificações, alocações e preparação do relatório. Não representa tempo isolado de uma multiplicação matricial, importação BIM ou resolução de projeto. A primeira bateria que falha é preservada até o fim da medição, mesmo se todas as repetições seguintes passarem. Um teste com implementação intermitente verifica esse comportamento. O tamanho da distribuição é medido no executável integrado, sem atribuir artificialmente seus bytes a um único adaptador. As medições do executável final e do build limpo são consolidadas na evidência geral antes da decisão final da issue.

Contagem do código próprio desta parcela, obtida após formatação:

| Arquivo | Linhas físicas, incluindo comentários e testes | Linhas antes do módulo de testes | Linhas de testes | Testes |
|---|---:|---:|---:|---:|
| `geometry.rs` | 411 | 300 | 111 | 5 |
| `solver.rs` | 502 | 391 | 111 | 5 |
| `sections.rs` | 560 | 473 | 87 | 5 |
| Total | 1473 | 1164 | 309 | 15 |

Outras medidas da integração: três interfaces públicas substituíveis; dois componentes externos diretos para esta parcela; zero patches upstream; zero fronteiras FFI próprias; zero novos runtimes de linguagem; zero usos de `unsafe` nestes três arquivos. A crate proíbe `unsafe` próprio. As dependências externas têm seu exame separado e não recebem essa afirmação por herança.

Essas são medidas do trabalho de integração e de sua superfície de manutenção. Não equivalem a horas futuras garantidas ou redução comprovada de custo. A estimativa qualitativa é que a atualização exigirá conferir mudanças e licenças dos dois componentes, executar a mesma bateria e testar novamente o executável distribuído. Uma expansão de domínio exige novas referências e ensaios; não basta atualizar a versão.

## Verificação executada

Em 14/11/2026, os 15 testes próprios desta parcela passaram com a toolchain fixada do repositório. A verificação foi executada com:

```sh
cargo test -p prumo-components --lib --offline --locked
cargo clippy -p prumo-components --lib --offline --locked -- -D warnings
```

As baterias públicas contêm 24 verificações de geometria, 41 de sistema linear e 66 de seções. Compilação, testes de BIM/documentos/processo/WASM, revisão independente, distribuição e matriz Windows/Linux pertencem à evidência integrada da PRU-003. Este registro da parcela não declara sozinho o encerramento da issue.

## Conferência administrativa da medição e do pacote

Os scripts `scripts/measure_components.py` e `scripts/package_dev.py` foram examinados antes da medição limpa. São ferramentas administrativas, fora do runtime Rust do produto. A revisão encontrou quatro pontos corrigidos: fontes e binários não eram reconferidos após as execuções; a medição usava `assert`, removível por Python otimizado; comandos de identificação de Git e Rust dependiam do diretório do chamador; e o empacotador era chamado pelo nome genérico `python`.

Após a correção, a medição registra e confere novamente as fontes, entradas empacotadas e identidades dos executáveis. Cada relatório deve corresponder ao hash e ao tamanho do worker congelado, protocolo 1, suíte `PRU-003.1`, plataforma nativa e aos seis componentes com suas versões exatas, sem duplicação. A aceitação também exige verificações aprovadas, contagem positiva de repetições, tempos não negativos e erro/tolerância finitos, não negativos e coerentes. A verificação usa erros explícitos, preservados no modo otimizado do Python.

Os comandos Git e Rust identificam o repositório a partir de seu diretório correto. A medição chama o empacotador com o próprio interpretador Python. Rótulos Linux/Windows x86_64 são conferidos contra o sistema e a arquitetura, e outros alvos são recusados. Os três executáveis extraídos são iniciados com `--help` na pasta temporária; o worker extraído executa a bateria completa. Isso comprova a extração e execução no mesmo sistema, com suas bibliotecas nativas, sem afirmar teste em outra máquina ou abertura gráfica do pacote por esse comando.

A conferência executável foi feita sem recompilar o worker: o relatório real do executável existente foi aceito e dez adulterações foram recusadas sob `python -O`. Os casos alteraram protocolo, suíte, hash, tamanho, unicidade do componente, versão, repetições, erro acima da tolerância, erro NaN e tolerância negativa. O snapshot então continha 53 arquivos e permaneceu idêntico em duas leituras. As chaves de caminhos usam `/`, independentemente do sistema. A sintaxe dos dois scripts foi conferida por `py_compile`.

A inspeção de `process.rs` e dos testes de fronteira confirmou que o worker próprio é recolhido por `kill`/`wait` nos caminhos de cancelamento, prazo e falha; o leitor do pipe é reunido antes do retorno. O objeto proprietário cobre saídas antecipadas. Não foi necessário alterar Rust nesta conferência. O alcance permanece o processo próprio revisado, sem generalização para descendentes ou programas nativos arbitrários.
