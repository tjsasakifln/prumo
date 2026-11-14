# Revisão do planejamento do Prumo

**Versão 2.1, em 14/11/2026.** Objeto: complementar a documentação e as 250 issues com Rust como linguagem obrigatória do código próprio do produto. O plano preserva 72 famílias obrigatórias e passa a conter 15 requisitos permanentes. Não houve implementação de produto, execução de build Rust, validação de cálculos ou aceite técnico profissional nesta revisão.

## Decisão incorporada

REQ-15 e TEC-RUST-01 vinculam Rust a todas as entregas, incluindo interface, BIM, redes, comandos, regras, cálculos próprios, persistência, serviços, CLI, MCP e adaptadores. A origem é a instrução do proprietário USR-05. A definição completa está em [RUST.md](RUST.md) e [technology.json](../planning/technology.json).

O reuso pertinente continua obrigatório. Bibliotecas externas podem manter sua linguagem original, com origem, versão, licença, função, adaptação Rust e testes. Ferramentas Python do planejamento permanecem fora do runtime. Frameworks e versões serão escolhidos por evidência durante a implementação.

## Revisões executadas

| Frente | Escopo conferido | Registro |
|---|---|---|
| Arquitetura Rust | Interface, MCP, Cargo, FFI, unsafe, componentes, fontes primárias e critérios de implementação | [Parecer de arquitetura](../planning/reviews/2.1-architecture.json) |
| Escopo e consistência | Comparação com a versão anterior, 250 issues, 72 famílias, requisitos, reuso, apresentação e preservação dos critérios de engenharia | [Parecer de escopo](../planning/reviews/2.1-scope.json) |
| Controles do plano | Renderizador, verificador, testes positivos e mutações negativas em cópias temporárias | [Ensaios do verificador](../planning/reviews/2.1-validator.json) |

Dois agentes fizeram revisão independente do conteúdo consolidado. Os scripts preparados por outro agente também foram examinados pelo revisor de escopo. O coordenador conferiu os pontos decisivos, aplicou as correções e verificou a publicação. Os pareceres finais não registram achados pendentes no recorte examinado. O [registro consolidado](../planning/review.json) distingue revisão documental de aprovação profissional.

## Correções e complementos

| Ponto encontrado | Tratamento |
|---|---|
| Rust aparecia como decisão parcial e a interface previa autoria TypeScript/React | Rust tornou-se obrigatório para toda implementação própria, incluindo interface e MCP |
| SDK TypeScript era candidato para o servidor próprio | SDK Rust rmcp tornou-se candidato prioritário; clientes externos de outras linguagens continuam possíveis para interoperabilidade |
| Dois textos tratavam rmcp como escolha já concluída | Corrigidos para servidor Rust com SDK qualificado e rmcp prioritário, preservando a avaliação por evidências |
| Um novo aceite de geometria falava da interface | Corrigido para geometria, topologia, tolerâncias e conversões próprias em Rust |
| Expressão sobre proteção dos logs era ambígua | Especificada a supressão ou o mascaramento de dados sensíveis, com exclusão de segredos |
| Reuso externo poderia ser confundido com autorização de outro núcleo próprio | Definidas fronteiras, papéis, origem e testes, preservando bibliotecas e extensões pertinentes |
| A decisão poderia ficar apenas em texto introdutório | Política estruturada, REQ-15 nas 250 issues, critérios específicos em 32 issues e verificações automáticas do plano |

A pesquisa acrescentou 13 fontes técnicas primárias e uma instrução do proprietário. O índice contém 222 identificadores de fonte, incluindo instruções; isso não significa 222 sites distintos. O catálogo passou de 28 para 32 candidatos com rmcp, egui/eframe, Dioxus e wgpu, todos ainda sujeitos a qualificação.

## Verificação de integridade

Foram conferidos sequência, dependências, identidades, 72 famílias obrigatórias, REQ-01 a REQ-15, fonte da decisão, política Rust, textos gerados, links locais, identidade do README/LEIAME e manifesto de publicação. Os requisitos anteriores e a matriz de cobertura foram preservados.

Vinte mutações negativas em cópias temporárias foram rejeitadas pelos diagnósticos esperados. Os casos cobrem linguagem ausente ou diferente, política tornada opcional, perda ou duplicidade de vínculo do REQ-15, interface fora de Rust, autorização indevida de lógica própria em outra linguagem, arquivo ausente ou malformado e regressões de cobertura, apresentação e manifesto. Três diagnósticos de arquivo/formato também foram conferidos no renderizador. O manifesto sintético desses ensaios ficou apenas na cópia de teste.

Após as correções de revisão, o plano final passou novamente pela conferência de renderização e pela validação integral. O verificador atesta integridade do planejamento. Não inspeciona código de produto ainda inexistente nem comprova correção numérica.

## Publicação das issues

As 250 issues existentes foram atualizadas e mantidas abertas. Cada resposta de atualização foi comparada com o texto aprovado. Uma leitura independente paginada da API do GitHub confirmou número, título, corpo integral e estado das 250 issues. O [manifesto](../planning/github-issues.json) registra os hashes desses corpos efetivamente conferidos.

Os textos foram aprovados antes da publicação. Depois disso, o backlog recebeu somente a atualização do estado de publicação. Os hashes anterior e publicado constam de [review.json](../planning/review.json). Nenhuma issue foi encerrada por esta alteração documental.

## Histórico e limite

A versão 1.0 continha 120 entregas estruturais. A versão 2.0 ampliou o produto para instalações e transferiu o lançamento completo para a issue 250. Os registros anteriores permanecem em [revisão 1.0](../planning/reviews/1.0.json), [revisão 2.0](../planning/reviews/2.0.json), [ensaios 2.0](../planning/reviews/2.0-validator.json) e no histórico Git.

A versão 2.1 fixa a linguagem de implementação sem reduzir capacidades, ensaios, tolerâncias ou condições de lançamento. As campanhas numéricas, a integração, os estudos com usuários e os aceites profissionais continuam obrigatórios nas respectivas issues.
