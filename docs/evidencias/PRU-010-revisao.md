# PRU-010: revisão independente da formulação e dos contratos

Revisão cruzada pelo agente responsável pelas transações, separado do autor do catálogo e da formulação de torção. O escopo lido compreende `catalogue.rs`, `quantities.rs`, `sections.rs`, `sections/torsion.rs`, a seleção de fontes em `canonical.rs`, os testes e as fichas de evidência e reuso.

## Conferência matemática

As propriedades de seção usam área, centroide e momentos centrais. Foram conferidos o sinal declarado de Ixy, a subtração dos furos independentemente da orientação e o transporte dos momentos de componentes pelo teorema dos eixos paralelos. Área tem dimensão de comprimento ao quadrado; Ixx, Iyy, Ixy e J têm dimensão de comprimento à quarta potência.

A formulação de torção corresponde à minimização de

`integral[(omega,x - y)^2 + (omega,y + x)^2] dA`.

Sua primeira variação produz `K*omega=f`, com `Kij=integral(gradNi.gradNj)` e `fi=integral(y*Ni,x - x*Ni,y)`. Substituir a solução na energia resulta em `J=Ixx+Iyy-f^T*omega`, como implementado. Assim, o momento polar não substitui silenciosamente a constante de torção.

Os gradientes dos elementos triangulares quadráticos são lineares. Os integrandos da matriz e da carga têm grau máximo dois; a regra de três pontos usada no código integra esses termos exatamente. Fixar um valor escalar do empenamento remove apenas sua constante arbitrária em cada componente conectado. A solução conserva as condições naturais dos contornos externos e internos.

A mudança para coordenadas adimensionais retorna à unidade física por `scale^4`. O pré-condicionador incompleto pode deslocar sua própria diagonal para obter pivôs positivos; a matriz e a carga do problema permanecem iguais. O código confere o resíduo contra essa matriz original antes de aceitar a solução linear.

Na preparação da malha, a bisseção atua sobre todas as faces adjacentes à aresta escolhida. As trocas de diagonal usam apenas arestas compartilhadas por dois triângulos, verificam a convexidade da região e o predicado robusto de círculo. As arestas de contorno e dos furos não são trocadas. O critério final permanece fixo: duas variações sucessivas de J até 0,2%, pelo menos três refinamentos e resíduo relativo até 4e-10, com limites de nós e triângulos. Esse é um critério de convergência de malha; não é uma demonstração de limite rigoroso de erro para qualquer contorno possível.

## Contraprovas novas, separadas da implementação

Os dois testes de `crates/prumo-domain/tests/catalogue_review.rs` foram escritos pelo revisor. As tolerâncias foram registradas antes da execução. Ambos passaram com `cargo test -p prumo-domain --test geometry --test geometry_review --test catalogue_review --offline --locked` em Rust 1.99.1/Linux.

| Caso | Referência independente | Limite e resultado |
|---|---|---|
| Elipse com semieixos 2 m e 1 m, aproximada por 128 vértices | `J=pi*a^3*b^3/(a^2+b^2)=8*pi/5 m^4`. A solução fechada de empenamento é proporcional a `x*y`; não usa a montagem nem o solver do produto. | Diferença até 0,5%, incluindo a aproximação poligonal; estado `Converged`. Passou. O resultado também é claramente inferior ao momento polar. |
| Mesma elipse com todas as dimensões divididas por quatro | Área dividida por 16 e J dividido por 256. | A área obedeceu à escala com limite relativo 1e-12; J, com limite relativo 1e-9. Passou. |
| Anel com raios 1 m e 0,5 m, 64 vértices por contorno | No anel circular, o campo tangencial é ortogonal às normais dos dois contornos: o empenamento constante satisfaz a condição natural. Logo `J=pi*(R^4-r^4)/2=15*pi/32 m^4`. | Diferença até 0,5% em J e 0,2% na área, incluindo a aproximação poligonal; estado `Converged`. Passou. O furo foi preservado. |

Nenhum desses ensaios alterou as tolerâncias do solver ou as referências existentes. Os testes conferem `engineering_release=false` mesmo quando as comparações passam.

Foi executado também `cargo test -p prumo-domain --test catalogue nonconverged_thin_i --offline --locked`. O caso passou: o I fino atingiu o limite de refinamento com estimativa numericamente plausível, mas conservou `NotConverged`, `j_m4=None` e ausência de liberação. A aproximação por placas finas dentro de 5% não contornou a exigência de convergência.

## Contratos e achados complementares

A seleção de fonte foi conferida: cada entrada efetiva tem digest próprio; a extensão completa do catálogo fica fora do digest de metadados gerais. A função de fontes por peça inclui a identidade física e as entradas atribuídas. A revisão de uma entrada invalida suas dependências sem tornar versões anteriores invisíveis. A atribuição de material legado e de catálogo é mutuamente exclusiva, e customização conserva sua base histórica.

Foram corrigidos e relidos dois pontos antes dos gates finais. `CatalogMaterial::state_for` agora valida todos os pares de condição com `value.require(quantity)`, inclusive os que não participam de um intervalo; o teste adicional cobre unidade incompatível e divergência entre a chave e o tipo físico do valor. A ficha de reuso passou a identificar `uom` 0.38.0 como dependência já utilizada pelas unidades do domínio. Não foi identificado desvio na formulação numérica examinada.

O resultado desta revisão é uma verificação de software e de consistência matemática para o domínio declarado. A hipótese da torção é homogênea, isotrópica e de Saint-Venant, sem qualificação de dimensionamento, centro de cisalhamento, empenamento restringido ou interação entre materiais. Cadastrar propriedades ortotrópicas não altera a formulação da consulta geométrica. Revisão profissional, uso de dados reais e identidade do artefato integrado não são presumidos por este documento.
