# PRU-005: revisão de persistência, compatibilidade e recuperação

**Data:** 14/11/2026. **Base anterior:** PRU-004 integrada em `d8730d9722bf982230d444b30e284f2f21979380`. **Estado desta revisão:** revisão de código, pesquisa de componentes, contraprovas, validação integrada local e empacotamento concluídos. Parecer favorável à integração, vinculada à aprovação dos checks do PR para o mesmo código.

## Escopo e método

A revisão confrontou [a issue 5](../../planning/issues/005.md), a seção de persistência da [arquitetura](../ARQUITETURA.md) e o [critério comum de conclusão](../CONCLUSAO-ISSUE.md) com o código de domínio, banco, pacote, serviço, interface e MCP. Não foram acrescentadas funções da PRU-006 nem atribuída qualificação de engenharia ao armazenamento.

Foram examinados os pacotes publicados e seus arquivos legais, as APIs usadas pelo adaptador, o formato gerado, a ordem das publicações e os caminhos de erro. As contraprovas separaram comportamento da biblioteca, validação do formato e interação real. Uma falha por motivo diferente do pretendido não foi aceita como prova da guarda.

Entre os arquivos de produto, o revisor implementou, com autorização da integração, somente o ajuste de precisão JSON em `Cargo.toml` e `canonical.rs`, seus testes de domínio e `tests/storage_precision.rs`. Essa parcela recebeu revisão cruzada do responsável pelo armazenamento: comparação de chaves escapadas, preservação dos valores textuais, distinção dos erros e normalização de zero. As demais alterações de produto foram feitas pelos respectivos responsáveis após os apontamentos abaixo. O revisor também manteve as fichas e o gerador administrativo de avisos.

## Decisões de reuso verificadas

Foram adotados rusqlite 0.42.0, libsqlite3-sys 0.40.0 com SQLite 3.54.1 incorporado, e zip 9.5.1 com features reduzidas. O componente C permanece atrás do adaptador Rust. Não existe SQL, URI de conexão ou caminho de extração recebido do usuário ou do agente. SQLx 0.9.0, SQLite do sistema, async_zip 0.0.19 e implementação própria do contêiner foram confrontados e não adotados neste recorte, pelos motivos documentados na [ficha de reuso](../componentes/PRU-005.md).

A presença de libsqlite3-sys 0.37.0 no cache não foi usada como autorização: essa versão não contém a correção pertinente ao aviso RUSTSEC-2026-0459. A amalgamação efetivamente incorporada foi comparada à identificação e ao SHA3-256 da publicação oficial de SQLite 3.54.1. Os avisos conhecidos de ZIP também foram conferidos contra a versão fixada.

Os [cinco ensaios isolados em Rust](PRU-005-bibliotecas-ensaio.rs) foram executados pelo revisor e passaram. Eles demonstraram duplicatas ocultadas pela contagem usual de entradas, cabeçalhos divergentes aceitos sem a opção estrita, prefixo não detectável apenas por `offset()`, diferença entre cópia crua de um banco em WAL e backup consistente, e configuração defensiva efetiva. A fixture binária ZIP foi construída pelos campos da especificação, sem usar o escritor da biblioteca.

## Apontamentos e correções

| Problema observado | Correção e evidência exigida |
|---|---|
| FULL com rollback journal não fornece a mesma garantia de durabilidade de FULL em WAL | O banco usa DELETE e EXTRA, com consulta da configuração. A evidência distingue encerramento de processo de queda elétrica |
| Envelope interno futuro era convertido para DTO fechado antes da versão, podendo virar corrupção genérica | Versões do envelope atual e dos históricos são conferidas antes do DTO; a versão do cabeçalho SQLite é examinada antes de abrir a conexão |
| Versões numéricas amplas podiam sair da barreira de incompatibilidade ao não caber em `u32`, `u64` ou `f64` | Erros específicos conservam a representação numérica e bloqueiam transformação; regressões incluem valores acima de `u64` e `1e9999` |
| Uma fonte lateral futura podia ser rotulada apenas como candidato inválido e depois substituída | Futuro no principal, autosave, backup, original reconhecido ou banco bloqueia as publicações, inclusive criação sobre principal ausente |
| Muitas extensões podiam apontar para o mesmo blob e multiplicar cópias antes da conferência agregada | O tamanho expandido é contado antes de cada clone; 1.000 aliases de 1 MiB são recusados sem gerar o volume completo |
| O limite do banco, isoladamente, permitia confirmar um estado cujo pacote Stored excederia o limite de exportação | O tamanho composto, incluindo projeções, extensões, manifesto e cabeçalhos, é verificado dentro da transação. Uma extensão de 44 MiB provoca rollback completo |
| `ZipArchive::offset() == 0` não detecta prefixo incluído nos offsets gravados | O menor offset de cabeçalho local deve ser zero. A contraprova com 9 bytes de prefixo e offsets ajustados passa na biblioteca e é recusada pelo perfil Prumo |
| Alguns testes hostis usavam manifesto futuro e aceitavam qualquer erro | As provas passaram a distinguir recusa estrutural de `UnsupportedVersion`, impedindo que o manifesto futuro mascarasse uma guarda ausente |
| Igualdade de `Project` não identifica extensões binárias nem eventos administrativos do banco | A imagem da autoridade também é comparada. Divergência na mesma revisão exige escolher a recuperação, em vez de reutilizar silenciosamente o DB |
| Após interrupção da primeira edição de um JSON, nova edição ou migração podia reconstruir o banco a partir do legado antigo | Commit pendente bloqueia essa reconstrução. Um subprocesso é abortado após o commit e a revisão nova é recuperada explicitamente |
| A fonte era relida após a inspeção sem comparar novamente o hash do candidato | Recuperação confere os bytes ou a imagem do DB realmente usados, além de UUID e revisão |
| Troca externa do arquivo podia deixar o editor mostrando outro conteúdo | Diagnóstico compara identidade, revisão e hash; a seleção visual também fornece sua base ao serviço, que a verifica sob lock antes de editar |
| Apenas a saída anormal do processo não comprovava que a fase de falha desejada havia sido atingida | O filho escreve e descarrega o marcador da fase imediatamente antes do aborto; o pai exige o marcador e a saída anormal |
| `float_roundtrip` preservava campos conhecidos, mas arredondava números de dados desconhecidos | `arbitrary_precision` conserva a precisão dos `Number`; a normalização canônica identifica zero pela mantissa, sem underflow |
| A representação interna de precisão arbitrária podia converter objeto com chave privada em número | Guarda compartilhada recusa o namespace antes de `Value`, inclusive com escapes. Arquivos são classificados como incompatíveis; no stdio, o SDK recusa a requisição e a sessão continua |

Os apontamentos não foram resolvidos removendo dados, diminuindo as exigências de aceitação ou aumentando tolerâncias. As [provas do armazenamento](PRU-005-storage.md), [provas da interface e MCP](PRU-005-ui-mcp.md) e [matriz integrada](PRU-005.md) identificam as regressões e os comandos de reprodução.

## Precisão e representação JSON

A contraprova inicial usou serde_json 1.0.154 com a feature anterior, `float_roundtrip`. O inteiro `18446744073709551617` voltou como `1.8446744073709552e+19`, e `0.12345678901234567890123456789` voltou como `0.12345678901234568`. São mudanças inaceitáveis para campos desconhecidos cujo significado não pode ser presumido pelo formato atual.

Com `arbitrary_precision`, os dígitos foram conservados. A feature não acrescenta dependências. O caso `1e-9999` mostrou outra obrigação: converter para `f64` a fim de reconhecer zero produziria zero mesmo com mantissa não nula. A normalização agora inspeciona os dígitos da mantissa. A escrita pode explicitar o sinal positivo do expoente, como `1e9999` para `1e+9999`, mantendo o valor. A cópia original da migração permanece byte a byte igual.

A biblioteca usa um marcador privado para transportar números de precisão arbitrária entre deserializadores. A chave literal `$serde_json::private::Number` e suas formas escapadas são recusadas como formato incompatível, sem reinterpretar seu objeto. Uma string de valor que contenha esse mesmo texto continua permitida. JSON truncado recebe erro de sintaxe, permitindo o diagnóstico de corrupção; não é confundido com incompatibilidade de representação.

Os testes de `storage_precision.rs` abrem um JSON legado textual, migram, editam uma peça, produzem a inspeção geométrica e reabrem o pacote. Conferem as extensões, o original preservado, os snapshots e os bits da coordenada `1.9109059804556539`. Os testes anteriores de unidades, forças, momentos e hashes continuam fazendo parte da rodada integrada.

O revisor reexecutou os três testes de precisão do serviço na assinatura final do erro tipado; todos passaram. Também reexecutou 7 testes unitários, 18 de modelo, 1 de histórico normativo e o doctest de distinção dimensional do domínio, sem falhas. Essas contagens são das baterias indicadas, não uma soma independente a acrescentar ao total da execução de workspace.

## Invariantes conferidos no código

O banco confirma conteúdo imutável, registro e cabeçalho na mesma transação. A sequência é comparada à revisão anterior, ao prefixo de histórico, à identidade do projeto e à unicidade do comando. Um evento administrativo de recuperação só conserva a revisão quando o projeto também é idêntico. Importação de baseline não inventa comandos anteriores. A leitura verifica o esquema exato, índices, integridade SQLite, referências e hashes antes de devolver o modelo.

A imagem exportada vem da API de backup. O leitor privado de uma imagem por bytes abre um arquivo temporário próprio, valida o esquema e chama obrigatoriamente a validação completa de conteúdo antes de retornar. A otimização desse caminho elimina repetição dentro da mesma leitura; não mantém um cache de validação para arquivos externos mutáveis.

O pacote é confrontado integralmente com a imagem do banco. Corrigir o SHA de uma projeção adulterada não a torna autoridade. Os limites se aplicam ao contêiner, metadados, entradas declaradas, bytes efetivamente lidos, expansão combinada, histórico e exportabilidade. As extensões opacas são indexadas no banco e sobrevivem quando só a fonte DB permanece disponível. Não são convertidas em regra, cálculo ou aprovação.

O serviço distingue a revisão publicada da confirmada ainda pendente. A inspeção considera também ausência do principal e arquivos auxiliares. Recuperação exige permissão, token atual, candidato coerente e confirmação. Preserva o incidente, publica uma cópia do candidato antes de mover o banco anterior e registra a recuperação depois de restaurar. A leitura não escolhe silenciosamente uma fonte alternativa.

A interface e o MCP usam o mesmo serviço. O painel permanece utilizável quando o projeto não abre; não oferece criação por confundir arquivo ausente com contexto livre de recuperação. O MCP não recebe caminhos, SQL, autoria ou concessão de permissão. A guarda de representação funciona antes da desserialização do SDK e não implementa um protocolo JSON-RPC paralelo.

`read_with_basis` fornece o projeto e o hash dos mesmos bytes lidos. A interface guarda essa base separadamente do diagnóstico. `with_write_basis` a confere sob o lock de mutação; uma consulta posterior não pode associar o hash de outro arquivo a um modelo antigo. Depois de gravar, a nova leitura pareada precisa concordar com o projeto devolvido, ou o editor solicita reabertura.

## Redução de trabalho sem retirar verificações

A integração mediu uma migração normativa em debug que excedia o prazo do cliente de teste. Foram conferidos ajustes destinados a eliminar trabalho repetido dentro da mesma operação, sem declarar confiável um arquivo externo nem retirar validações. As observações anteriores ao ajuste final permanecem no [registro de latências](PRU-005-latencia.json).

`compare_packages` continua validando os dois pacotes recebidos. `NormativeState::validate` valida o ativo e todos os anteriores uma vez antes de comparar a cadeia com um helper privado. Os dois chamadores desse helper possuem a validação prévia na mesma chamada. A regressão altera separadamente cada pacote antigo, o ativo e o impacto, e exige recusa.

`EncodedPackage` possui bytes privados e seu único construtor é o encoder que valida uma imagem consistente do banco e monta as projeções, hashes, nomes e limites. Não possui desserialização, construção pública por `Vec` ou acesso mutável. A publicação aceita esse tipo e confere tamanho e hash do temporário antes da substituição. Consumir o tipo devolve bytes comuns; qualquer entrada recebida continua passando pelo decoder integral. O teste de compilação impede a construção externa e a contraprova adultera uma projeção após consumir o pacote.

No fim de um commit, a reconstrução é comparada byte a byte ao projeto validado no início daquela mesma chamada. Só essa igualdade permite dispensar a repetição da validação de domínio. Objetos, versões, cadeia histórica, extensões, limites e exportabilidade continuam conferidos. O helper é privado e os leitores externos não recebem esse atalho. Um teste fornece os bytes de outro projeto igualmente válido e exige erro, mostrando que validade isolada não substitui identidade da revisão.

Na inspeção, `backup_read_only` fornece bytes comuns de uma imagem consistente, sem atribuir validação ao seu conteúdo. O serviço só reutiliza o projeto lido quando a imagem inteira coincide byte a byte com o banco do pacote já decodificado naquela chamada. Uma divergência segue para o snapshot com validação integral. Não há cache entre operações nem sinalizador externo que dispense essa comparação.

Uma tentativa de eliminar a árvore intermediária de `Value` não foi aprovada: o ensaio offline existente encontrou incompatibilidade entre `arbitrary_precision`, o buffer de enum internamente tagueado do Serde e os campos `f64` da inspeção geométrica. A reconstrução direta recusou um projeto válido com `invalid type: map, expected f64`. Essa observação impediu tratar uma redução de tempo como sucesso antes de conservar o roundtrip do modelo completo. O caminho anterior foi restaurado e a mesma prova passou novamente.

A amostra final do serviço em debug registrou 2,481 s para fixar o pacote normativo, 6,794 s para cadastrar a referência e 10,867 s para migrar. A inspeção posterior levou 4,662 s. A migração continuou excedendo o limite anterior de 10 s do helper genérico dos testes MCP. Essa observação não foi apagada nem classificada como atendimento de um prazo que não alcançou.

O [PRD](../../PRD.md), a [issue 5](../../planning/issues/005.md), os [contratos MCP](../MCP.md) e a [arquitetura](../ARQUITETURA.md) não estabelecem migração em 10 s. O documento de [UX](../UX.md) apresenta metas próprias para cenários e hardware previamente fixados, sem transformar esse valor genérico de teste em requisito do armazenamento. A revisão aprovou um orçamento explícito de 30 s somente nas chamadas `pin`, `register` e `migrate` da prova normativa com catálogo completo. O helper comum, discovery e o cliente independente por pipes mantêm 10 s; as demais provas não tiveram seus prazos alterados. A recusa de migração sem confirmação também passou a exigir resposta em até 10 s. Permanecem todas as comparações do histórico, hashes, confirmação, ausência de mutação na prévia, recusa e reabertura pela interface. Esse limite serve para detectar travamento durante o teste funcional em debug; não qualifica o desempenho do aplicativo nem altera tolerâncias numéricas ou limites do formato.

A prova posterior com processo MCP real passou em 46,91 s no total. As chamadas registraram 2,812 s para fixar, 7,187 s para cadastrar e 11,319 s para migrar. O pacote final da fixture ficou na revisão 4, com 12.660.856 bytes. Esses resultados constam da [evidência do cliente](PRU-005-ui-mcp.md) e continuam identificando o custo acima de 10 s; a mudança de orçamento não foi apresentada como uma melhoria de velocidade.

## Auditoria de distribuição

O lock examinado tem SHA-256 `149f51de3ed35dc6861c00479abc8f16e65414fdbdc1f8ee313ffee041f8ec23`: 469 pacotes externos, 321 no escopo normal/build dos targets nativos e 272 textos legais distintos. Dez pacotes foram acrescentados, sem remover ou atualizar versões anteriores. Dois pacotes já existentes, rustversion e zlib-rs, também passaram a integrar esse escopo nativo.

`cargo deny --locked check advisories licenses sources` e a conferência reproduzível de avisos passaram. A base RustSec examinada está no commit `4ef2df8e806a65e2f1ecc14f56f51249062b23d6`. Não foi adicionada exceção de licença ou vulnerabilidade. As quatro exceções de manutenção herdadas permanecem identificadas e restritas às suas cadeias anteriores.

O aviso MIT do binding não foi confundido com a dedicação do núcleo SQLite ao domínio público. O gerador preserva o cabeçalho efetivamente recebido e seu hash. O responsável pela integração conferiu esse ajuste administrativo. As versões, opções, licenças por pacote e obrigações da distribuição constam da [ficha de componentes](../componentes/PRU-005.md) e dos [avisos consolidados](../../THIRD_PARTY_NOTICES.md).

O workflow recompila o workspace no grafo normal depois dos testes e antes de empacotar, em Linux e Windows. Essa etapa foi conferida porque os testes podem recompilar os executáveis com a união de features das dependências de desenvolvimento. O build final recuperou os hashes do grafo de distribuição examinado; os avisos continuam correspondendo ao escopo normal/build que efetivamente será distribuído.

## Verificação local e identidade do artefato

A rodada integrada final passou em **37 baterias, com 186 testes aprovados, zero falhas, zero ignorados e zero filtrados**. O revisor conferiu essas contagens diretamente no log bruto. Elas incluem as provas do domínio anterior, armazenamento, interface, MCP, precisão, 14 interrupções reais de processo e doctests. Os cinco ensaios isolados da biblioteca pertencem à contraprova adicional descrita acima, sem serem somados novamente às baterias do workspace.

Build fixado, formatação, Clippy com `--all-targets -- -D warnings`, interface em WASM, reprodução do módulo WASM, verificador do planejamento, inventário e auditoria passaram. Os fluxos completos da interface e do MCP com o catálogo normativo permanecem na rodada. A [matriz principal](PRU-005.md) registra os comandos e a identificação dos executáveis.

Na conferência nativa final da integração, uma nova cópia do legado da revisão 5 foi migrada. A tela apresentou o resultado e o estado íntegro sem movimento do ponteiro ou clique adicional. A inspeção independente do pacote confirmou 17 entradas, CRC, tamanhos, hashes, integridade SQLite e `project.json` byte a byte igual ao caso anterior. O executável observado possui SHA-256 `0caf3e42396a21b09eb4073baefde22b7393538b6dcacd9b20d6285c559ab4f5`. Depois da recompilação para distribuição, o revisor recalculou o hash e confirmou exatamente essa identidade.

O pacote local `prumo-development-linux-x86_64.zip` possui **142.012.212 bytes**, com SHA-256 `41ff7df48bdfbd9cf5aef374db75789168ee0db8558848a4423694a3cb7e5354`. A extração, os hashes, os três executáveis e a suíte do worker passaram. O revisor conferiu os hashes dos executáveis e do ZIP, o conteúdo do manifesto e o log dessa instalação. O manifesto registra perfil debug, toolchain 1.99.1, origem `081d96fe22a240a5a092cf900251cd568503ca65` e `source_dirty: true`, identificando corretamente a montagem local anterior ao commit. Esse pacote não é atribuído a um HEAD limpo; os artefatos do CI identificam o commit publicado.

A aprovação de merge exige os checks Linux, Windows e de planejamento do PR, com seus resultados e artefatos vinculados à revisão publicada. A aprovação local não substitui esses checks, e a issue só pode ser encerrada após o conjunto de aceites.

## Limites do parecer

Os ensaios de encerramento abrupto não equivalem a desligamento elétrico ou ensaio de qualquer sistema de arquivos. Os limites do formato são explícitos e uma operação que os exceda é recusada. Não há remoção automática de histórico para acomodar um arquivo. Cópias no mesmo disco não substituem uma estratégia externa contra perda do dispositivo.

O recorte preserva as capacidades instaladas das entregas anteriores, incluindo medidas geométricas rastreáveis. Não libera análise estrutural, dimensionamento de instalações, emissão normativa, desfazer/refazer ou interpretação de extensões futuras. Os critérios de implementação, revisão e validação local desta entrega foram atendidos. O registro de integração deve conservar os resultados dos checks e a identidade do commit efetivamente publicado.
