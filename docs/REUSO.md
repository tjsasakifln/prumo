# Política de reuso e candidatos de componentes

**Estado:** política proposta e triagem documental, sem código incorporado nesta etapa. **Consulta de fontes primárias:** 14/11/2026. Requisito: incorporar componentes abertos pertinentes e validados para reduzir esforço total. A classificação abaixo é triagem de engenharia e licença declarada, não validação executada nem autorização jurídica ampla. Fixar commit e inspecionar os arquivos e dependências efetivamente distribuídos antes de adotar.

## Critério de escolha

Toda issue deve começar procurando componente já existente para a função. O agente deve registrar: candidatos consultados; capacidade reaproveitável; lacunas; resultados dos testes próprios; licença e obrigações; versão/commit; custo de integrar, manter e substituir; decisão adotar/adaptar/rejeitar com motivo. Não aceitar rejeição baseada apenas em preferência por reescrever. Também não aceitar adoção baseada em popularidade, estrelas ou reprodução de um único exemplo.

O padrão preferido é dependência fixada com adaptador pequeno e testes de contrato. Fork é indicado para correções necessárias, com histórico e plano de atualização. Extração de trechos preserva autoria, licença e proveniência. Cada release inclui inventário e avisos correspondentes. Não transformar licença copyleft em permissiva movendo o componente para outro processo.

## Compatibilidade com a implementação Rust

Todo código próprio do produto, inclusive interface e servidor MCP, será Rust. Componentes externos mantêm suas linguagens quando qualificados; adaptadores e orquestração próprios são Rust. Cada ficha deve distinguir produção, referência independente, ferramenta de apoio e cliente de teste. A origem, o escopo de patches e bindings e a fronteira FFI/processo/WASM seguem [RUST.md](RUST.md). Essa regra preserva o reuso sem autorizar módulos próprios em outras linguagens.

## Matriz inicial de 15 candidatos, com papéis revistos

| Candidato oficial | Licença observada | Uso possível no Prumo | Decisão proposta e prova exigida |
| --- | --- | --- | --- |
| [IfcOpenShell](https://github.com/IfcOpenShell/IfcOpenShell) | Bibliotecas LGPL-3.0-or-later; Bonsai, Bonsai Viewer e IfcSverchok GPL-3.0-or-later | IFC, geometria, autoria, propriedades, BCF/IDS, diferenças, quantitativos | Prioridade alta. Avaliar módulos específicos e empacotamento. Validar schemas, GUID, unidades, georreferenciamento, ida e volta e modelos hostis. Não importar todo monorepo como se tivesse licença única. |
| [web-ifc](https://github.com/ThatOpen/engine_web-ifc) | MPL-2.0 | Leitura/escrita IFC e geometria via WASM no visualizador | Prioridade alta para experimento. Medir importação, precisão e memória; limitar autoria a capacidades verificadas e manter avisos/obrigações dos arquivos alterados. |
| [That Open Components](https://github.com/ThatOpen/engine_components) | MIT no repositório consultado | Funções externas de seleção, cortes e coordenação BIM | Avaliar integração delimitada com a interface Rust; não adotar como base de autoria TypeScript do Prumo. Testar integração sem permitir que o modelo de visualização se torne uma segunda autoridade do projeto. Dependências conservam suas próprias licenças. |
| [Three.js](https://github.com/mrdoob/three.js) | MIT | Biblioteca externa de cena e renderização, quando pertinente | Comparar o custo de integração com gráficos Rust, preservando controle e interface próprios em Rust. Testes de grandes coordenadas, desempenho, recuperação de contexto gráfico e acessibilidade da interface que o envolve. |
| [MCP TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk) | Transição: Apache-2.0 para novas contribuições e partes relicenciadas; MIT para código anterior não relicenciado | Cliente externo de interoperabilidade ou referência de teste | Reclassificado pela decisão Rust; o servidor próprio avalia rmcp. Fixar versão compatível com revisão de protocolo escolhida. Testar clientes modernos e legado declarado; preservar ambas obrigações aplicáveis. Não descrever o SDK atual simplesmente como MIT. |
| [PyNite](https://github.com/JWock82/Pynite) | MIT | Análise de barras/placas em Python, protótipo ou referência independente | Alta prioridade para avaliação de solver inicial e oráculo independente. Verificar formulação, suporte de hipóteses, condicionamento, desempenho e benchmarks. Não assumir dimensionamento NBR. Empacotar Python se for adotado em runtime. |
| [Eigen](https://gitlab.com/libeigen/eigen) | Principalmente MPL-2.0, verificar módulos e arquivos escolhidos | Álgebra linear C++ como biblioteca externa acessada por Rust | Qualificar a fronteira de integração e o benefício sobre alternativas adequadas. Verificar pivotamento, diagnóstico de singularidade, estabilidade, backend e reprodutibilidade. Não se trata de solver estrutural completo. |
| [Open CASCADE Technology](https://github.com/Open-Cascade-SAS/OCCT) | LGPL-2.1 com exceção OCCT | Geometria B-rep, operações geométricas e interoperabilidade CAD | Prioridade alta se exigida edição geométrica robusta. Validar tolerâncias, booleanas, sólidos inválidos, conversões e redistribuição das bibliotecas. |
| [CGAL](https://github.com/CGAL/cgal) | GPL/LGPL v3+ por pacote/arquivo; alternativa comercial | Predicados robustos, interseções, triangulações e geometria computacional | Avaliar por pacote. Não adotar algoritmo GPL em distribuição incompatível por pressupor que todo CGAL é LGPL. Comparar custo de alternativa/comercial conforme licença final do Prumo. |
| [Gmsh](https://gitlab.onelab.info/gmsh/gmsh) | GPL-2.0-or-later com exceção de ligação; alternativa comercial | Geração e qualidade de malha | Avaliar benchmark de malha e licença de integração. A documentação oficial exige licença comercial para integrar partes em software fechado distribuído. Processo separado não elimina automaticamente obrigações. |
| [CalculiX](https://github.com/Dhondtguido/CalculiX) | GPL-2.0 no repositório; confirmar cabeçalhos e cláusula or-later no recorte adotado | MEF, estática, dinâmica e referência independente | Candidato forte para validação cruzada ou backend se modelo de distribuição for compatível. Verificar formulários de elementos, sinais, unidades, cargas e discretização, além do empacotamento. |
| [Code_Aster](https://gitlab.com/codeaster/src) | GPLv3 indicada pelo repositório; confirmar cabeçalhos e dependências por versão | MEF avançado, não linearidade, dinâmica e base de casos de verificação | Candidato forte de verificação/solver especializado. Custo de integração e instalação precisa ser medido. Avaliar Ifc2CA como adaptador inicial, sem presumir preservação integral das hipóteses. |
| [OpenSees](https://github.com/OpenSees/OpenSees) | Licença própria UC Regents com restrição à incorporação em produto de distribuição comercial | Não linearidade, dinâmica sísmica e comparação independente | Restrito até licenciamento comercial aplicável. A documentação pede autorização UC Berkeley para incorporação comercial; OpenSeesPy também declara necessidade para redistribuição e serviço comercial. Não classificar simplesmente como BSD/MIT. |
| [MOOSE](https://github.com/idaholab/moose) | LGPL-2.1 no repositório consultado; dependências separadas | Framework MEF/multifísica, modelos constitutivos e pesquisa avançada | Candidato secundário. Demonstrar que o benefício compensa dependências, compilação e esforço operacional; evitar trazer framework amplo para função já atendida por biblioteca pequena. |
| [xeokit SDK](https://github.com/xeokit/xeokit-sdk) | AGPLv3; alternativa comercial oferecida pelo mantenedor | Visualização BIM de modelos grandes | Alternativa condicionada a distribuição compatível ou licença comercial. Comparar com Three.js/ThatOpen no mesmo fixture. Não tratar AGPL como licença permissiva. |

### Submódulos de alto interesse já existentes

**IfcMCP:** a documentação atual do IfcOpenShell descreve servidor que expõe consultas e edição IFC, inclusive wrappers de IfcQuery e IfcEdit. Avaliar o componente externo para funções IFC delimitadas. O servidor próprio permanece Rust, com SDK qualificado e rmcp como candidato prioritário e seu adaptador Rust controla comandos autorizados, transacionais e rastreáveis. A integração não admite mutações genéricas irrestritas. Verificar licença específica do submódulo, revisão MCP e manutenção antes de incorporar. [Documentação](https://docs.ifcopenshell.org/ifcmcp.html)

**Ifc2CA:** já converte IFC para modelo analítico destinado ao Code_Aster. Usar como candidato para mapeamento e ensaio de referência, conferindo vínculos, offsets, cargas, unidades e perdas; não presumir que resolve dimensionamento brasileiro. [Documentação](https://docs.ifcopenshell.org/ifc2ca.html)

**IfcTester, IfcDiff e BCF:** avaliar reaproveitamento específico dentro do ecossistema IfcOpenShell antes de implementar versões próprias dessas utilidades. [Introdução oficial](https://docs.ifcopenshell.org/introduction)

## Sequência para obter benefício real

1. Na primeira issue, avaliar prioritariamente o SDK MCP Rust rmcp e candidatos BIM pertinentes para uma operação real pela interface Rust e pelo MCP; registrar adoção ou rejeição concreta. Qualificar cada componente usado antes da incorporação.
2. Na fundação de visualização, comparar interfaces com autoria Rust e gráficos adequados, incluindo egui/eframe, Dioxus e wgpu. Bibliotecas externas de BIM/visualização podem integrar o experimento, com o mesmo fixture e métricas de memória, precisão, acessibilidade e resposta.
3. Antes do solver próprio, comparar PyNite, biblioteca linear e pelo menos um motor independente adequado. Investir em regras normativas, modelo físico/analítico e experiência, sem reescrever álgebra ou parser já validados sem justificativa.
4. Nos módulos avançados, avaliar componentes adicionais por hipótese e tipo de elemento. O histórico de validação do fornecedor é insumo; os testes de integração do Prumo continuam obrigatórios.
5. Manter pelo menos um conjunto de oráculos cuja implementação e preparação de entradas sejam independentes do núcleo adotado. Se PyNite virar motor de produção, os próprios resultados dele deixam de ser oráculo independente.

## Fontes de licença e escopo

O backlog também menciona opções ainda sem ficha documental completa neste catálogo: Tauri, SuiteSparse, SQLite, sectionproperties, ezdxf, PDF.js, OpenCV, CALFEM, ARPACK, SLEPc, LibreDWG e Tauri updater. Seu estado é **pesquisa e qualificação pendentes**, conforme [registro de pendências](../planning/reuse-pending.json). React permanece registrado como alternativa anterior não selecionável para autoria própria da interface. Tauri e seu atualizador dependem da escolha de um contêiner compatível com autoria Rust. Nenhum nome autoriza incorporação antes de examinar fontes oficiais, licença do recorte, versão, dependências e testes. Submódulos do IfcOpenShell também exigem conferência específica por commit.

As fontes primárias e referências de pesquisa estão em [candidatos-reuso.json](candidatos-reuso.json). São fontes de triagem; a issue de adoção deve registrar commit imutável e guardar apenas os materiais cuja redistribuição seja permitida.

## Ampliação para instalações

A revisão 2.0 acrescenta 13 candidatos pesquisados para redes de água/gás, hidrologia, elétrica, geração fotovoltaica, propriedades térmicas, grafos e diagramas. A [análise complementar](REUSO-INSTALACOES.md) detalha o uso proposto e os limites; o [catálogo estruturado](candidatos-reuso.json) passou a conter 28 candidatos nessa revisão. Todos permanecem sujeitos à qualificação por módulo, licença e commit.

As fontes primárias adicionais constam dos catálogos de instalações. Uma ferramenta genérica de cálculo não transfere automaticamente conformidade às normas brasileiras. O núcleo comum de UI/MCP, a rastreabilidade e a independência das referências continuam obrigatórios em cada nova issue.

## Candidatos acrescentados pela decisão Rust

A revisão 2.1 acrescenta quatro fichas, totalizando 32 candidatos documentados. Permanecem em triagem, sem aprovação técnica de incorporação.

| Candidato | Uso proposto | Licença observada e qualificação |
|---|---|---|
| [MCP Rust SDK rmcp](https://github.com/modelcontextprotocol/rust-sdk) | Servidor MCP próprio desde a primeira entrega | Transição Apache-2.0/MIT indicada no arquivo oficial. Conferir recorte e avisos; fixar versão, features, protocolo e clientes reais |
| [egui/eframe](https://github.com/emilk/egui) | Interface de autoria Rust em destinos nativos e web | MIT ou Apache-2.0 declaradas; conferir também fontes e recursos. Ensaiar CAD/BIM, teclado, leitor de tela, texto, impressão e múltiplas vistas |
| [Dioxus](https://github.com/DioxusLabs/dioxus) | Alternativa de interface com autoria Rust | MIT ou Apache-2.0 declaradas. Distinguir WebView e renderizadores experimentais; qualificar gráficos e distribuição |
| [wgpu](https://github.com/gfx-rs/wgpu) | Renderização gráfica Rust | Arquivos MIT e Apache-2.0 observados; conferir licenças por crate e dependência. Ensaiar backends, precisão, limites e recuperação gráfica |

As fontes primárias estão em [fontes-rust.json](fontes-rust.json), incluindo o arquivo de licença do rmcp. Frameworks e versões serão selecionados pelas evidências exigidas nas issues 1 e 3.
