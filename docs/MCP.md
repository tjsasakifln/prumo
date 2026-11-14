# MCP e operação do Prumo por agentes

**Estado:** contrato proposto, sem servidor implementado nesta etapa de planejamento. **Data da referência:** 14/11/2026. A issue #1 deverá entregar uma operação real no projeto BIM, acessível pela interface e pelo MCP, com persistência e consulta. A issue #3 confirma decisões de arquitetura e compatibilidade aplicáveis.

## 1. Princípios

O servidor MCP próprio será implementado em Rust. O MCP é um adaptador para os mesmos comandos Rust usados pelo usuário, conforme [RUST.md](RUST.md). O agente não altera diretamente banco, arquivos de resultados ou parâmetros internos do solver. Modelagem, cálculo, verificação e documentação são funções determinísticas, com entradas e saídas rastreáveis.

O agente pode propor alternativas, montar um modelo, consultar dados, solicitar cálculos, explicar diagnósticos e preparar documentos dentro da autorização recebida. O cálculo não é executado por geração de texto. Uma explicação deve apontar o resultado, a hipótese, a verificação e a versão correspondentes.

Autorização de edição não implica autorização de transmissão a qualquer provedor. Leitura, edição, análise, exportação e emissão são capacidades distintas e restritas ao projeto. Operações reversíveis previamente autorizadas podem continuar sem confirmação repetitiva; a autorização e seu limite permanecem auditáveis.

## 2. Revisões de protocolo e transportes

A referência pública atual consultada é **MCP 2026-07-28**. Ela usa versão e capacidades por requisição e exige `server/discover`. O fluxo de inicialização de **2025-11-25** pertence ao adaptador legado. A implementação fixa versões suportadas, registra as respectivas dependências e testa cada caminho; não deve combinar regras incompatíveis dos dois perfis. [Versionamento oficial](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning.md).

| Perfil | Comportamento que a implementação deve respeitar |
| --- | --- |
| 2026-07-28 | Descoberta, metadados e capacidades por requisição; rejeição explícita de versão não suportada |
| 2025-11-25, quando declarado | Inicialização e negociação legadas, com testes próprios |
| Extensões | Uso apenas quando ambas as partes as suportarem; fallback documentado ou erro explicativo |

**Transporte local inicial:** stdio. Mensagens de protocolo em stdout e logs em stderr; nenhuma mensagem humana misturada ao canal. O processo respeita fechamento de entrada e cancelamento. [Especificação stdio](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio.md).

**HTTP:** somente disponibilizar após implementação e testes específicos. Na revisão moderna o endpoint utiliza POST, sem sessão de protocolo nem endpoint GET de fluxo. Validar origem e autenticação; quando local, restringir a interface de rede autorizada. Requisições devem conter os metadados e cabeçalhos exigidos pela revisão. [Streamable HTTP](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http).

A presença do SDK não basta para afirmar compatibilidade. Testar com versões fixas de pelo menos dois clientes independentes que implementem o perfil anunciado. O [SDK Rust oficial rmcp](https://github.com/modelcontextprotocol/rust-sdk) é o candidato prioritário para o servidor próprio. Fixar versão/commit, features e revisão de protocolo; a declaração de main não qualifica automaticamente uma release. SDKs de outras linguagens podem servir a clientes externos de interoperabilidade. [IfcMCP](https://docs.ifcopenshell.org/ifcmcp.html) permanece candidato externo para funções IFC delimitadas, sem substituir os comandos Rust ou permitir mutações irrestritas. Aplicam-se [REUSO.md](REUSO.md) e [fontes-rust.json](fontes-rust.json).

## 3. Contratos de domínio

Todo payload tem schema versionado. Grandezas incluem unidade, números devem ser finitos, listas têm limites e campos desconhecidos são rejeitados quando não previstos. Textos externos não são instruções de execução.

### Contexto de um comando de mutação

| Campo | Regra |
| --- | --- |
| `project_id` | Identidade do projeto autorizado |
| `base_revision` | Revisão sobre a qual a mudança foi preparada |
| `command_id` | Identidade auditável do comando |
| `idempotency_key` | Identidade de negócio para impedir repetição de efeito |
| `schema_version` | Versão exata do contrato |
| `payload` | Dados tipados para a operação permitida |
| Ator e permissões | Derivados do contexto autenticado; nunca aceitos como autodeclaração do payload |

Chave idempotente é vinculada ao ator, projeto, operação e hash do payload. Reenvio idêntico retorna a execução original; reutilização com conteúdo diferente falha. O identificador JSON-RPC não substitui essa chave.

### Prévia e aplicação

1. A ferramenta de prévia valida o comando e calcula diferenças, objetos afetados, hipóteses, resultados que ficarão obsoletos e permissões necessárias.
2. A resposta fornece `plan_id`, `plan_hash`, `base_revision`, validade e diagnósticos, sem modificar o projeto.
3. Aplicar exige o plano e o hash correspondentes. Se revisão, autorização, conteúdo ou dependência relevante mudou, o plano é recusado e precisa ser recalculado.
4. O serviço executa transação atômica, grava auditoria e retorna nova revisão. Uma autorização anterior válida pode permitir essa aplicação sem nova interação humana.

Exemplo de payload de aplicação, apenas ilustrativo do contrato:

```json
{
  "project_id": "8de96549-6433-4144-8439-7329a16d8069",
  "base_revision": 12,
  "command_id": "8a491d76-5091-4e90-9742-382715c83574",
  "idempotency_key": "c5d31722-dcfe-4c47-b599-cdf88c025512",
  "schema_version": "1.0",
  "plan_id": "7b404a6f-324b-43a9-96e2-c8a38b4b07b2",
  "plan_hash": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
}
```

Os valores do exemplo não autorizam uma operação real. A validação final de schemas e a nomenclatura pública serão versionadas na implementação, preservando as garantias aqui definidas.

### Resposta de negócio

Retornar estado, revisão, identificadores, dados estruturados com unidades, diagnósticos e referência à auditoria. Resultados de cálculo também incluem snapshot, solver, perfil normativo, critério de convergência e identidade da evidência. O adaptador envolve essa resposta no formato da revisão MCP negociada, incluindo os campos exigidos por ela.

Erros de negócio possuem códigos estáveis, tais como `REVISION_CONFLICT`, `IDEMPOTENCY_CONFLICT`, `INVALID_UNIT`, `FORBIDDEN_OPERATION`, `UNSUPPORTED_CAPABILITY`, `INVALID_MODEL`, `ANALYSIS_NOT_CONVERGED`, `STALE_RESULTS` e `ISSUE_BLOCKED`. Eles não devem ser confundidos com os códigos JSON-RPC de protocolo. A resposta identifica elementos e correção possível, sem transformar falha em sucesso textual.

## 4. Catálogo funcional inicial

As ferramentas abaixo são requisitos; somente ferramentas efetivamente implementadas e autorizadas serão anunciadas. Seus nomes finais podem ser ajustados por versão de contrato, com migração explícita.

| Grupo de ferramentas | Entradas essenciais | Saída e restrição |
| --- | --- | --- |
| `prumo.project.create`, `prumo.project.read` | Dados mínimos do projeto ou identidade autorizada | Projeto, revisão e perfil; criação é idempotente |
| `prumo.model.query` | Projeto, revisão, seleção limitada, propriedades | Objetos e unidades; paginação e limites de volume |
| `prumo.model.preview_change` | Contexto, comando permitido e payload | Plano, diferenças, hipóteses e invalidações, sem mutação |
| `prumo.model.apply_change` | Projeto, revisão, plano, hash e chave idempotente | Nova revisão e auditoria; aplica exatamente a prévia |
| `prumo.model.undo` | Revisão e comando reversível | Comando compensatório auditado; não apaga histórico |
| `prumo.model.validate` | Projeto, revisão e conjunto de verificações | Diagnósticos de geometria, topologia, dados e aplicabilidade |
| `prumo.analysis.submit` | Snapshot, perfil de análise e chave idempotente | Identidade durável de job, estado e dados de acompanhamento |
| `prumo.analysis.status`, `prumo.analysis.cancel` | Job autorizado | Estado ou intenção de cancelamento; conclusão exige confirmação do worker |
| `prumo.results.query` | Job, snapshot e grandezas selecionadas | Valores com unidade, estado, versão e origem |
| `prumo.design.check` | Snapshot, resultados compatíveis e regras implementadas | Verificações e limites, com itens não cobertos explicitados |
| `prumo.exchange.preview_import` | Identificador controlado de arquivo e perfil | Mapeamentos, perda de informação e proposta de importação |
| `prumo.exchange.export` | Revisão, formato e perfil declarado | Arquivo em destino autorizado e relatório de intercâmbio |
| `prumo.documents.preview` | Conjunto coerente de modelo, resultados e template | Prévia identificada, sem declaração de emissão |
| `prumo.documents.issue` | Conjunto exato, revisão, aprovação válida e responsável | Emissão auditada ou bloqueio com motivo |

Não há ferramenta genérica para shell, execução Python arbitrária, SQL, chamada de função por caminho livre ou gravação em caminho escolhido pelo modelo. Arquivos são identificadores obtidos por seleção autorizada e resolvidos pelo serviço. Um conector técnico de reuso pode operar internamente, mas não contorna o contrato de domínio.

## 5. Jobs, reconexão e resultados

Jobs persistem estado antes de responder à submissão. Devem registrar snapshot, limites, motor, parâmetros e estado terminal. O mesmo envio repetido não cria cálculo duplicado. Progresso informa estágio e quantidade mensurável quando disponível, sem percentuais fictícios.

A extensão oficial **Tasks** pode carregar acompanhamento durável se suportada por ambas as partes. O fallback permanece no contrato de aplicação: submissão retorna `job_id` e consulta usa esse identificador. [Extensão Tasks](https://modelcontextprotocol.io/extensions/tasks/overview).

Perda de conexão não significa conclusão. Cancelamento pode ser cooperativo e precisa ser confirmado; resultado intermediário nunca assume status aprovado. Reinício do worker exige recuperação ou falha explícita. Após uma alteração no projeto, um job pode terminar para seu snapshot antigo, mas a interface e o MCP devem indicar a obsolescência.

Consultas de resultado retornam recortes e referências a arquivos, evitando exposição desnecessária de todo o projeto. Uma explicação do agente deve citar identidades reais de elementos, casos, combinações e verificações. Não inferir aprovação pela ausência de mensagens de erro em texto.

## 6. Segurança e autorizações

Perfis mínimos: consulta; edição; cálculo; preparação de documentos; exportação; emissão técnica. A autorização verifica o projeto e a operação em cada chamada, mesmo quando a ferramenta está visível ao cliente. Recursos, logs e caches não podem misturar projetos ou organizações.

Para HTTP, implementar os requisitos aplicáveis de autorização, audiência do token, armazenamento de credenciais e proteção do fluxo. Token recebido do cliente não deve ser repassado como credencial de outro serviço. [Segurança de autorização MCP](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization/security-considerations).

Aplicar ainda:

- Tratamento de IFC, BCF, PDFs, nomes de objetos e comentários como conteúdo não confiável.
- Restrição de arquivos e prevenção de acesso fora das raízes autorizadas.
- Limites de tamanho, tempo, quantidade, expansão de arquivos e custo computacional.
- Supressão ou mascaramento de dados sensíveis nos logs, excluindo segredos e minimizando conteúdo do projeto.
- Validação de tipo, enumeração, unidade, relações e valores antes de executar comandos.
- Autorização explícita do destino antes de enviar projeto a provedor externo.
- Testes de isolamento e de revogação, inclusive durante job e reconexão.

Um conteúdo importado não pode conceder permissão, ordenar envio de arquivos, alterar a política do projeto ou aprovar uma emissão. As credenciais de assinatura do responsável não ficam disponíveis ao agente.

## 7. Emissão técnica

Preparar documento, exportar prévia e emitir conjunto técnico são estados diferentes. A emissão exige identificação do responsável e aprovação autorizada do conjunto exato, ligado à revisão e aos hashes de resultados/documentos. Se esses dados mudarem, a aprovação anterior não se aplica ao novo conjunto.

A ferramenta recusa emissão com resultados obsoletos, análise incompleta, erro impeditivo, capacidade não validada para aquele domínio ou falta de aprovação exigida. Os critérios técnicos são definidos em [VALIDACAO.md](VALIDACAO.md); o mecanismo MCP não redefine esses critérios.

## 8. Aceitação da primeira operação e evolução

A issue #1 deve entregar servidor e interface próprios em Rust e demonstrar criação de projeto BIM real, objeto tipado mínimo, salvamento, reabertura e consulta, por interface e MCP. Reexecutar o mesmo comando nas duas superfícies deve produzir estado equivalente. Uma entrada inválida deve gerar diagnóstico sem corromper o projeto. Não usar cálculo ou objeto fictício para declarar conclusão.

Novas famílias de engenharia acrescentam ferramentas ou comandos ao contrato comum, com casos de equivalência. Gate de lançamento: versões e clientes declarados testados; zero mutação fora do escopo; nenhuma emissão obsoleta; recuperação de retries; isolamento de projetos; diagnósticos úteis; documentação de instalação que corresponda ao pacote distribuído.

Fontes e data de consulta: [fontes-arquitetura.json](fontes-arquitetura.json). Diretrizes de interação: [UX.md](UX.md).

## Ampliação para todas as instalações

O contrato de comandos cobre redes e ambientes além da estrutura. Consultar, lançar, conectar, desconectar, recalcular, explicar falhas, detalhar, extrair quantidades e preparar entregas são operações do mesmo serviço da interface. Ferramentas podem ser agrupadas por domínio para descoberta, preservando esquema, escopo e permissões comuns.

Operações em lote retornam uma prévia das conexões, escolhas de catálogo, conflitos, alterações de cálculo e documentos invalidados. A aprovação reutilizável respeita o escopo autorizado. Comandos carregam revisão esperada e chave de idempotência; repetição não duplica tubulações, fios, equipamentos ou documentos. A seleção de peça nunca aceita texto de catálogo como autorização para agir fora do projeto.

Os testes incluem um cliente real por família de operações, troca UI/MCP no mesmo projeto, falhas no meio de alterações, mudanças concorrentes e resultados que terminam depois de nova revisão. A issue 112 cobre o incremento estrutural; a completude ampliada é conferida nas etapas 227 a 250. Nenhuma operação relevante criada após a 112 fica dispensada de interface e MCP equivalentes.

O agente pode propor otimização de rotas, diâmetros, seções e equipamentos. Os resultados vêm dos motores determinísticos e de regras qualificadas, com restrições e objetivos explícitos. Uma explicação generativa não pode substituir diagnóstico, resultado numérico ou revisão técnica.
