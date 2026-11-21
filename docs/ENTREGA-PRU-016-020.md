# PRU-016 a PRU-020: implementação incremental e limites

Esta branch acrescenta ao domínio Rust um estado versionado de preparação pré-cálculo, persistido pelo serviço transacional existente, e conecta a mesma operação ao editor nativo e ao MCP. Nenhum método declara capacidade de solver, aprovação de engenharia ou equivalência entre referência e modelo analítico.

| Issue | Funcionalidade executável nesta PR | Limites e critérios ainda não atendidos |
| --- | --- | --- |
| 016 | Apoios nodais, seis graus de liberdade, orientação rotacionada, mola linear ou unilateral, liberação nas extremidades; rejeita vínculos repetidos e rigidez inválida | Links rígidos, excentricidades, testes do solver e matrizes independentes não concluídos |
| 017 | Cadastro rastreável de referências DXF/PDF/imagem por hash, dois fatores de calibração, detecção de discrepância de escala e promoção explícita de linhas | Leitor DXF completo, extração/vetorização PDF ou imagem, camadas e blocos ainda não disponíveis; candidatos são informados pelo usuário |
| 018 | Manifesto de referência IFC2x3/IFC4/IFC4.3, GUIDs, reprovação de duplicidades, substituição controlada por SHA, promoção selecionada com GUID no modelo editável | O manifesto não equivale à leitura e homologação IFC; faltam parsers abrangentes, unidades e coordenadas de três geradores, importação geométrica completa e testes com arquivos reais |
| 019 | Cópia e troca de material em lote de peças físicas sem gerador, até 500 peças; identidades distintas, relação com origem e marcação de alteração local | Cópia de pavimentos com filhos, instâncias paramétricas com propagação e exceções, biblioteca de modelos e contraprovas com desfazer ainda incompletas |
| 020 | Revisão orientada em seis etapas, diagnóstico de nó desconectado, nó coincidente, falta de material, carga nodal duplicada, dupla liberação, apoio ausente e escala inconsistente | Caminho completo de cargas, laje sem apoio, testes com usuários e etapa formal de prontidão ainda não concluídos |

## Evidências a executar

* cargo test -p prumo-domain --test precalculation --locked
* cargo test --workspace --locked
* cargo fmt --all -- --check
* cargo clippy --workspace --all-targets --locked -- -D warnings
* cargo check -p prumo-ui --target wasm32-unknown-unknown --locked

As ferramentas MCP prumo.precalculation.read e prumo.precalculation.review consultam o domínio da interface. A mutação usa prumo.model.edit com edit_precalculation e conserva as regras de acesso e revisão.

## Reuso

Não foi introduzida dependência de terceiros. O componente IFC4 local em crates/prumo-components permanece um ensaio limitado, não um importador integral. Bibliotecas candidatas devem ser avaliadas por licença, atualização e testes de fronteira, conforme docs/REUSO.md, antes de adoção.

## Integridade

Referências registram SHA, não caminhos ou permissão para arquivos arbitrários. Nenhum candidato é promovido automaticamente. Falhas de validação não gravam o projeto graças ao mecanismo transacional. Concluir o assistente não constitui aprovação profissional.

## Fechamento

Manter #16 a #20 abertas até satisfazer e verificar individualmente todos os critérios originais. A integração desta PR não comprova conclusão integral nem autoriza avançar à #21 antes das pendências obrigatórias.
