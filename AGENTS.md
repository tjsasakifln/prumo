# Instruções de implementação do Prumo

## Missão e estado

Este repositório começa como planejamento. Não existe motor de cálculo validado por ter sido documentado aqui. O produto abrange estruturas e instalações hidrossanitárias, elétricas, fotovoltaicas, SPDA, cabeamento, gás, incêndio e climatização. O objetivo é implementar o produto completo definido em PRD.md, na sequência ampliada de 250 issues.

Leia README.md, PRD.md, ROADMAP.md, a issue escolhida e os documentos pertinentes antes de alterar o código. Verifique também instruções do proprietário presentes na sessão e em diretórios mais específicos. Não invente uma autorização, norma, resultado ou aprovação ausente.

## Seleção da próxima issue

1. Consulte as issues e a branch principal atuais. Não use apenas uma cópia antiga da fila.
2. Selecione a menor issue ainda não concluída da sequência PRU-001 a PRU-250.
3. Confirme que todas as anteriores foram implementadas, integradas e verificadas. Fechamento como duplicada, cancelada ou não planejada não conta.
4. Leia os critérios completos, incluindo testes de erro, referências normativas, reuso e contratos transversais.
5. Mantenha a issue aberta se qualquer critério obrigatório estiver pendente. Nunca crie um resultado de teste ou assinatura para passar o bloqueio.

A sequência ampliada mantém a correspondência entre número GitHub e identificador PRU. A antiga issue 120 passou a ser um marco de desenvolvimento estrutural; o lançamento completo pertence à 250. PRs e issues de manutenção criadas depois não alteram os identificadores PRU. Correções de uma entrega permanecem vinculadas a ela e devem ser resolvidas antes de avançar. Uma eventual subdivisão deve preservar a dependência e o aceite integral da issue original.

## Rust obrigatório

Implemente em Rust todo código próprio do produto, inclusive interface, BIM, redes, comandos, regras normativas, cálculos próprios, persistência, CLI, MCP, serviços, adaptadores e orquestração. Leia docs/RUST.md e planning/technology.json. REQ-15 e TEC-RUST-01 aplicam-se às 250 issues. A escolha futura de framework deve respeitar essa decisão do proprietário.

Na issue 1, entregue workspace Cargo funcional, Cargo.lock e rust-toolchain.toml com compilador estável fixado; declare edition, rust-version, resolver, targets e features. Execute formatação, Clippy, testes e compilação com dependências fixadas na matriz adotada. Os comandos Cargo só passam a fazer parte da verificação do produto quando o workspace real existir.

Componentes externos podem conservar suas linguagens, com origem, licença, versão, função, adaptador Rust e testes de fronteira documentados. Bindings gerados e patches externos têm escopo restrito e revisado. Não crie módulos próprios em outras linguagens sob o rótulo de dependência. O host de extensões e o executor de rotinas são Rust; conteúdo externo isolado continua sujeito a comandos e permissões.

Proíba unsafe nas camadas puras próprias de domínio, comandos e regras. Concentre exceções necessárias em crates de integração pequenos, com invariantes e revisão. Rust não dispensa contraprovas numéricas, testes de integração nem revisão profissional. Os scripts Python de manutenção do planejamento permanecem fora do runtime do produto.

## Reuso obrigatório quando pertinente

Antes de implementar um módulo, examine docs/REUSO.md e docs/candidatos-reuso.json. Procure componentes abertos adequados nas fontes oficiais. Compare adoção, adaptação e implementação própria pelo custo total.

Registre origem, licença por módulo, dependências, commit/versão, manutenção, segurança, ensaios próprios e motivo da decisão. Preserve avisos. Candidato listado não significa componente aprovado. Bibliotecas científicas não herdam automaticamente conformidade com normas brasileiras.

Na primeira issue já existem triagem de licença, versão fixada e permissões mínimas para os componentes usados. A qualificação ampla da issue 3 não autoriza uso anterior sem exame.

## Regras invariantes

- BIM e MCP são reais desde a primeira entrega. Criar pastas, respostas fixas ou ferramentas que não alteram o projeto não atende ao requisito.
- Interface, CLI e MCP usam o mesmo serviço de comandos e validação.
- Modelo físico, analítico, resultados e documentos têm identidades relacionadas e estados próprios.
- Unidade, sinal, eixo, tolerância e hipótese devem ser explícitos.
- O motor numérico não usa modelo de linguagem para decidir resistência ou gerar valores de cálculo.
- Não alterar modelo, norma ou material sem invalidar resultados dependentes.
- Não acrescentar vínculos ou rigidez artificial silenciosa para fazer o solver convergir.
- Não tratar resultado incompleto, desatualizado, fora do domínio ou não convergido como aprovado.
- Não gerar falsa assinatura, revisão humana, ART, certificado ou homologação.
- Não expor shell, SQL, caminho arbitrário, credencial ou acesso entre projetos por ferramentas MCP.
- Não copiar código, normas, manuais, bases ou detalhes proprietários sem autorização compatível.

## Como implementar e verificar

Trabalhe em branch isolada quando houver código em desenvolvimento. Preserve alterações de outras pessoas. Cada entrega deve indicar o problema resolvido, o comportamento produzido e evidências que permitam reproduzir a verificação.

Use os critérios específicos da issue e docs/VALIDACAO.md. Escolha testes que possam detectar o erro real. Uma mudança em fórmula exige contraprova; uma mudança em desenho exige abrir e inspecionar o arquivo; uma mudança em MCP exige cliente e contrato; uma mudança em experiência exige executar a tarefa.

Mantenha os casos de referência independentes da implementação. Fixe tolerâncias antes da comparação. Registre versão, configuração, snapshot, binário e dados. Revisão por outro agente é útil para defeitos de implementação e planejamento, mas não substitui a revisão técnica por profissional habilitado exigida para a qualificação final.

Etapas de modelagem anteriores ao solver podem testar topologia e contratos usando fixtures explicitamente identificadas. Nunca mostrar a fixture como cálculo disponível. A integração numérica é obrigatória nas issues correspondentes e nas campanhas finais.

Cada nova função relevante deve incluir operação equivalente por MCP. Depois da issue 99, documentos, intercâmbio e emissão continuam a ampliar o contrato; a 112 confere os fluxos estruturais então existentes. As instalações ampliam continuamente o mesmo contrato; a integração completa é conferida nas issues 227 a 250.

## Critério de conclusão de uma issue

- Escopo e critérios específicos implementados, com comportamento funcional.
- Código próprio em Rust, conforme REQ-15; compilação, dependências e fronteiras externas verificadas na parcela entregue.
- Reuso examinado e decisão registrada; licenças resolvidas para o código incorporado.
- Testes pertinentes executados sobre o commit e artefato corretos.
- Referências normativas e limites atualizados quando houver cálculo.
- Revisão independente dos pontos críticos, com correções aplicadas.
- Documentação, exemplos, estados de capacidade e matriz de evidências atualizados.
- Código integrado na branch principal e verificações requeridas aprovadas.
- Evidências e PRs ligadas à issue; não fechar por um resumo sem comprovação.

## Lançamento

Nenhum agente pode retirar requisito, diminuir cobertura, alargar tolerância ou dispensar revisão apenas para concluir a issue 250. Uma necessidade de mudar materialmente o objetivo deve ser apresentada ao proprietário com impactos claros.

Antes do lançamento, reconfirme normas, componentes e vulnerabilidades. Projetos antigos preservam seu pacote normativo. A versão comercial deve ser exatamente o artefato aprovado, com suporte, atualização, direitos de distribuição e tratamento de incidentes numéricos.

## Comunicação

Escreva em português claro. Separe fatos, hipóteses, dados pendentes e funcionalidades planejadas. Evite travessões e jargão desnecessário. Informe o resultado, a evidência e os impedimentos reais, sem chamar planejamento de implementação ou testes automatizados de certificação profissional.

## Ampliação para instalações

Desde as primeiras issues, os contratos devem admitir ambientes, redes, portas e conexões tipadas. A etapa inicial valida esses contratos sem apresentar motores futuros como disponíveis. Os cálculos de redes entram nas suas issues de implementação. Um cruzamento visual não cria ligação hidráulica, elétrica, lógica ou resistente.

Cada função distingue modelar, calcular, verificar e emitir. Dados de catálogo, concessionária e regulamentação local têm origem, versão e domínio. Mudanças de equipamento, circuito, ramal, vazão, pressão, carga, proteção, ambiente ou regra invalidam as dependências corretas, inclusive entre disciplinas.

Não ampliar uma biblioteca de água para gás compressível, nem um solver elétrico para dimensionamento normativo, sem qualificação específica. Limites documentais dos sistemas pesquisados ficam na matriz; pendências de fonte não autorizam omissão de capacidade obrigatória. A emissão integrada será qualificada após os módulos de instalações.

## Identidade da apresentação

README.md e LEIAME.md apresentam o Prumo por suas próprias funções. Não incluir nomes, slogans, comparações ou links identificadores de softwares de referência nesses dois documentos. Manter atribuição e fontes técnicas nos documentos de pesquisa e de cobertura. O verificador do planejamento confere essa restrição.
