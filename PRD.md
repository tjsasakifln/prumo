# Prumo: requisitos de produto

**Versão do planejamento:** 2.0. **Data:** 14/11/2026. **Estado:** especificação, sem software de cálculo implementado.

## 1. Objetivo

Criar um sistema brasileiro profissional de estruturas e instalações prediais com a cobertura funcional pertinente documentada nos ecossistemas Eberick, CYPECAD, TQS e QiBuilder/Builder, unindo concepção, análise, dimensionamento, detalhamento, coordenação e documentação. O uso deve se aproximar da montagem de peças com conexões claras, mantendo as hipóteses de engenharia compreensíveis e verificáveis.

O usuário poderá começar manualmente, pedir trabalho a um agente, revisar a alteração, continuar pela interface e emitir uma entrega coerente. As duas formas de operação acessarão o mesmo modelo e os mesmos comandos. O motor de cálculo será determinístico, independente de modelos de linguagem.

O planejamento inicial criou o repositório e a fila estrutural; esta revisão amplia a especificação e registra o trabalho necessário para estruturas e instalações. A sequência tem **250 issues de implementação**, com conclusão numérica estrita. Esta especificação não declara que as funcionalidades, as metas de desempenho ou a confiabilidade já foram atingidas.

## 2. Usuários e tarefas

O público inclui engenheiros e projetistas habilitados nas respectivas atribuições, escritórios, revisores e equipes de estruturas e instalações prediais no Brasil. Abrange concreto, aço, madeira e sistemas complementares, além de oito disciplinas de instalações.

As tarefas principais são conceber estruturas e instalações, receber arquitetura, lançar peças e ações, compreender o caminho de cargas, calcular, localizar impedimentos, comparar alternativas, detalhar, revisar e emitir. O sistema também deve permitir verificar elementos isolados com esforços externos rastreáveis, sem exigir um edifício fictício.

A facilidade de operação não elimina a necessidade de conhecimento de engenharia. O produto deve reduzir o esforço para expressar e conferir uma decisão, mantendo visíveis os dados ausentes, os limites do método e a responsabilidade pela entrega.

## 3. Decisões iniciais de produto

- **Nome:** Prumo.
- **Plataforma de lançamento planejada:** desktop Windows com operação local e interface web reutilizável. A escolha considera os fluxos CAD e o cálculo de modelos grandes. O ADR da issue 1 confirma a base; a issue 3 qualifica componentes e alternativas.
- **Autonomia local:** abrir, editar, calcular e gerar entregas não depende de um provedor de agentes ou de um serviço remoto. Recursos de colaboração podem usar serviço autenticado sem retirar a operação local.
- **Formato nativo:** pacote `.prumo` versionado, preservando os dados completos de cálculo. IFC é intercâmbio BIM e não substitui dados de análise que seu schema não represente.
- **Interface:** plantas, cortes, 3D, propriedades e diagnósticos sincronizados. Entrada numérica com unidades e alternativas a arrastes/gestos simultâneos.
- **Automação:** MCP real desde a issue 1; cada nova operação relevante amplia simultaneamente a interface e o contrato de comandos.
- **Reuso:** avaliação obrigatória de componentes abertos pertinentes antes de escrever um módulo equivalente do zero. A escolha considera licença, adequação, segurança, manutenção e validação própria.
- **Negócio:** produto comercial, sem fixação prematura de preço, cobrança ou assinatura. O modelo escolhido será implementado e testado antes do lançamento, preservando o acesso do cliente aos próprios projetos.

## 4. Requisitos permanentes

| ID | Requisito | Evidência exigida |
|---|---|---|
| REQ-01 | Cobertura funcional dos sistemas de referência no recorte brasileiro | Matriz de capacidades, fontes, issues e casos qualificados |
| REQ-02 | Normas brasileiras e correlatas com versões rastreáveis | Registro normativo, regra/item/domínio, testes e revisão |
| REQ-03 | BIM nativo desde a primeira entrega | Objetos físicos/analíticos ligados, persistência e identidades estáveis |
| REQ-04 | MCP desde a primeira entrega | Cliente real criando/consultando o mesmo projeto da interface |
| REQ-05 | Operação intuitiva e acessível | Estudo com novos engenheiros, métricas e ausência de erros críticos |
| REQ-06 | Reuso criterioso de código aberto | Decisão por módulo, licença, versão, testes e plano de manutenção |
| REQ-07 | Confiabilidade demonstrada | Referências independentes, fronteiras, projetos completos e revisão profissional |
| REQ-08 | Resultados e documentos coerentes | Snapshot, hashes, invalidação e pacote de emissão único |
| REQ-09 | Execução sequencial até lançamento | Dependências, código integrado e evidências antes da próxima issue |
| REQ-10 | Produto comercial operável | Instalação, atualização, suporte, direitos de distribuição e manutenção |
| REQ-11 | Controle de agentes e dados | Escopos, transações, autoria, limites e resistência a conteúdo malicioso |
| REQ-12 | Preservação do escopo | Nenhuma capacidade retirada ou tolerância relaxada apenas para encerrar a sequência |
| REQ-13 | Oito disciplinas de instalações integradas ao modelo estrutural | Cobertura por função, modelos físicos e redes, cálculos próprios, documentos e qualificação integrada |
| REQ-14 | Apresentação com identidade própria | README e LEIAME sem nomes ou comparações de softwares de referência, com fontes preservadas na pesquisa |

## 5. Abrangência funcional

O produto precisa cobrir concreto armado, protendido e pré-moldado; fundações e interação solo-estrutura; aço, mistos, tubulares e perfis formados a frio; madeira serrada e produtos engenheirados; alvenaria estrutural; paredes moldadas no local; alumínio com base própria; materiais genéricos para análise sem aprovação normativa implícita.

As análises incluem ações e combinações ELU/ELS, vento, sismo, dinâmica, segunda ordem, não linearidades, etapas construtivas, efeitos diferidos e situações de incêndio no domínio qualificado. Cada capacidade deve distinguir análise, resistência, serviço, detalhamento e emissão.

As instalações incluem hidrossanitária, gás, incêndio, elétrica, fotovoltaica, SPDA, cabeamento estruturado e climatização. O escopo detalhado de água quente, piscinas, tratamento local, sistemas preventivos, geração e equipamentos está em [INSTALACOES.md](docs/INSTALACOES.md). A cobertura inclui recursos avançados documentados e atualizações até a data de pesquisa; não se restringe à edição comercial mais simples.

Os entregáveis incluem modelos, desenhos, armaduras, diagramas elétricos, esquemas verticais, isométricos, mapas de cabos, listas de materiais, memoriais, quantitativos, formatos CAD e intercâmbio BIM. Programação visual, rotinas, API e agentes devem operar pelo núcleo de comandos, com controle de alterações e evidências.

A [matriz de paridade](docs/PARIDADE.md) é a referência detalhada. Ela distingue aplicativo, módulo e versão, bem como funcionalidades pesquisadas e requisitos adicionais do Prumo. A issue 2 completa o inventário por capacidade; a descoberta de uma lacuna obriga a incorporá-la à entrega correspondente e ao controle de lançamento. Recursos novos posteriores ao recorte devem ser identificados como evolução, sem alterar silenciosamente a comparação da versão inicial.

## 6. Jornada principal

1. Criar projeto, escolher unidades, local, normas e configuração de trabalho.
2. Receber referências ou começar pelos eixos e níveis.
3. Montar peças e conexões, acompanhando os modelos físico e analítico.
4. Definir materiais, ações, vínculos e hipóteses ainda ausentes.
5. Revisar o caminho de cargas e corrigir impedimentos localizados.
6. Calcular e conferir demanda, capacidade, serviço e casos governantes.
7. Comparar alternativas de forma rastreável, sem perder requisitos.
8. Detalhar e reverificar as configurações efetivamente executáveis.
9. Revisar tecnicamente o conjunto e resolver pendências.
10. Emitir o pacote de uma revisão única, com autoria e integridade verificáveis.

Cada passo tem consulta por MCP e, quando pertinente, comando de alteração. Operações reversíveis podem ser previamente autorizadas. Emitir uma entrega técnica exige os estados e a atribuição de responsabilidade definidos no produto; um agente não pode fabricar aprovação profissional.

## 7. Regras essenciais de experiência

O encaixe deve mostrar se apenas alinha peças ou cria ligação resistente. Um objeto importado deve indicar se é referência, elemento físico ou elemento com análise validada. Valores sugeridos precisam de origem e estado de confirmação. Ações não podem ser omitidas para facilitar o preenchimento.

O diagnóstico deve indicar o objeto, o problema, sua consequência e opções de correção, sem depender de códigos internos. O usuário poderá localizar o elemento e comparar o antes/depois. Resultados não calculados, não convergidos, obsoletos e fora do domínio devem ter estados distintos.

Metas finais de produto a comprovar: pelo menos 44 profissionais novos no Prumo, em onze famílias de tarefas (três estruturais e oito de instalações), ao menos quatro por família, mediana de até 45 minutos por família, pelo menos 90% de subtarefas e de participantes concluindo o fluxo essencial, média SUS de pelo menos 80 e zero erro crítico observado. A campanha estrutural inicial de doze participantes permanece como marco intermediário; a issue 245 verifica o produto ampliado. O protocolo deve ser fixado antes da coleta. Os detalhes e o desenho da amostra estão em [UX](docs/UX.md).

## 8. Regras de confiabilidade

O cálculo recebe um snapshot imutável e devolve resultados, diagnóstico e identificação das versões usadas. Mudar geometria, material, carga, vínculo, armadura, rede, equipamento, circuito, vazão, pressão, proteção, ambiente, norma ou motor invalida as evidências dependentes, inclusive entre disciplinas. Não se aceita trocar uma tolerância depois de ver um erro para transformar o caso em aprovado.

Soluções analíticas, conservação, convergência, ensaios autorizados e implementações independentes sustentam a verificação. Comparações com sistemas comerciais exigem acesso legítimo e hipóteses equalizadas. O fornecedor do componente pode ajudar a testar, mas seus próprios resultados não constituem uma segunda origem independente.

O domínio aprovado de cada capacidade precisa estar publicado. Regras não implementadas e dados ausentes impedem emissão conclusiva. O sistema não deve acrescentar molas artificiais para esconder mecanismo, inventar parâmetros geotécnicos nem transformar geometria BIM em prova de resistência.

## 9. Reuso e custo total

Toda issue deve registrar se há componente aberto adequado. O resultado pode ser adoção, adaptação, contribuição ao projeto de origem ou implementação própria justificada. A regra não exige usar uma biblioteca inadequada para cumprir uma meta de reutilização.

O custo inclui integração, adaptação normativa, testes, atualização, distribuição, suporte e dependências. Código com licença incompatível ou qualidade insuficiente não acelera o lançamento. Rodar um componente em outro processo não elimina automaticamente suas obrigações de licença.

Componentes pesquisados são candidatos. A homologação exige versão/commit, trecho utilizado, licença e avisos, dependências transitivas, resultado de testes próprios, limites conhecidos e decisão registrada. Nenhum componente de cálculo recebe autorização geral para todos os métodos por ter passado em um exemplo.

## 10. Aceite da sequência

As 250 issues formam uma ordem de implementação, não uma estimativa de dias nem promessa de uma única alteração por issue. Uma entrega pode exigir mais de uma PR, mas continua aberta até que todos os critérios tenham sido cumpridos. Mudanças necessárias para corrigir evidências afetam a conclusão da própria issue.

Os marcos intermediários produzem versões de desenvolvimento. A versão comercial só sai após as campanhas independentes, pilotos, usabilidade, segurança, licenças, instalação e auditoria final. Os requisitos de [lançamento](docs/LANCAMENTO.md) integram a issue 250 e não podem ser dispensados pelo implementador.

## 11. Contrato da ampliação

A revisão 2.0 preserva as entregas estruturais e amplia a fila por instrução do proprietário. As primeiras issues admitem disciplinas, ambientes e portas tipadas desde o modelo de domínio. Os motores de instalações são implementados posteriormente, sem exigir cálculos futuros para encerrar as fundações do software.

As issues 111 a 120 qualificam o incremento estrutural e preparam a expansão. A antiga etapa 120 não publica a versão comercial. As issues 121 a 238 implementam instalações e integração; 239 a 250 qualificam o produto ampliado e encerram o lançamento. Evidências estruturais válidas podem ser reutilizadas com análise de impacto; qualquer dependência alterada exige nova execução.

A interface distingue representar um equipamento, calcular seu comportamento, verificar regras aplicáveis e emitir documentos. Subestações, equipamentos e sistemas preventivos com suporte apenas de modelagem na fonte recebem esse limite explícito. Funções recentes documentadas, como carga térmica, entram na implementação e em testes próprios. Uma lacuna de fonte exige investigação e resolução do escopo, sem alegação de conformidade por inferência.

O README e o LEIAME descrevem o Prumo por suas próprias capacidades. Marcas de referência, versões, fontes e comparações pertencem à documentação técnica de pesquisa.
