# Revisão do planejamento do Prumo

**Versão 2.0, em 14/11/2026.** Objeto: ampliação do planejamento para estruturas e oito disciplinas de instalações, com 250 issues, 72 famílias obrigatórias e 14 requisitos permanentes. A revisão foi executada por agentes especializados, seguida da conferência do coordenador. Não houve implementação ou validação de cálculos nem aceite técnico profissional nesta sessão.

## Frentes examinadas

| Frente | Verificações realizadas |
|---|---|
| Cobertura das instalações | Catálogos e manuais oficiais, recursos avançados, edições e atualizações; água, esgoto, chuva, gás, incêndio, elétrica, fotovoltaica, SPDA, comunicação e climatização |
| Normas e confiabilidade | Classes de evidência, transições, domínio dos métodos, independência de referências, atribuições profissionais e condições de qualificação |
| Arquitetura e sequência | Modelo comum, portas e redes, catálogos, contratos UI/MCP, dependências, integração, emissão e liberação final |
| Apresentação e controles | README/LEIAME com identidade própria, geração dos textos, integridade da fila, cobertura e registro de publicação |

As frentes fizeram revisão cruzada de conteúdo preparado por outro agente. Os pareceres não identificaram problema crítico ou grave remanescente no recorte examinado após as correções. As decisões e seus limites estão registrados em [review.json](../planning/review.json).

## Correções incorporadas

| Achado | Tratamento |
|---|---|
| O lançamento antigo antecederia instalações | Etapa 120 transformada em marco estrutural; liberação completa transferida para 250 |
| Base poderia admitir apenas vigas e pilares | Contratos iniciais ampliados para disciplinas, ambientes, portas, redes e unidades específicas, sem exigir motores futuros |
| Fotovoltaica e recursos recentes poderiam ser omitidos | Disciplina própria, baterias, inversores, CC/CA, geração e cálculo térmico incluídos explicitamente |
| Diferenças entre documentos antigos e novos | Registro por capacidade e edição, com investigação obrigatória dos conflitos antes do aceite |
| Subestação e preventivos poderiam receber aprovação por simples representação | Modelagem, cálculo, verificação e emissão separados no domínio e nas evidências |
| Normas antigas de água quente e tratamento poderiam ser combinadas | Registro explícito de transições e confirmação do texto aplicável antes de implementar |
| Série normativa de climatização poderia receber ano indiscriminado | Situação conferida por parte; suspensão histórica documentada e vigência atual pendente de verificação |
| SPDA pediria telecom ainda futura na 207 | Portas/redes genéricas disponíveis nessa etapa; integração completa de sinais exigida após os módulos de telecom |
| Climatização exigiria coordenação global antes da 227 | Testes locais e interfaces nas 225/226; transferência e coordenação geral nas 227/228/238 |
| Marcos 111 a 118 ainda citavam binário final | Evidências delimitadas ao incremento estrutural e reconferidas no candidato ampliado |
| Usabilidade e pilotos só cobririam estruturas | Campanhas finais com todas as oito disciplinas e onze famílias de tarefas, conforme UX e VALIDACAO |
| Biblioteca existente poderia parecer componente homologado | Treze candidatos adicionais em triagem, com licença, domínio e testes próprios antes da adoção |
| Lista normativa em texto poderia ser renderizada letra a letra | Integração normaliza listas; verificador rejeita tipos incorretos |
| Nome de referência poderia reaparecer na apresentação | Verificação de README e LEIAME, incluindo alias, separadores e URLs codificadas |

## Verificação de integridade

O comando `python3 scripts/validate_plan.py` verifica a sequência, predecessores, fases, identidades, fontes, requisitos, capacidades obrigatórias, textos gerados, links locais e manifesto de publicação. A etapa de preparação usa `--allow-unpublished` apenas enquanto os novos registros ainda não foram publicados; a entrega final exige a verificação completa.

Foram executadas doze mutações negativas em cópias temporárias, cobrindo redução da fila, ausência/opcionalidade de capacidade, perda dos requisitos novos, lançamento incorreto, marcas na apresentação, tipo inválido, aceite vazio e hash divergente. Todas foram rejeitadas pelos diagnósticos esperados. Um caso positivo de fonte com título sem descrição também passou. [Registro dos ensaios](../planning/reviews/2.0-validator.json).

O manifesto sintético usado para testar o verificador ficou restrito à cópia temporária. A confirmação da publicação real usa leitura independente das issues do GitHub, com comparação de número, título e corpo completo. O [registro publicado](../planning/github-issues.json) conserva os hashes e a situação efetivamente conferida.

## Histórico e limite desta revisão

A versão 1.0 continha 120 entregas estruturais. Seu registro permanece em [revisão 1.0](../planning/reviews/1.0.json) e no histórico Git. A versão 2.0 preserva os requisitos estruturais e amplia o lançamento, sem marcar capacidades como implementadas por constarem no plano.

A consistência documental não prova a correção dos cálculos. As campanhas numéricas, os ensaios de integração, os estudos com usuários e os aceites por profissionais habilitados continuam como condições de conclusão das issues e da liberação comercial.
