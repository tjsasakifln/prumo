# Arquitetura do Prumo

**Estado:** especificação inicial de produto, ainda sem implementação. **Data:** 14/11/2026. Rust é a linguagem obrigatória do código próprio, conforme [RUST.md](RUST.md), REQ-15 e TEC-RUST-01. As issues #1 e #3 selecionam frameworks e componentes dentro dessa decisão, com licença e ensaios reproduzíveis. Os requisitos de domínio, rastreabilidade e operação descritos aqui são critérios de aceitação, não afirmações de capacidade já entregue.

## 1. Decisão inicial

Aplicativo desktop para Windows com operação local completa e interface web reutilizável. Um projeto deve poder abrir, editar, calcular e produzir documentos sem depender de conexão, conta remota ou provedor de agentes. A colaboração hospedada e o processamento remoto só podem ser oferecidos quando sua capacidade, autenticação e isolamento estiverem implementados e testados.

A implementação própria será **Rust em todas as camadas**, incluindo a interface, o servidor MCP e os cálculos desenvolvidos pelo Prumo. A interface poderá usar execução nativa ou Rust/WASM. Motores externos poderão executar em processo separado, por FFI ou por WASM, com adaptação Rust, contratos e qualificação. A arquitetura preserva a operação local e o reaproveitamento web da interface.

| Candidato de interface/gráficos | Papel possível | Evidência exigida antes da escolha |
| --- | --- | --- |
| egui/eframe | Autoria Rust com destinos nativos e web | Acessibilidade, edição CAD/BIM, grandes coordenadas, impressão, múltiplas vistas e consumo no caso de referência |
| Dioxus | Autoria Rust em desktop/WebView e web | Integração gráfica, limites de recursos, impressão e operação local; identificar renderizadores experimentais |
| wgpu | Camada gráfica Rust de uma interface qualificada | Backends/targets, precisão visual, seleção, escala, memória e recuperação de falha; não é uma interface BIM completa |
| Contêiner opcional com interface Rust/WASM | Tauri ou solução compatível, se houver benefício demonstrado | Autoria Rust preservada, permissões, integração local, empacotamento e atualização |

Essas opções são candidatos, não uma seleção concluída. O ADR compara alternativas plausíveis com o mesmo projeto e as metas de [UX.md](UX.md). Fontes e limites: [fontes-rust.json](fontes-rust.json). As capacidades e o empacotamento de Tauri, quando pertinente, seguem sua [documentação oficial](https://tauri.app/security/capabilities/) e os testes do Prumo.

Não exigir que o cliente instale Python, compiladores ou ferramentas de desenvolvimento. Se uma biblioteca externa Python ou de outra linguagem for aprovada, o aplicativo deve distribuir o runtime necessário, com versão fixa, avisos e obrigações correspondentes. Essa integração não autoriza código próprio do produto fora de Rust. A [política de reuso](REUSO.md) aplica-se também ao solver, geometria, parser IFC, visualização e MCP.

## 2. Componentes e responsabilidades

| Componente | Responsabilidade | Limite obrigatório |
| --- | --- | --- |
| Interface | Planta, corte, 3D, propriedades, montagem, diagnóstico, relatórios | Sem implementação paralela de regra normativa ou alteração direta do banco |
| Serviço de comandos | Validação de entradas, permissões, transações, histórico, consultas | Única porta de mutação para UI, CLI e MCP |
| Domínio BIM | Objetos físicos, modelo analítico, materiais, seções, ações, estágios e vínculos | Geometria visível não define sozinha conectividade estrutural |
| Motor numérico | Matrizes, solução, análise, convergência e diagnóstico | Snapshot imutável; sem dependência de linguagem natural ou do provedor de agente |
| Pacotes normativos | Verificações, parâmetros, limites de aplicação, referências e testes | Edição fixada por projeto; regra não implementada deve ser identificada |
| Adaptadores BIM/CAD | IFC, BCF, IDS, desenhos e conversões | Capacidades declaradas e relatório de perdas |
| Documentação | Memoriais, pranchas, detalhes, tabelas e conjunto emitido | Conteúdo rastreável ao modelo, cálculo e verificações correspondentes |
| Persistência | Banco transacional, histórico, objetos por hash, recuperação e pacote nativo | Salvamento atômico, integridade e migrações reversíveis por backup |
| MCP | Ferramentas e recursos para agentes | Os mesmos comandos e permissões da interface; sem shell ou SQL arbitrários |
| Serviços hospedados | Compartilhamento, identidade, sincronização e jobs remotos, quando aprovados | Autenticação, autorização por projeto, isolamento e comportamento de falha testados |

O código futuro será organizado em workspace Cargo. Estrutura possível: `apps/prumo-desktop`, `apps/prumo-cli`, `apps/prumo-mcp`, `crates/prumo-domain`, `crates/prumo-commands`, `crates/prumo-contracts`, `crates/prumo-persistence`, `crates/prumo-numerics`, `crates/prumo-norms`, `crates/prumo-ui`, `crates/prumo-render` e `crates/prumo-adapters`, além de `verification`, `fixtures` e `docs/adr`. Os diretórios não representam módulos entregues; serão criados conforme comportamento e responsabilidade reais. Compilação, toolchain, lockfile e dependências externas seguem [RUST.md](RUST.md).

## 3. BIM nativo e modelo analítico

O modelo de projeto deve conter entidades de engenharia desde sua origem. Pilar, viga, laje, ligação, material, apoio, ação e verificação são objetos identificáveis com propriedades e relações. Uma malha gráfica usada para visualização é uma representação derivada.

Cada entidade recebe identidade estável interna. GUID IFC, identificador do arquivo de origem e relação de proveniência são campos próprios. Uma cópia cria identidade nova; uma reexportação do mesmo objeto preserva seu GUID quando aplicável. Divisão e fusão de elementos mantêm a correspondência de revisão.

O **modelo físico** e o **modelo analítico** são distintos e ligados por relações que admitem 1:n e n:1. A idealização registra eixos, offsets, nós, liberações, rigidez, diafragmas, regiões de malha, estágios e justificativas. Um modelo visualmente conectado pode estar estruturalmente desconectado; essa diferença deve ser apresentada e diagnosticada.

Um encaixe que altere comportamento resistente precisa mostrar a hipótese e os elementos afetados. O usuário pode inspecionar o caminho de cargas, os apoios, as conexões e as transferências entre elementos. Lançamento assistido propõe soluções e explicita hipóteses; não cria silenciosamente categorias de uso, dados geotécnicos ou ligações.

### Unidades e coordenadas

- Tipos próprios para comprimento, força, momento, tensão, massa, temperatura, tempo, rotação e rigidez.
- Convenção interna única, documentada e verificada, com conversões explícitas nos contratos externos.
- Sistemas local de engenharia e global/georreferenciado separados, com transformação registrada.
- Tratamento de vírgula e ponto conforme a interface, sem interpretação ambígua de valor técnico.
- Tolerâncias distintas para seleção, coincidência geométrica, fusão topológica, malha, convergência e comparação numérica.
- Proibição de números não finitos, unidade ausente quando obrigatória e conversões incompatíveis.

## 4. Persistência e formato `.prumo`

Durante a edição, usar banco local transacional com objetos imutáveis por hash e registro de comandos. O arquivo de intercâmbio nativo `.prumo` será um pacote versionado, com gravação atômica e validação de integridade. Seu esquema final pertence às issues correspondentes; a estrutura abaixo é o contrato funcional inicial.

| Conteúdo do pacote | Requisito |
| --- | --- |
| Manifesto | Identidade do projeto, versão de formato, revisão, data, origem e hashes |
| Modelo físico | Elementos, propriedades, relações, coordenadas e referências |
| Modelo analítico | Idealização completa e correspondência com o modelo físico |
| Catálogos necessários | Materiais, seções e parâmetros efetivamente usados, com origem e versão |
| Perfil normativo | Edições, emendas, regras habilitadas, limites e parâmetros do cálculo |
| Histórico | Comandos, revisão base, ator autenticado, motivo e alterações |
| Resultados | Snapshot, solver, configurações, diagnósticos, convergência e verificações |
| Documentos | Prévia ou emissão, revisão e evidência associadas |
| Intercâmbio | IFC e outros arquivos autorizados, com declaração de perdas |

Credenciais, tokens, segredos de assinatura e documentos normativos licenciados sem direito de redistribuição não integram o pacote. Importar arquivos exige limites de tamanho e descompressão, rejeição de caminhos inválidos, verificação de hashes e migração controlada. Uma migração preserva a origem e não converte automaticamente resultados antigos em resultados válidos para versão nova.

**IFC não será o único registro do cálculo.** O domínio de análise estrutural da publicação IFC4.3 não representa integralmente dinâmica, ações de protensão, topologia MEF e resultados detalhados de malha. Esses dados devem permanecer no pacote nativo, com as perdas do intercâmbio declaradas. [Escopo oficial do domínio estrutural](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/ifcstructuralanalysisdomain/content.html).

## 5. Interface do motor numérico

O adaptador do motor recebe um snapshot completo e imutável, independentemente de a solicitação partir da interface ou de um agente. O contrato deve incluir unidades, convenções de sinais, eixos, nós, elementos, materiais, seções, vínculos, ações, combinações, estágios, regras aplicáveis, malha e critérios de solução.

A resposta inclui status, versão do motor e dependências, hash da entrada, resultados por grandeza, equilíbrio, convergência, diagnósticos, limitações e hashes das evidências. Não convergir, identificar mecanismo ou atingir limite de memória são resultados diagnósticos; nenhum deles pode gerar aprovação parcial silenciosa.

Workers têm limites de CPU/memória, cancelamento cooperativo, comunicação estruturada e gravação temporária isolada. Uma falha não corrompe o projeto. Jobs remotos, se implementados, utilizam o mesmo contrato e deixam explícito onde os dados serão processados. Resultado obtido de outro snapshot nunca substitui o vigente sem identificação.

Cada componente existente adotado deve passar pelos casos da [validação](VALIDACAO.md), incluindo a tradução de entradas e saídas. Comparar dois programas que usam a mesma implementação não constitui independência suficiente.

## 6. Resultados, revisão e emissão

Mudanças em geometria, material, hipótese, apoio, ação, combinação, estágio, regra normativa, solver ou configuração invalidam os resultados dependentes. Exibir claramente estado e motivo. O usuário pode consultar resultado antigo para comparação, sempre identificado como histórico.

Um conjunto emitido associa revisão do projeto, snapshot analítico, solver, perfil normativo, verificações, documentos e responsável. A emissão exige conjunto coerente e revisão autorizada, conforme [MCP](MCP.md) e [validação](VALIDACAO.md). Um agente não pode substituir o responsável nem obter sua assinatura por texto de comando.

## 7. Interoperabilidade

O catálogo oficial identifica IFC4.3 ADD2 como final e IFC4.4 como desenvolvimento. A matriz de intercâmbio deve fixar schema e perfil, incluindo IFC4 ADD2 TC1 para edifícios e compatibilidade legada definida para IFC2x3. [Catálogo buildingSMART](https://standards.buildingsmart.org/).

Cada adaptador declara entidades, propriedades, materiais, vínculos, georreferenciamento, unidades e versões suportadas. Arquivo arquitetônico importado permanece referência até promoção explícita dos elementos ao domínio de projeto. Informações analíticas ausentes exigem decisão; não podem ser completadas como se fossem dados recebidos.

Testar schema, semântica, GUID, coordenadas e conservação das informações previstas, além da aparência. Reabrir exportações em pelo menos dois leitores independentes na matriz de validação. Comparar ida e volta com tolerâncias específicas e relatório de perdas.

IDS verifica requisitos de informação, e BCF organiza apontamentos relacionados ao modelo. Nenhum desses formatos comprova a segurança estrutural. [IDS](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/) e [BCF](https://github.com/buildingSMART/BCF-XML).

DWG deve possuir decisão própria de SDK, licença e compatibilidade. Não converter extensão de arquivo para simular formato. Não prometer edição plena dos formatos proprietários dos sistemas de referência sem especificação ou licença apropriada.

## 8. Entrega incremental e critérios arquiteturais

Desde a issue #1: workspace Cargo funcional, interface e servidor MCP em Rust, projeto mínimo real, persistência, comando pelas duas superfícies, falha diagnosticada e teste de equivalência. Conforme novas famílias estruturais e de instalações forem implementadas, seus comandos, desenhos, relatórios e verificações evoluem juntos. Não concentrar toda a integração em uma etapa final.

Critérios transversais: reabrir sem perda; rejeitar unidades inválidas; impedir mutação concorrente silenciosa; repetir comando sem duplicação; bloquear emissão obsoleta; recuperar queda do worker; demonstrar equivalência UI/MCP; explicar perdas do intercâmbio; preservar procedência e licenças de código reutilizado.

Fontes adicionais e data de consulta: [fontes-arquitetura.json](fontes-arquitetura.json). Metas de facilidade de uso: [UX.md](UX.md).

## 9. Domínio compartilhado de instalações

A revisão 2.0 inclui ambientes e sistemas prediais desde os contratos iniciais. Cada elemento pode ter portas tipadas, orientação, dimensão, cota, conectividade, papel no sistema e limites de operação. Rede hidráulica, circuito elétrico, conexão lógica, distribuição de ar e ligação resistente têm tipos e regras próprios. Proximidade geométrica não cria conexão entre sistemas. Uma mesma peça pode ter representações distintas e relações explícitas com equipamentos, ambientes e estrutura.

A topologia de cálculo é derivada do modelo físico com proveniência: nós e ramais hidráulicos, fases e circuitos, séries fotovoltaicas, enlaces de comunicação, zonas de proteção e ambientes térmicos. Catálogos versionados preservam portas, símbolos, propriedades de cálculo, itens associados, unidades comerciais e avisos de procedência. Kits e blocos mantêm identidades próprias, composição e regra de atualização sem duplicar quantitativos.

Os motores são especializados por domínio: estruturas; redes pressurizadas; redes gravitárias; gases; elétrica; fotovoltaica; proteção contra descargas; comunicação; térmica e circulação de ar. A aprovação de um motor em um domínio não se estende aos demais. Todos recebem entradas imutáveis, perfis normativos e hipóteses; retornam unidades, convergência, diagnóstico, limites e hashes.

O sistema de unidades inclui vazão, volume, pressão absoluta e manométrica, potência ativa/reativa/aparente, corrente, tensão CA/CC, frequência, irradiância, energia, temperatura, umidade, capacidade térmica e propriedades de fluido. Conversões entre condições de referência do gás e entre grandezas elétricas exigem contexto explícito.

O grafo de dependências atravessa disciplinas: potência de bomba afeta circuito; reservatório afeta massa e apoios; passagem afeta estrutura; carga térmica afeta equipamentos; sombreamento e montagem afetam geração e cargas; posição de armadura usada como componente natural afeta o SPDA. O sistema apresenta impacto e exige a revisão correspondente antes de emissão.

As etapas 121 a 130 consolidam o domínio de instalações. Documentos técnicos nativos acompanham cada módulo; a coordenação e emissão multiformato são ampliadas em 227 a 238. As campanhas finais 239 a 250 exercitam todas as disciplinas no mesmo candidato. IFC preserva sistemas, distribuição e propriedades segundo os perfis declarados; o pacote nativo mantém dados analíticos não representáveis. Referências proprietárias usam adaptadores licenciados e matrizes de versão, sem promessa de autoria completa do formato.

Fontes de escopo e limites: [INSTALACOES.md](INSTALACOES.md).
