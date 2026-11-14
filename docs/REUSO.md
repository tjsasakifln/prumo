# Política de reuso e candidatos de componentes

**Estado:** política proposta e triagem documental, sem código incorporado nesta etapa. **Consulta de fontes primárias:** 14/11/2026. Requisito: incorporar componentes abertos pertinentes e validados para reduzir esforço total. A classificação abaixo é triagem de engenharia e licença declarada, não validação executada nem autorização jurídica ampla. Fixar commit e inspecionar os arquivos e dependências efetivamente distribuídos antes de adotar.

## Critério de escolha

Toda issue deve começar procurando componente já existente para a função. O agente deve registrar: candidatos consultados; capacidade reaproveitável; lacunas; resultados dos testes próprios; licença e obrigações; versão/commit; custo de integrar, manter e substituir; decisão adotar/adaptar/rejeitar com motivo. Não aceitar rejeição baseada apenas em preferência por reescrever. Também não aceitar adoção baseada em popularidade, estrelas ou reprodução de um único exemplo.

O padrão preferido é dependência fixada com adaptador pequeno e testes de contrato. Fork é indicado para correções necessárias, com histórico e plano de atualização. Extração de trechos preserva autoria, licença e proveniência. Cada release inclui inventário e avisos correspondentes. Não transformar licença copyleft em permissiva movendo o componente para outro processo.

## Matriz de 15 candidatos

| Candidato oficial | Licença observada | Uso possível no Prumo | Decisão proposta e prova exigida |
| --- | --- | --- | --- |
| [IfcOpenShell](https://github.com/IfcOpenShell/IfcOpenShell) | Bibliotecas LGPL-3.0-or-later; Bonsai, Bonsai Viewer e IfcSverchok GPL-3.0-or-later | IFC, geometria, autoria, propriedades, BCF/IDS, diferenças, quantitativos | Prioridade alta. Avaliar módulos específicos e empacotamento. Validar schemas, GUID, unidades, georreferenciamento, ida e volta e modelos hostis. Não importar todo monorepo como se tivesse licença única. |
| [web-ifc](https://github.com/ThatOpen/engine_web-ifc) | MPL-2.0 | Leitura/escrita IFC e geometria via WASM no visualizador | Prioridade alta para experimento. Medir importação, precisão e memória; limitar autoria a capacidades verificadas e manter avisos/obrigações dos arquivos alterados. |
| [That Open Components](https://github.com/ThatOpen/engine_components) | MIT no repositório consultado | Seleção, cortes, coordenação e componentes de interface BIM | Prioridade alta para reduzir trabalho de visualização. Testar integração sem permitir que o modelo de visualização se torne uma segunda autoridade do projeto. Dependências conservam suas próprias licenças. |
| [Three.js](https://github.com/mrdoob/three.js) | MIT | Cena, câmera, seleção e renderização 3D | Prioridade alta como base ou dependência do toolkit BIM. Testes de grandes coordenadas, desempenho, recuperação de contexto gráfico e acessibilidade da interface que o envolve. |
| [MCP TypeScript SDK](https://github.com/modelcontextprotocol/typescript-sdk) | Transição: Apache-2.0 para novas contribuições e partes relicenciadas; MIT para código anterior não relicenciado | Servidor MCP, schemas e transporte | Prioridade alta. Fixar versão compatível com revisão de protocolo escolhida. Testar clientes modernos e legado declarado; preservar ambas obrigações aplicáveis. Não descrever o SDK atual simplesmente como MIT. |
| [PyNite](https://github.com/JWock82/Pynite) | MIT | Análise de barras/placas em Python, protótipo ou referência independente | Alta prioridade para avaliação de solver inicial e oráculo independente. Verificar formulação, suporte de hipóteses, condicionamento, desempenho e benchmarks. Não assumir dimensionamento NBR. Empacotar Python se for adotado em runtime. |
| [Eigen](https://gitlab.com/libeigen/eigen) | Principalmente MPL-2.0, verificar módulos e arquivos escolhidos | Álgebra linear densa/esparsa em worker C++ | Prioridade alta se C++ for adotado. Verificar pivotamento, diagnóstico de singularidade, estabilidade, backend e reprodutibilidade. Não se trata de solver estrutural completo. |
| [Open CASCADE Technology](https://github.com/Open-Cascade-SAS/OCCT) | LGPL-2.1 com exceção OCCT | Geometria B-rep, operações geométricas e interoperabilidade CAD | Prioridade alta se exigida edição geométrica robusta. Validar tolerâncias, booleanas, sólidos inválidos, conversões e redistribuição das bibliotecas. |
| [CGAL](https://github.com/CGAL/cgal) | GPL/LGPL v3+ por pacote/arquivo; alternativa comercial | Predicados robustos, interseções, triangulações e geometria computacional | Avaliar por pacote. Não adotar algoritmo GPL em distribuição incompatível por pressupor que todo CGAL é LGPL. Comparar custo de alternativa/comercial conforme licença final do Prumo. |
| [Gmsh](https://gitlab.onelab.info/gmsh/gmsh) | GPL-2.0-or-later com exceção de ligação; alternativa comercial | Geração e qualidade de malha | Avaliar benchmark de malha e licença de integração. A documentação oficial exige licença comercial para integrar partes em software fechado distribuído. Processo separado não elimina automaticamente obrigações. |
| [CalculiX](https://github.com/Dhondtguido/CalculiX) | GPL-2.0 no repositório; confirmar cabeçalhos e cláusula or-later no recorte adotado | MEF, estática, dinâmica e referência independente | Candidato forte para validação cruzada ou backend se modelo de distribuição for compatível. Verificar formulários de elementos, sinais, unidades, cargas e discretização, além do empacotamento. |
| [Code_Aster](https://gitlab.com/codeaster/src) | GPLv3 indicada pelo repositório; confirmar cabeçalhos e dependências por versão | MEF avançado, não linearidade, dinâmica e base de casos de verificação | Candidato forte de verificação/solver especializado. Custo de integração e instalação precisa ser medido. Avaliar Ifc2CA como adaptador inicial, sem presumir preservação integral das hipóteses. |
| [OpenSees](https://github.com/OpenSees/OpenSees) | Licença própria UC Regents com restrição à incorporação em produto de distribuição comercial | Não linearidade, dinâmica sísmica e comparação independente | Restrito até licenciamento comercial aplicável. A documentação pede autorização UC Berkeley para incorporação comercial; OpenSeesPy também declara necessidade para redistribuição e serviço comercial. Não classificar simplesmente como BSD/MIT. |
| [MOOSE](https://github.com/idaholab/moose) | LGPL-2.1 no repositório consultado; dependências separadas | Framework MEF/multifísica, modelos constitutivos e pesquisa avançada | Candidato secundário. Demonstrar que o benefício compensa dependências, compilação e esforço operacional; evitar trazer framework amplo para função já atendida por biblioteca pequena. |
| [xeokit SDK](https://github.com/xeokit/xeokit-sdk) | AGPLv3; alternativa comercial oferecida pelo mantenedor | Visualização BIM de modelos grandes | Alternativa condicionada a distribuição compatível ou licença comercial. Comparar com Three.js/ThatOpen no mesmo fixture. Não tratar AGPL como licença permissiva. |

### Submódulos de alto interesse já existentes

**IfcMCP:** a documentação atual do IfcOpenShell descreve servidor que expõe consultas e edição IFC, inclusive wrappers de IfcQuery e IfcEdit. Isso deve entrar no experimento inicial de reuso. O adaptador do Prumo deverá substituir mutações genéricas irrestritas por comandos de domínio autorizados, transacionais e rastreáveis. Verificar licença específica do submódulo, revisão MCP e manutenção antes de incorporar. [Documentação](https://docs.ifcopenshell.org/ifcmcp.html)

**Ifc2CA:** já converte IFC para modelo analítico destinado ao Code_Aster. Usar como candidato para mapeamento e ensaio de referência, conferindo vínculos, offsets, cargas, unidades e perdas; não presumir que resolve dimensionamento brasileiro. [Documentação](https://docs.ifcopenshell.org/ifc2ca.html)

**IfcTester, IfcDiff e BCF:** avaliar reaproveitamento específico dentro do ecossistema IfcOpenShell antes de implementar versões próprias dessas utilidades. [Introdução oficial](https://docs.ifcopenshell.org/introduction)

## Sequência para obter benefício real

1. Na primeira issue, reusar o SDK MCP e testar IfcMCP/IfcOpenShell como candidato BIM para uma operação end-to-end; registrar eventual rejeição concreta.
2. Na fundação de visualização, comparar Three.js + That Open/web-ifc com a opção IFC nativa, usando o mesmo fixture e métricas de memória, precisão e resposta.
3. Antes do solver próprio, comparar PyNite, biblioteca linear e pelo menos um motor independente adequado. Investir em regras normativas, modelo físico/analítico e experiência, sem reescrever álgebra ou parser já validados sem justificativa.
4. Nos módulos avançados, avaliar componentes adicionais por hipótese e tipo de elemento. O histórico de validação do fornecedor é insumo; os testes de integração do Prumo continuam obrigatórios.
5. Manter pelo menos um conjunto de oráculos cuja implementação e preparação de entradas sejam independentes do núcleo adotado. Se PyNite virar motor de produção, os próprios resultados dele deixam de ser oráculo independente.

## Fontes de licença e escopo

O backlog também menciona opções ainda sem ficha documental completa neste catálogo: Tauri, React, SuiteSparse, SQLite, sectionproperties, ezdxf, PDF.js, OpenCV, CALFEM, ARPACK, SLEPc, LibreDWG e Tauri updater. Seu estado é **pesquisa e qualificação pendentes**, conforme [registro de pendências](../planning/reuse-pending.json). Nenhum desses nomes autoriza incorporação antes de examinar fontes oficiais, licença do recorte, versão, dependências e testes. Submódulos do IfcOpenShell também exigem conferência específica por commit.

As fontes primárias e referências de pesquisa estão em [candidatos-reuso.json](candidatos-reuso.json). São fontes de triagem; a issue de adoção deve registrar commit imutável e guardar apenas os materiais cuja redistribuição seja permitida.
