# Experiência de uso e critérios de aprendizagem

**Estado:** metas propostas para o produto, ainda não medidas. **Data:** 14/11/2026. Os ensaios devem ocorrer com pessoas reais e evidências registradas. Revisão por agentes ajuda a encontrar falhas, mas não substitui o ensaio de uso com engenheiros.

## 1. Objetivo

Permitir que um engenheiro produza um modelo correto com ações previsíveis, visualização clara das hipóteses e correção orientada dos problemas. A montagem por peças e encaixes reduz o esforço de operar o programa. Ela preserva as decisões de engenharia e não substitui conhecimento técnico.

O fluxo principal será: criar projeto; estabelecer níveis e eixos; posicionar elementos; declarar usos e ações; revisar hipóteses; calcular; corrigir diagnósticos; detalhar; conferir e emitir. Uma ação principal deve estar clara em cada etapa. O usuário pode voltar, comparar alternativas e desfazer alterações reversíveis.

## 2. Regras da interface

| Situação | Comportamento exigido |
| --- | --- |
| Inserir elemento | Prévia, dimensão editável, orientação e relação com níveis visíveis |
| Encaixar peças | Mostrar hipótese mecânica e efeito na conectividade antes de aplicar |
| Repetir pavimento | Informar o que será copiado, o que permanece relacionado e quais cargas exigem revisão |
| Introduzir valor | Unidade explícita, conversão compreensível e tratamento de formato decimal sem ambiguidade |
| Aplicar carga automática | Origem, categoria, valor e região visíveis; hipótese ausente solicitada |
| Diagnosticar erro | Elemento, problema, consequência e caminho de correção, com zoom ou seleção direta |
| Consultar resultado | Grandeza, unidade, combinação, deformada/escala e revisão sempre identificadas |
| Alterar modelo calculado | Indicar resultados afetados e estado obsoleto de forma imediata |
| Trabalhar por agente | Mostrar mudanças, estado e autor, preservando os mesmos controles da interface |
| Emitir | Apresentar conjunto, versão, verificações, responsável e pendências impeditivas |

Planta, corte e 3D devem permanecer sincronizados. Oferecer edição por tabela, busca de comandos em português, seleção por propriedades, isolamento de elementos e comparação entre versões. Informações avançadas podem estar recolhidas, mas devem ser acessíveis e jamais omitidas da verificação necessária.

Nenhum sinal de aprovação depende exclusivamente de cor. Um resultado pode estar completo e ainda conter reprovação técnica; a interface distingue término do processamento, validade dos dados, atendimento das verificações e emissão autorizada.

## 3. Operação com uma mão e teclado

Todas as tarefas essenciais devem possuir caminho por teclado e caminho por apontador, sem combinação obrigatória de duas mãos. Oferecer seleção sem arrastar, atalhos reconfiguráveis, modo de tecla única quando pertinente, foco visível e ordem de navegação coerente.

Usar como referência técnica os critérios de teclado, foco e alternativa a arrastar das [WCAG 2.2](https://www.w3.org/TR/WCAG22/). Para a aplicação desktop, considerar a orientação informativa [WCAG2ICT](https://www.w3.org/TR/wcag2ict/), sem apresentar essa orientação como certificação já obtida.

Pan, zoom, rotação, seleção múltipla, cancelamento e edição numérica não podem depender somente de acordes complexos de teclas e botões. O canvas precisa ter alternativa de consulta e seleção por lista/tabela. Diálogos mantêm foco controlado, rótulos claros e retorno ao ponto de origem.

Incluir validação com participantes que representem operação por uma mão e por teclado, mediante consentimento. Simular apenas internamente não basta para afirmar acessibilidade comprovada.

## 4. Ensaio de aprendizagem para lançamento

Protocolo deve ser aprovado e fixado antes da execução. Mudanças em metas ou critérios depois de observar o resultado exigem novo protocolo identificado, sem reaproveitar seletivamente resultados favoráveis.

### Participantes e tarefas

Pelo menos **12 engenheiros sem experiência prévia no Prumo**, com formação compatível com a tipologia atribuída. Usar **três tipologias**: casa de concreto, galpão de aço e cobertura de madeira. Distribuir pelo menos quatro participantes por tipologia e registrar familiaridade anterior com outros programas e experiência de projeto.

Cada tarefa é um projeto de referência de escopo reduzido, mas com decisões reais: elementos, vínculos, ações, ao menos um erro introduzido, cálculo, consulta de verificação e produção de prévia documental. Não exigir detalhamento de um empreendimento completo em 45 minutos. A amostra e os limites são declarados no relatório, sem generalizar a todas as obras.

Introdução padronizada de no máximo 10 minutos. Material de ajuda que acompanha o produto pode ser consultado; auxílio do moderador é registrado e reduz a classificação de conclusão independente. Não permitir que autores da interface conduzam os participantes até o caminho correto.

### Metas de aceitação

| Métrica | Meta proposta | Como verificar |
| --- | --- | --- |
| Tempo de primeira tarefa | Mediana de até 45 minutos em cada tipologia | Cronometragem automática, protocolo e registros consentidos |
| Tarefas concluídas corretamente | Pelo menos 90% do conjunto de subtarefas essenciais, sem auxílio do moderador | Rubrica prévia e avaliação dos arquivos/resultados |
| Participantes que concluem o fluxo | Pelo menos 90% da amostra concluem o fluxo essencial completo, sem auxílio do moderador | Todos os participantes que iniciam entram no denominador, incluindo desistências; publicar também a distribuição por tipologia |
| Falhas críticas | Zero falha crítica de compreensão ou operação que resulte em aprovação ou emissão indevida | Revisão independente de hipóteses, resultados e decisões |
| Percepção de usabilidade | SUS médio de pelo menos 80/100 na amostra total, com distribuição por tipologia publicada | Questionário aplicado de modo padronizado; registrar todos os participantes |
| Uma mão e teclado | 100% das tarefas essenciais possuem caminho funcional; nenhum bloqueio encontrado permanece aberto | Teste específico e observação consentida |
| Recuperação de erro | Identificar e corrigir ao menos apoio ausente e carga atribuída incorretamente nas tarefas previstas | Comandos e intervenções registrados |
| Reconhecimento de hipótese | Participante identifica a hipótese resistente do encaixe e a consequência da alteração | Pergunta prática vinculada à tarefa |

As metas de subtarefas e de participantes são cumulativas. Com a amostra mínima de 12 participantes, pelo menos 11 precisam concluir o fluxo essencial completo. O protocolo define previamente quais etapas compõem esse fluxo e a rubrica de conclusão correta; não basta completar etapas isoladas para classificar uma pessoa como concluinte.

O SUS, System Usability Scale, é uma medida de percepção de uso, como descreve seu autor em [SUS: A Retrospective, John Brooke, 2013](https://iihm.imag.fr/coutrix/ens/M2-MoSIG-HiL/1stAssignment/UsabilityQuestionnaire/JUS_Brooke_February_2013.pdf). Não mede correção estrutural. Os critérios técnicos seguem [VALIDACAO.md](VALIDACAO.md).

### Medição independente

Avaliar os resultados anonimizados e em ordem aleatória por revisor que não conheça a condição experimental, a hipótese de melhoria ou a identidade do participante. Quando comparar modo manual com modo assistido por agente, manter as mesmas tarefas e critérios e equilibrar a ordem para reduzir efeito de aprendizagem. O participante vê a interface e conhece se usa um agente; não declarar cegamento total que não ocorreu.

Publicar tentativas, desistências, tempos, erros e assistência recebida, incluindo resultados desfavoráveis. Participantes não podem ser substituídos depois da medição apenas porque tiveram dificuldade. O relatório precisa informar limitações da amostra.

## 5. Resposta da interface

Alvos propostos em hardware e modelos de referência previamente fixados:

- Feedback de ação local com p95 de até 100 ms.
- Seleção e navegação 3D com pelo menos 30 quadros por segundo no cenário declarado.
- Reconhecimento visual de solicitação de cancelamento em até 1 segundo, com término do worker informado separadamente.
- Nenhuma perda de edição confirmada após reinício normal; recuperação após falha conforme contrato de persistência.
- Zero resultado obsoleto apresentado como vigente na suíte de alterações.

Tempos de cálculo são medidos separadamente e informados por tipo de análise. Não prometer desempenho usando apenas projeto vazio ou demonstração reduzida sem identificá-la.

## 6. Evolução por issue

Cada issue que altera interação inclui tarefa de uso, diagnóstico de falha e verificação por interface e MCP quando aplicável. A issue de validação final executa o protocolo completo. Um problema crítico de usabilidade que induza uma decisão técnica incorreta bloqueia a conclusão da capacidade afetada até correção e novo ensaio.

As metas deste documento serão vinculadas a evidências reais antes de qualquer afirmação de aprendizagem ou facilidade de uso no lançamento.
