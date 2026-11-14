# Prumo: requisitos de produto

**Versão do planejamento:** 1.0. **Data:** 14/11/2026. **Estado:** especificação, sem software de cálculo implementado.

## 1. Objetivo

Criar um sistema brasileiro profissional de cálculo estrutural com a cobertura funcional pertinente documentada nos ecossistemas Eberick, CYPECAD e TQS, unindo concepção, análise, dimensionamento, detalhamento e documentação. O uso deve se aproximar da montagem de peças com conexões claras, mantendo as hipóteses de engenharia compreensíveis e verificáveis.

O usuário poderá começar manualmente, pedir trabalho a um agente, revisar a alteração, continuar pela interface e emitir uma entrega coerente. As duas formas de operação acessarão o mesmo modelo e os mesmos comandos. O motor de cálculo será determinístico, independente de modelos de linguagem.

O pedido desta sessão é criar o repositório e registrar o trabalho necessário. A sequência tem **120 issues de implementação**, com conclusão numérica estrita. Esta especificação não declara que as funcionalidades, as metas de desempenho ou a confiabilidade já foram atingidas.

## 2. Usuários e tarefas

O público inicial é formado por engenheiros estruturais, projetistas sob responsabilidade técnica, escritórios, revisores e equipes que trabalham com concreto, aço, madeira e sistemas complementares no Brasil.

As tarefas principais são conceber uma estrutura, receber arquitetura, lançar peças e ações, compreender o caminho de cargas, calcular, localizar impedimentos, comparar alternativas, detalhar, revisar e emitir. O sistema também deve permitir verificar elementos isolados com esforços externos rastreáveis, sem exigir um edifício fictício.

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

## 5. Abrangência funcional

O produto precisa cobrir concreto armado, protendido e pré-moldado; fundações e interação solo-estrutura; aço, mistos, tubulares e perfis formados a frio; madeira serrada e produtos engenheirados; alvenaria estrutural; paredes moldadas no local; alumínio com base própria; materiais genéricos para análise sem aprovação normativa implícita.

As análises incluem ações e combinações ELU/ELS, vento, sismo, dinâmica, segunda ordem, não linearidades, etapas construtivas, efeitos diferidos e situações de incêndio no domínio qualificado. Cada capacidade deve distinguir análise, resistência, serviço, detalhamento e emissão.

Os entregáveis incluem modelos, desenhos, armaduras, listas de materiais, memoriais, quantitativos, formatos CAD e intercâmbio BIM. Programação visual, rotinas, API e agentes devem operar pelo núcleo de comandos, com controle de alterações e evidências.

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

Metas de produto a comprovar: pelo menos doze engenheiros novos no Prumo, três tipologias, mediana de até 45 minutos por tipologia, pelo menos 90% de subtarefas e de participantes concluindo o fluxo essencial, média SUS de pelo menos 80 e zero erro crítico observado. O protocolo deve ser fixado antes da coleta. Os detalhes e o desenho da amostra estão em [UX](docs/UX.md).

## 8. Regras de confiabilidade

O cálculo recebe um snapshot imutável e devolve resultados, diagnóstico e identificação das versões usadas. Mudar geometria, material, carga, vínculo, armadura, norma ou motor invalida as evidências dependentes. Não se aceita trocar uma tolerância depois de ver um erro para transformar o caso em aprovado.

Soluções analíticas, conservação, convergência, ensaios autorizados e implementações independentes sustentam a verificação. Comparações com sistemas comerciais exigem acesso legítimo e hipóteses equalizadas. O fornecedor do componente pode ajudar a testar, mas seus próprios resultados não constituem uma segunda origem independente.

O domínio aprovado de cada capacidade precisa estar publicado. Regras não implementadas e dados ausentes impedem emissão conclusiva. O sistema não deve acrescentar molas artificiais para esconder mecanismo, inventar parâmetros geotécnicos nem transformar geometria BIM em prova de resistência.

## 9. Reuso e custo total

Toda issue deve registrar se há componente aberto adequado. O resultado pode ser adoção, adaptação, contribuição ao projeto de origem ou implementação própria justificada. A regra não exige usar uma biblioteca inadequada para cumprir uma meta de reutilização.

O custo inclui integração, adaptação normativa, testes, atualização, distribuição, suporte e dependências. Código com licença incompatível ou qualidade insuficiente não acelera o lançamento. Rodar um componente em outro processo não elimina automaticamente suas obrigações de licença.

Componentes pesquisados são candidatos. A homologação exige versão/commit, trecho utilizado, licença e avisos, dependências transitivas, resultado de testes próprios, limites conhecidos e decisão registrada. Nenhum componente de cálculo recebe autorização geral para todos os métodos por ter passado em um exemplo.

## 10. Aceite da sequência

As 120 issues formam uma ordem de implementação, não uma estimativa de dias nem promessa de uma única alteração por issue. Uma entrega pode exigir mais de uma PR, mas continua aberta até que todos os critérios tenham sido cumpridos. Mudanças necessárias para corrigir evidências afetam a conclusão da própria issue.

Os marcos intermediários produzem versões de desenvolvimento. A versão comercial só sai após as campanhas independentes, pilotos, usabilidade, segurança, licenças, instalação e auditoria final. Os requisitos de [lançamento](docs/LANCAMENTO.md) integram a issue 120 e não podem ser dispensados pelo implementador.
