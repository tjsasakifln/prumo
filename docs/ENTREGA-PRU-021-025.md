# Entrega experimental PRU-021 a PRU-025

Estado: **implementação parcial, sem aprovação de engenharia nem aceite integral das cinco issues**. Esta branch depende da branch do PR #260, pois a integração e validação da issue #20 continuam pendentes.

## Escopo entregue e lacunas

| Issue | Capacidade implementada em Rust | Critérios obrigatórios ainda pendentes |
| --- | --- | --- |
| #21 | Montagem de matriz SPD esparsa por mapas, fatoração LDLt, deslocamentos, reações, resíduos, cancelamento e falhas por mecanismo/condicionamento; serviço inclui hashes do projeto e entrada | Worker isolado, comparação de implementação independente no binário distribuível, qualificação numérica e desempenho |
| #22 | Treliça axial 3D, pórtico 2D Euler-Bernoulli e Timoshenko, carga uniforme transversal EB, recuperação de esforços e convenções de sinal | Pórticos 3D, torção de Saint-Venant/empennamento, seção variável, offsets, cabos, carregamento térmico, liberações |
| #23 | Inspeção/refinamento de malha triangular, protocolo de convergência e elemento CST de membrana isotrópica em tensão plana, tensões e energia | Placa/casca com flexão, travamento, singularidades, patch tests de flexão, qualificação de malhas reais com aberturas |
| #24 | Vínculos multiponto sem ciclos, condensação K reduzida = Tt K T, recuperação, diafragma rígido 2D com excentricidade informada | Diafragmas flexíveis/semirrígidos validados, integração de barras, paredes e lajes, comparação de torção de pavimento |
| #25 | Livro de cargas nodais/lineares/superficiais, balanço de forças e momentos, distribuição conservativa por triângulos e grafo explícito de transferência até apoios | Distribuição automática pela rigidez, transferência estrutural de pilar sobre laje com MEF, cálculo de reações reais e comparação independente |

## Limites

A operação é um ensaio **com entrada SI explícita**, não lê automaticamente rigidezes, hipóteses, ações ou apoios do projeto BIM. O serviço de análise registra ID, revisão e SHA do projeto, além do SHA da entrada, e não altera o arquivo.

O solver experimental limita-se a 256 graus de liberdade, 65.536 termos, matrizes simétricas definidas positivas e razão indicativa de pivôs menor ou igual a 1e10. A razão de pivôs não é o número de condição espectral. Não adiciona estabilização artificial. O nome legado do coeficiente no contrato esparso é stiffness_n_per_m, enquanto os ensaios de pórticos usam blocos generalizados em N/m, N e N.m. A finalidade e o sistema SI de cada grau estão documentados no adaptador.

A treliça admite apenas esforço axial. O pórtico 2D admite UX, UY e RZ por nó; não representa flexão espacial ou empenamento. O CST tem apenas membrana, não flexão de lajes. Vínculos multiponto representam cinemática declarada, não validam a hipótese de diafragma rígido. Os caminhos de carga exigem frações fornecidas pelo projetista; a reação requerida por equilíbrio não é uma reação calculada por deformabilidade.

## Testes a executar no commit exato

- cargo fmt --all -- --check
- cargo clippy --workspace --all-targets --locked -- -D warnings
- cargo test -p prumo-analysis --test numerics --locked
- cargo test -p prumo-app --test analysis_trials --locked
- cargo test --workspace --locked
- cargo build --workspace --locked

Contraprovas nos testes: sistema 2x2 com resultado fechado [1/11, 7/11], invariância a permutação, estabilidade/cancelamento, barras axiais, deflexão de balanço PL³/(3EI) e acréscimo Timo PL/(kGA), momentos nulos em viga biapoiada, patch test CST de deformação uniforme, refino de malha, energia por condensação, deslocamentos de diafragma com excentricidade, carga de pilar sobre laje distribuída até dois apoios e fechamento independente de forças e momentos.

**Os testes foram adicionados ao código, não declarados como aprovados sem evidência de execução.**

## Reuso

Não foi acrescentada dependência externa. O componente anterior de álgebra densa via nalgebra 0.35.0, licença Apache-2.0, continua disponível para comparação de soluções de referência. A investigação de SuiteSparse, Eigen, PyNite, CALFEM, Gmsh, CalculiX e Code_Aster permanece condicionada a licença, manutenção, arquitetura, testes de fronteira e independência de referências de acordo com docs/REUSO.md. Menção no backlog não autoriza incorporação.

## Critério de integração

O MCP prumo.analysis.trial e o painel Rust chamam o mesmo ProjectService. A saída mantém engineering_release=false, independentemente do sucesso numérico do ensaio. Manter as issues #21 a #25 abertas, sem merge e sem aprovação do cálculo de projetos, até resolver as dependências anteriores, comprovar cada critério completo e validar a CI.
