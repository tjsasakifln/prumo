# Condições de lançamento do Prumo

**Esta é a definição de pronto do produto, aplicada pela issue 250.** O repositório começa em estado de planejamento. Os critérios abaixo ainda precisam ser demonstrados durante a implementação.

## 1. Cobertura e sequência

As issues 1 a 249 devem estar implementadas e integradas, com evidências atuais. A matriz de capacidades liga cada requisito a código, testes, verificação, limitações e documentos. Nenhum item obrigatório pode ser removido da comparação para encerrar o produto.

O inventário de paridade é refinado na issue 2 e revisitado antes da publicação. Descobertas pertinentes dentro do recorte contratado entram na entrega correspondente. Recursos posteriores ao recorte são evoluções registradas, sem alegações retroativas de equivalência.

## 2. Engenharia

- Todas as normas necessárias têm edição, emendas, acesso autorizado e interpretação registrados.
- Cada método tem domínio, casos independentes e tolerâncias pré-registradas.
- Há evidência de equilíbrio, convergência, estados limites, fronteiras e falha controlada.
- Os casos obrigatórios têm zero falsos aprovados; nenhuma divergência crítica ou grave fica sem resolução.
- Todas as capacidades exigidas de concreto, fundações, aço, madeira, alvenaria, paredes, alumínio e todas as oito disciplinas de instalações possuem comprovação correspondente.
- Revisores profissionais habilitados, independentes da implementação das regras examinadas, registram o aceite técnico. Essa revisão não ocorreu na sessão de criação do repositório.

## 3. Projeto e documentos

Os resultados de estruturas e instalações pertencem ao snapshot calculado. Modelos, desenhos, diagramas, esquemas verticais, mapas de cabos, listas e memoriais da entrega são coerentes entre si, incluindo dimensões, unidades, quantidades e revisão. Os arquivos exportados são reabertos e conferidos. IFC, CAD, BCF e IDS são ensaiados com implementações independentes, conforme os perfis prometidos.

A emissão técnica registra autoria, decisão de revisão e integridade dos arquivos. Resultados desatualizados, capacidades não verificadas e pendências críticas impedem emissão conclusiva. Um agente não pode representar a assinatura de um responsável humano.

## 4. Produto e experiência

O estudo final precisa atender às metas pré-registradas em UX.md e na issue 245. A amostra inclui pelo menos 44 profissionais sem experiência no Prumo, onze famílias de tarefas e pelo menos quatro participantes por família. A campanha estrutural da issue 117 é um marco intermediário. Medidas não podem mudar depois de conhecidos os resultados.

Metas: mediana de até 45 minutos por família, pelo menos 90% das subtarefas e dos participantes concluindo o fluxo essencial, nenhuma família abaixo de 75% de conclusão, média SUS de pelo menos 80 e zero erro crítico observado. São critérios próprios de produto para tarefas delimitadas, não limites prescritos por normas de engenharia.

Operação por uma mão e teclado, recuperação de erros, compreensão de estados e acessibilidade aplicável são testadas. Um software que exige intervenção constante de quem o desenvolveu não atende ao fluxo intuitivo solicitado.

## 5. Agentes e segurança

Interface e MCP exercitam o mesmo serviço de domínio. Controle de acesso, idempotência, transações, revisão base e isolamento entre projetos funcionam no binário candidato. Ferramentas não permitem que texto de arquivo conceda acesso, mude política, exfiltre dados ou aprove resultados.

Parsers, extensões, empacotamento e atualizações passam por auditoria. Não existem vulnerabilidades críticas exploráveis ou graves sem mitigação verificada no escopo de distribuição. Os testes abrangem a configuração realmente entregue, incluindo transporte remoto se oferecido.

## 6. Distribuição e condições comerciais

Licenças e avisos de terceiros estão conciliados com o inventário efetivamente distribuído. Direitos de uso, fontes de catálogo, conversores CAD e documentação normativa foram resolvidos. O catálogo comercial descreve capacidades e limites comprovados, sem anunciar certificação inexistente.

Instalação, atualização, reversão, backup, recuperação e suporte foram testados em máquinas limpas. Se o modelo exigir cobrança, o ciclo implementado inclui falha, renovação, cancelamento, estorno, suspensão e reativação. O cliente preserva meios previstos de recuperar seus próprios arquivos.

Existe procedimento para incidentes numéricos: identificar versões, regras, projetos e entregas afetados; impedir nova emissão inadequada; informar responsáveis por canal autorizado; emitir correção; preservar histórico e indicar o que precisa ser reanalisado.

## 7. Artefato final

O registro de lançamento contém versão, commit, SHA-256 do binário, dependências, versões normativas, matriz de capacidades, relatórios de ensaio, revisores e aceites. O arquivo distribuído deve ter o mesmo hash do candidato aprovado.

Qualquer alteração posterior exige análise de impacto. Evidências invalidadas são reexecutadas. Fechar issues não substitui essas comprovações. Se uma condição permanecer pendente, a issue 250 permanece aberta e a versão não é anunciada como lançamento completo.

## 8. Ampliação e preservação do histórico

O marco 120 conclui a qualificação do incremento estrutural e prepara as instalações, sem publicar a versão comercial. A liberação na 250 exige reconciliação de todas as 249 entregas anteriores e das duas matrizes de escopo. Nenhuma capacidade de instalações permanece pendente por ter sido descrita como evolução futura na versão anterior do plano.

As evidências estruturais anteriores são reaproveitáveis apenas quando a análise de impacto provar que continuam válidas no candidato final. Alterações em modelo, unidades, catálogos, regras, adaptadores, documentos ou dependências exigem os ensaios afetados. A versão comercial deve apresentar capacidades e limites por disciplina, com documentação coerente e nenhuma modelagem divulgada como cálculo que não existe.
