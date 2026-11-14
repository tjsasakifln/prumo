## Problema e resultado

Explique o problema concreto, a mudança e o comportamento resultante.

## Issue e dependências

Identifique PRU e issue GitHub. Registre a conferência das etapas anteriores.

## Reuso e decisões

Componentes avaliados, decisão, versão/commit, licença e obrigações dos trechos incorporados.

## Verificação

Casos executados, resultado, evidência reproduzível, limitações e revisão independente. Para cálculo, incluir referência independente, unidades, tolerância pré-registrada e domínio. Para documentos, abrir e inspecionar o exportado.

## Coerência do produto

- [ ] Interface e MCP usam o mesmo contrato.
- [ ] Alterações relevantes invalidam resultados dependentes, inclusive entre estruturas e instalações.
- [ ] Capacidades e documentos atualizados.
- [ ] Nenhuma pendência foi retirada apenas para concluir.
- [ ] Artefato verificado corresponde ao commit examinado.
- [ ] O escopo e a evidência distinguem modelagem, cálculo, verificação e emissão por disciplina.

## Implementação Rust

- [ ] Código próprio do produto respeita REQ-15 e TEC-RUST-01, incluindo interface e MCP.
- [ ] Build, testes e análise estática usam toolchain/lockfile e matriz pertinentes ao código entregue; mudança apenas documental identifica essa condição.
- [ ] Componentes externos, bindings, patches e fronteiras unsafe/FFI têm origem, função, licença e evidência conferidas quando aplicáveis.
