# Revisão do planejamento

**Objeto examinado:** documentação e sequência de 120 issues do Prumo, em 14/11/2026. **Natureza da revisão:** pesquisa e revisão independente por agentes especializados, seguidas de conferência do coordenador e validação automática de integridade. Não houve validação de software estrutural nem assinatura ou aceite técnico de profissional habilitado nesta sessão.

## Frentes de revisão

1. Cobertura funcional dos manuais e catálogos oficiais Eberick, CYPECAD/CYPE 3D e TQS, com distinção de módulos e versões.
2. Normas, verificação numérica, independência de referências e critérios de confiabilidade.
3. Arquitetura BIM, MCP, segurança, reutilização de componentes e experiência de uso.

As revisões buscaram omissões capazes de produzir um programa aparentemente completo que ainda não pudesse atender ao objetivo. As recomendações foram incorporadas antes do registro definitivo da fila.

## Correções incorporadas

| Achado | Tratamento no plano |
|---|---|
| Manuais funcionais citam normas antigas | Cadastro separa metadados atuais, edição em uso, texto licenciado e regra implementada |
| Pilares-parede não equivalem ao sistema completo de paredes moldadas no local | NBR 16055, modelagem, detalhamento e campanhas incluídos em 49, 53, 60 e 113 |
| Tirantes não podem trabalhar artificialmente à compressão | Tipos unilaterais, ativação/desativação e inversão de carregamento em 22, 26 e 77 |
| Verificação isolada não exige edifício fictício | Entradas de esforços externos e limites da conclusão em 42, 62, 65 e 72 |
| Pré-moldados exigem fases e tipologias próprias | Transporte, içamento, insertos, painéis, pi/TT e telhas explicitados em 57 e 58 |
| Perfis de alma aligeirada precisam de método específico | Geometria, modos locais e base complementar em 71 e 72 |
| IFC não conserva todo o cálculo | Pacote nativo, modelo físico/analítico e relatório de perdas em 4, 5 e 104 |
| Modelagem inicial exigia solver ainda futuro | Aceite inicial restrito a representação e contratos; integração numérica obrigatória em 22, 25, 40 e 112 |
| Grelhas eram dimensionadas antes de ter gerador | Base numérica antecipada à 22; 95 aprofunda comparação e escolha de idealização |
| Geradores sísmico/térmico antecedem verificações resistentes por material | Estados intermediários e bloqueio de emissão explicitados em 35, 38 e 39 |
| Reservatórios precedem o módulo geotécnico | Pressões e vínculos prescritos na fase estrutural, com qualificação integrada posterior em 70 e 114 |
| Campanhas de módulo antecedem a emissão multiformato | Relatórios técnicos nativos nas campanhas iniciais; entrega final nas issues 101 a 106 |
| MCP completo antecedia documentos futuros | Paridade das capacidades disponíveis na 99, extensão obrigatória em cada nova função e conferência completa na 112 |
| Segurança e licença poderiam começar depois da primeira integração | Triagem, versões, stdio local e limites mínimos de acesso exigidos já na issue 1 |
| Código aberto poderia ser confundido com componente homologado | Quinze fichas documentais, pendências explícitas e qualificação por método, versão e distribuição |
| Exportações móveis e analíticas poderiam ser apenas nomes | Reabertura, escala, orientação e comparação do destino exigidas em 104 e 108 |
| Usabilidade só media subtarefas | Acrescentada taxa de participantes que concluem o fluxo essencial, além de tempo, SUS e erros críticos |
| Incidente numérico poderia afetar entregas já emitidas | Rastreio de versões/regras/projetos e comunicação aos responsáveis na issue 119 |

## Conferência de integridade

O verificador `scripts/validate_plan.py` confere 120 identificadores contíguos, predecessores, fases, referências, capacidades, requisitos e textos gerados. Depois da criação das issues, o registro `planning/github-issues.json` conserva números, URLs, títulos e hashes dos corpos. A conferência de publicação compara os textos recuperados do GitHub aos arquivos locais.

Essas verificações demonstram consistência e publicação do plano. Não demonstram correção de fórmulas, segurança de estruturas ou qualidade de um produto que ainda será implementado. Os testes numéricos e revisões profissionais necessários estão definidos nas próprias issues e em [VALIDACAO.md](VALIDACAO.md).
