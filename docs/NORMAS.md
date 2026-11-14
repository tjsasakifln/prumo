# Normas e rastreabilidade técnica do Prumo

Atualizado em 14/11/2026. Este documento estabelece requisitos de implementação. A pesquisa realizada abrange metadados públicos; nenhum motor de cálculo ou conformidade normativa foi validado nesta etapa.

Relacionados: [plano de validação](VALIDACAO.md) e [catálogo de fontes](fontes-normas.json).

## Decisões que mudam o planejamento

1. O catálogo ABNT consultado já apresenta **NBR 6118:2026** e **NBR 8800:2024 Versão Corrigida:2025**. A notícia da ABECE identifica a emenda da 6118 de 11/03/2026. As versões dos manuais dos fabricantes devem ser confrontadas com essas edições antes de implementar regras. [N01, N02, N03]
2. A base de ações também mudou: a ABECE informa publicação da **NBR 8681:2025**. A mesma associação informa a **NBR 15421:2023** para sismos e a **NBR 15200:2024** para concreto em incêndio. São evidências primárias de participantes das comissões, sujeitas à confirmação de catálogo, emendas e texto licenciado na implantação. [N05, N06]
3. **NBR 7190 é uma série**, com a parte 1 para critérios de dimensionamento e as partes 2 a 7 para caracterização de materiais e ligações. O lançamento precisa de propriedades rastreáveis e das limitações dos produtos de madeira, além das fórmulas de barras. [N04, N07]
4. Manual do fabricante serve como fonte de comportamento e abrangência do produto. A definição normativa precisa vir do exemplar aplicável da norma, com sua edição e eventuais erratas. A última versão identificada publicamente é um ponto de partida, não substitui o controle de atualização antes de cada implementação e lançamento.
5. Equivalência e confiabilidade são metas de aceitação mensuráveis. O fechamento de issues não substitui evidências numéricas, revisão técnica independente e comprovação de que a versão final resolve os casos prometidos.

## Classes de evidência normativa

| Classe | Evidência | Uso permitido no projeto |
|---|---|---|
| A | Edição vista diretamente no catálogo da ABNT | Registrar metadados públicos; confirmar situação, incorporação de erratas e texto licenciado antes de implementar |
| B | Notícia ou publicação de comissão, associação participante ou instituto responsável | Registrar edição como referência identificada; exigir confirmação no catálogo antes de congelar o pacote |
| C | Documentação oficial do fabricante de software | Identificar norma que o fabricante declara suportar; não tratar como prova da edição atual |
| P | Referência necessária por escopo, sem edição confirmada nesta pesquisa | Criar pendência explícita; não preencher ano por memória e não liberar a função sem resolver |

**Situação desta entrega:** metadados públicos pesquisados; textos completos licenciados não adquiridos nem conferidos nesta sessão. Antes da implementação de cada regra, obter acesso legal ao texto aplicável e revisão técnica da interpretação. O repositório deve guardar identificação, rastreabilidade e testes próprios; não deve publicar exemplares integrais ou reproduções de tabelas protegidas sem autorização. A ABNT disponibiliza aquisição e visualização no seu próprio ambiente. [N15]

## Registro inicial de normas

| Norma ou família | Referência pública identificada | Classe | Cobertura a tratar no Prumo |
|---|---|---|---|
| NBR 6118 | 2026 no catálogo; emenda 1 de 2026 da edição 2023 descrita pela ABECE | A | Concreto armado e protendido, ELU, ELS, durabilidade, detalhamento e avaliação técnica aplicável |
| NBR 8800 | 2024, versão corrigida 2025 no catálogo | A | Aço e estruturas mistas de edificações; barras, ligações, estabilidade, ELU/ELS |
| NBR 7190-1 | 2022, publicação de 29/06/2022 informada pela ABECE | B | Dimensionamento e requisitos gerais de estruturas de madeira |
| NBR 7190-2 a 7190-7 | Série publicada em 2022, registrada no boletim CBIC | B | Classificação, ensaios, ligações mecânicas, madeira laminada colada e colada cruzada |
| NBR 8681 | 2025 informada pela ABECE, participante da revisão | B | Ações, resistências, situações de projeto e combinações coerentes entre materiais |
| NBR 6120 | 2019 identificada em material institucional ABECE; erratas atuais a conferir | B | Ações permanentes e variáveis por ocupação e casos especiais de edifícios |
| NBR 6123 | 2023 informada pela ABECE; situação de correções posteriores a confirmar na ABNT | B | Ação estática/dinâmica do vento; hipóteses, topografia, categoria, direção e limites do modelo |
| NBR 6122 | Base 2019 e processo de emenda em 2022 documentado pela ABMS; publicação consolidada atual pendente | B | Projeto e execução de fundações; geotecnia, investigação, capacidades, recalques e ensaios |
| NBR 9062 | 2017 e comentários publicados pelo IBRACON; situação atual a confirmar | B | Pré-moldados, ligações e fases de produção, transporte, içamento e montagem |
| NBR 14931 | Revisão de 2023 informada pela ABECE | B | Exigências de execução que afetam detalhamento, formas, escoramento e especificação |
| NBR 15200 | 2024 informada pela ABECE; boletim CBIC registra errata em 24/10/2024 | B | Verificação de estruturas de concreto em incêndio e relatório de método/aplicabilidade |
| NBR 14323 | Revisão de 2013 informada em relatório institucional CBCA; edição atual a conferir | B | Estruturas de aço e mistas em incêndio |
| NBR 14432 | Edição não confirmada nesta pesquisa | P | Resistência ao fogo e relação com exigências locais, sem inferir TRRF só do material |
| NBR 14762 | Edição atual não confirmada; processo de revisão não prova publicação | P | Perfis de aço formados a frio e seus modos locais, distorcionais e globais |
| NBR 16239 | Publicação de 2013 registrada pelo CBCA; edição atual a conferir | B | Perfis tubulares e ligações abrangidas |
| NBR 15421 | 2023 informada pela ABECE | B | Aplicabilidade sísmica, classificação, análise e verificações |
| NBR 15575-1 | 2025 no catálogo | A | Desempenho geral relacionado ao projeto estrutural residencial |
| NBR 15575-2 | 2013 no catálogo | A | Desempenho de sistemas estruturais; não confundir com prova de atendimento de toda a série |
| NBR 15575-3, -4, -5 e -6 | Respectivamente 2021, 2025, 2025 e 2021 no catálogo | A | Interfaces de pisos, vedações, coberturas e instalações, somente no alcance declarado |
| NBR 7480 e correlatas de fios/cordoalhas | Edições não confirmadas nesta pesquisa | P | Catálogos de armaduras e protensão, propriedades e certificados |
| NBR 12655 e normas de ensaio do concreto | Edições não confirmadas nesta pesquisa | P | Especificação, controle, propriedades e aceitação de materiais |
| NBR 14861 | Edição não confirmada nesta pesquisa | P | Lajes alveolares pré-moldadas, interfaces e dados de produto previstos no backlog |
| NBR 14859, série | Partes e edições a confirmar | P | Lajes pré-fabricadas, elementos e produtos previstos na matriz obrigatória de abrangência |
| NBR 16935 e correlatas | Existência e articulação com a 14931 identificadas pela ABECE; edição a confirmar | P | Concreto com fibras se incluído na matriz de equivalência |
| NBR 16055 | 2022 identificada na documentação oficial TQS; situação atual a confirmar na ABNT | C | Paredes de concreto moldadas no local, limites, interação e detalhamento do sistema, distintos da verificação genérica de pilares-parede [N23] |
| NBR 16868, série | Partes e edições a confirmar | P | Alvenaria estrutural prevista na matriz obrigatória de abrangência |
| Normas e critérios de alumínio estrutural | Base de dimensionamento e edição a selecionar e licenciar | P | Abrangência de alumínio conforme matriz funcional; não reutilizar verificações NBR 8800 apenas alterando propriedades |
| NBR 11682 e NBR 5629 | Edições não confirmadas nesta pesquisa | P | Estabilidade de encostas/contenções e tirantes conforme os métodos previstos no backlog |
| NBR 6457, 6484, 6489, 8036 e ensaios geotécnicos pertinentes | Edições e aplicabilidade a confirmar | P | Dados de sondagem, investigação e provas de carga; não estimar suporte do solo por geometria do edifício |
| Normas de produtos de aço, perfis, soldas e parafusos | Conjunto por tipo de componente a identificar | P | Catálogos certificados, execução e verificações de ligações |
| Normas de representação, documentação e BIM | Conjunto por entrega a identificar | P | Peças executivas e intercâmbio; validar semântica do conteúdo além da conformidade de arquivo |

Fontes de grupos: N01 a N14. A lista é um inventário inicial de aplicabilidade. A issue de base normativa deve completar referências cruzadas dos textos licenciados; nenhum grupo P fica presumido como implementado. Obras de arte especiais, pontes e estruturas marítimas precisam de definição própria de escopo e normas; não se tornam suportadas porque um solver aceita sua geometria.

## Arquitetura de conformidade

### Pacote normativo imutável por projeto

Cada projeto fixa `code_bundle_id` com versão, lista de documentos e situação de vigência verificada em data conhecida. Cada referência inclui código, parte, edição, errata/emenda, título, editor, URL pública, evidência de acesso licenciado, data de verificação e responsável pela interpretação. Apenas metadados de licença, sem credenciais ou conteúdo protegido, entram no repositório.

O pacote associa cada regra a: identificador estável, item normativo, interpretação técnica própria, domínio de aplicação, unidades, entradas, hipóteses, procedimento adotado, estados possíveis, resultados intermediários, testes independentes e registro de revisão. A rastreabilidade liga `requisito -> regra -> implementação -> caso de ensaio -> evidência -> versão entregue`.

Uma revisão normativa cria pacote novo. Projetos existentes abrem com o pacote original, mostram o histórico e permitem comparação explícita. Migrar exige comparar premissas e resultados, recalcular, revisar alertas e invalidar aprovações anteriores afetadas. Atualização silenciosa é vedada.

### Resultado explicável e incompletude visível

Cada verificação apresenta elemento, combinação, esforço, resistência/limite, unidade, taxa de utilização, equação ou método identificado, item normativo e hipóteses. Valores intermediários devem permitir reconstituição independente. Arredondamento de apresentação não pode alterar a decisão de aprovação.

Estados mínimos distintos: `não calculado`, `dados incompletos`, `não aplicável com justificativa`, `fora do domínio`, `não convergiu`, `calculado e não atende`, `calculado e atende`, `resultado desatualizado`. O estado geral não pode ficar aprovado com verificação obrigatória ausente.

Catálogos e modelos pré-configurados possuem domínio documentado. A interface simples ajuda a escolher e corrigir; ela não elimina decisões como tipo de apoio, ligação, exposição, estabilidade lateral, geometria do solo, diafragma ou comprimento de flambagem.

### Fronteira entre agentes e cálculo

Agentes criam propostas estruturadas e podem executar as mesmas ações disponíveis à interface com autorização e registro. Todos os resultados estruturais vêm de módulos determinísticos versionados. Uma resposta de linguagem natural, palpite ou texto recuperado não pode entrar como resistência ou critério sem origem verificável.

Toda alteração material invalida os resultados dependentes e marca pranchas/memoriais anteriores. O pacote de cálculo registra hash do modelo físico, modelo analítico, ações, combinações, materiais, pacotes normativos, solver, parâmetros, malha, versão do aplicativo e dependências relevantes. A emissão final associa revisor habilitado e versão técnica exata; o simples nome de um agente não atende esse papel.


## Condições para fechar a base normativa

- Completar a matriz de aplicabilidade a partir de todos os textos licenciados e suas referências cruzadas, preservando a abrangência obrigatória da matriz funcional.
- Verificar no catálogo ABNT as edições e eventuais emendas/correções das classes B e P; registrar comprovante e data de consulta.
- Registrar quem possui acesso autorizado, em quais condições ele pode ser usado pela equipe e quem revisará a interpretação. Essa lista estará inicialmente pendente; nenhum nome ou habilitação deve ser presumido.
- Criar pelo menos um teste de aplicação e um de rejeição para cada requisito implementado, acrescentando fronteiras e mudanças de regime pertinentes.
- Vincular requisitos, funções, testes e evidências a uma versão imutável.
- Documentar conflitos de interpretação e resolvê-los antes de disponibilizar o resultado como verificado.
- Provar reprodução de projeto antigo e migração explícita para um pacote novo, com identificação de resultados afetados.

As pendências de acesso às normas e de revisão técnica pertencem à execução das issues e impedem o aceite das capacidades correspondentes. O planejamento e a criação do repositório não as declaram cumpridas.

## Referências públicas

Todos os links foram consultados em 14/11/2026. Títulos e notícias comprovam apenas os fatos indicados; não substituem o exemplar completo aplicável.

- **N01**. [ABNT Catálogo](https://www.abntcatalogo.com.br/). ABNT. Catálogo exibe 6118:2026; 8800:2024 Versão Corrigida:2025; partes 1 a 6 da 15575 com edições individualizadas. Metadados apenas.
- **N02**. [NBR 6118 tem emenda publicada pela ABNT](https://site.abece.com.br/nbr-6118-tem-emenda-publicada-pela-abnt/). ABECE. Notícia de março de 2026 identifica emenda de 11/03/2026, equivalência à edição 2026 e alterações referentes à avaliação técnica de projeto.
- **N03**. [NBR 8800 tem versão corrigida publicada](https://site.abece.com.br/nbr-8800-tem-versao-corrigida-publicada/). ABECE. Publicação de fevereiro de 2025 identifica versão corrigida de 2025 da norma de 2024, incluindo ajustes em fórmulas.
- **N04**. [Nova NBR 7190 é publicada](https://site.abece.com.br/nova-nbr-7190-e-publicada/). ABECE. Parte 1 publicada em 29/06/2022; a associação registra critérios de dimensionamento, madeira engenheirada e incêndio.
- **N05**. [Em vigor, a nova NBR 8681](https://site.abece.com.br/em-vigor-a-nova-nbr-8681/). ABECE. Notícia de outubro de 2025 registra nova edição da norma de ações e segurança e participação direta da associação na revisão.
- **N06**. [Normas Técnicas](https://site.abece.com.br/normas-tecnicas/). ABECE. Lista trabalhos das comissões; registra 15200:2024, 8681:2025 e 15421:2023. A seção da 6118 antecede a notícia mais recente de 2026.
- **N07**. [Boletim de normas técnicas do setor da construção civil junho/julho 2022](https://cbic.org.br/wp-content/uploads/2022/07/boletim-de-normas-tecnicas-setor-da-construcao-civil-junhojulho2022.pdf). CBIC. Boletim setorial registra partes 1 a 7 da série 7190, publicadas em junho de 2022, incluindo métodos de ensaio e madeira engenheirada.
- **N08**. [Em vigor, a nova NBR 6123](https://site.abece.com.br/em-vigor-a-nova-nbr-6123/). ABECE. Notícia registra edição 2023 e ação estática/dinâmica do vento; não comprova por si eventuais correções posteriores.
- **N09**. [ABNT convida associados ABMS para reunião sobre Projeto de Emenda à Norma de Projeto e Execução de Fundações](https://www.abms.com.br/noticia/abnt-convida-associados-abms-para-reuniao). ABMS. Publicação de 21/03/2022 descreve consulta de emenda da 6122:2019. Exige conferir a consolidação publicada posteriormente no catálogo.
- **N10**. [ABNT NBR 9062:2017 Comentários e Exemplos](https://www.ibracon.org.br/loja/detalhes/4780). IBRACON. Publicação de 2022 vinculada ao comitê IBRACON/ABCIC comenta edição 2017 e disponibiliza erratas do livro. Situação atual da norma exige confirmação.
- **N11**. [Nova NBR 14931 é publicada](https://site.abece.com.br/nova-nbr-14931-e-publicada/). ABECE. Notícia de maio de 2023 descreve revisão de execução e interfaces com concreto armado, protendido e fibras.
- **N12**. [Alterações de normas técnicas do setor da construção civil 2024](https://cbic.org.br/wp-content/uploads/2025/02/boletim-alteracoes-de-normas-tecnicas-setor-da-construcao-civil-2024.pdf). CBIC. Boletim registra errata da NBR 15200 em 24/10/2024; incorporação à versão licenciada deve ser conferida.
- **N13**. [Relatório de atividades 2013](https://www.cbca-acobrasil.org.br/upfiles/downloads/relatorios/CBCA-Relatorio-de-Atividades-2013.pdf). CBCA/Aço Brasil. Relatório institucional registra publicação da 16239 e revisão da 14323 em 2013; não demonstra situação atual.
- **N14**. [Palestras e Lives](https://site.abece.com.br/palestras-lives/). ABECE. Acervo institucional identifica apresentações sobre a NBR 6120:2019; edição e erratas devem ser conferidas na ABNT.
- **N15**. [FAQ do ABNT Catálogo](https://abntcatalogo.com.br/faq.aspx). ABNT. Informa aquisição de normas e disponibilização eletrônica em visualizador próprio. Pesquisa pública não equivale a acesso licenciado ao texto.
- **N23**. [Norma NBR 16055: 2022](https://docs.tqs.com.br/Docs/Details?id=1971660650&language=pt-BR). TQS. Documentação do fabricante declara adaptação do módulo de paredes de concreto à NBR 16055:2022; situação atual e texto aplicável devem ser conferidos na ABNT.
