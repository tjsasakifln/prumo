# Critério comum de conclusão de uma issue

Uma issue representa uma entrega demonstrável. A descrição específica complementa estas regras e não as substitui. A conclusão da issue não declara aprovação do produto inteiro nem de uma capacidade futura.

## Antes de implementar

Ler as fontes pertinentes, conferir a versão do repositório e validar a conclusão das anteriores. Resolver o que for decisivo com as fontes autorizadas antes de perguntar ao proprietário. Registrar hipóteses, dependências e limites.

Examinar componentes abertos adequados e registrar uma decisão de reuso. Para qualquer componente incorporado, verificar procedência, licença por trecho, dependências, versão, manutenção e testes próprios. Uma alternativa descartada deve ter motivo concreto.

## Entrega funcional

O comportamento solicitado precisa existir e ser utilizável. Não encerrar com telas vazias, respostas fixas, retornos simulados ou rascunhos apresentados como função concluída. Fixtures são permitidas nos testes quando identificadas e independentes de resultados reais.

Novas funções pertinentes ampliam os comandos comuns e o MCP, sem criar lógica de engenharia paralela na interface. Persistência, unidades, identidades, erros, autoria, concorrência e invalidação devem permanecer coerentes.

Antes da implementação dos motores posteriores, uma issue pode concluir o seu contrato e representação com casos próprios. Não pode declarar aprovada a análise que ainda não existe. As interfaces de integração futura e seus testes obrigatórios precisam estar indicados.

## Evidências

- Testes específicos executados no código integrado, com entradas, resultados e identificação da versão.
- Contraprova independente para regras numéricas e normativas.
- Inspeção do arquivo exportado para desenhos, documentos e modelos de intercâmbio.
- Cliente real e testes de autorização para MCP.
- Execução das tarefas e observação de erros para experiência de uso.
- Revisão independente proporcional ao risco e registro das correções.

Fórmulas e regras normativas exigem fonte, edição, item, domínio, interpretação e testes. Alterar tolerância para fazer passar um erro é proibido. Dois wrappers do mesmo motor não formam duas referências independentes.

## Registro de conclusão

Ligar na issue as PRs, commits, evidências, limitações e decisões de reuso. Atualizar documentação, exemplos, estado da capacidade e matriz de rastreabilidade. O registro de estado do plano e os textos devem permanecer sincronizados.

Uma mudança material no planejamento exige revisar o impacto nas dependências e na issue 250. As especificações locais são geradas por `scripts/render_plan.py`; atualize `planning/backlog.json`, regenere e publique o texto correspondente no GitHub. O manifesto de publicação deve registrar o hash do corpo efetivamente lido do GitHub, sem inventar conferência.

Integre a alteração conforme as verificações exigidas na branch principal. Feche a issue somente após todos os aceites. Se algo obrigatório continuar pendente, mantenha a issue aberta e explique o impedimento real.

A revisão ampliada preserva as 119 primeiras entregas estruturais, adapta a 120 e acrescenta instalações até a 250. Os marcos de qualificação estrutural não liberam funções futuras de instalações; a evidência final deve cobrir todas as disciplinas no mesmo candidato de distribuição.
