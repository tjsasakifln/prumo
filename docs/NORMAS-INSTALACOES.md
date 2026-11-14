# Base normativa das instalações do Prumo

Revisão 2.0, pesquisa de 14/11/2026. Este documento amplia o [registro normativo](NORMAS.md) para água, esgoto, pluvial, gás, incêndio, elétrica, fotovoltaica, SPDA, comunicações e climatização. Define trabalho de implementação e qualificação. Os textos normativos completos não foram adquiridos ou conferidos nesta sessão, e nenhuma capacidade de cálculo foi validada.

Fontes locais: [hidrossanitário, gás e incêndio](fontes-instalacoes-hidrossanitario.json), [elétrica, fotovoltaica, SPDA, comunicações e climatização](fontes-instalacoes-eletrico.json) e [plataforma BIM](fontes-instalacoes-plataforma.json). Os identificadores QH, QE e QP abaixo permitem localizar título, URL, data e ressalva da evidência. Os requisitos de aceite e ensaio são propostas próprias do Prumo.

## 1. Classes e limite da evidência

| Classe | Evidência disponível | Consequência para a implementação |
| --- | --- | --- |
| A | Metadados de edição vistos diretamente no catálogo ABNT | Registrar referência pública; obter acesso autorizado ao texto, confirmar correções, vigência e aplicabilidade antes de implementar |
| C | Norma ou método identificado em manual oficial do fabricante | Usar como pista de abrangência funcional; conferir edição e texto na fonte normativa antes de definir a regra |
| P | Referência necessária pelo escopo, ainda sem edição confirmada nesta pesquisa | Manter pendência explícita; não preencher ano por memória, declarar conformidade ou concluir a função sem resolver |

Uma classe A não representa acesso ao texto integral nem validação da interpretação. Uma classe C não demonstra que o fabricante implementou toda a norma ou que sua documentação está atualizada. Um repositório aberto com testes próprios também não comprova atendimento às normas brasileiras.

As classes acima qualificam evidências de normas ABNT e técnicas correlatas. Para documentos de Corpos de Bombeiros, concessionárias, operadoras e municípios, a fonte de publicação é o próprio órgão competente. O portal foi identificado, mas cada instrumento e revisão ainda precisa ser selecionado conforme projeto, local e data.

## 2. Inventário por disciplina

### Água, esgoto, pluvial e tratamento

| Família | Referência pesquisada e classe | Aplicação e pendência principal |
| --- | --- | --- |
| NBR 5626 | 2020 identificada em manual; C, QH-004 e QH-006 | Água fria e quente, distribuição, pressões, reservação e interfaces de equipamentos. Confirmar versão consolidada e requisitos do método de demanda escolhido |
| NBR 7198 | Referência legada encontrada no catálogo funcional; C, QH-037 | Resolver no catálogo autorizado sua relação com a NBR 5626 antes de ativar regra de água quente. Não compor um pacote atual apenas somando referências antigas |
| NBR 15569 | 2021 citada pelo manual; C, QH-007 | Aquecimento solar térmico. Confirmar edição, método, dados de radiação e limites de aplicação |
| NBR 10339 | Edição a confirmar; P | Piscinas, sistemas hidráulicos e requisitos de segurança no domínio oferecido. A seleção de bomba/filtro documentada em QH-008 não comprova todos os requisitos dessa norma |
| NBR 8160 | 1999 citada pelos manuais; C, QH-009 e QH-010 | Ramais, tubos de queda, coletores, ventilação, desconectores e componentes sanitários. Qualificar as regras por tipo de trecho e ocupação |
| NBR 10844 | 1989 citada pelo manual; C, QH-012 | Captação, calhas e condutores pluviais. Validar separadamente métodos de trechos verticais, horizontais e seções abertas |
| NBR 15527 | Família identificada; edição a confirmar; C, QH-044 a QH-046 | Aproveitamento de chuva. Verificar admissibilidade e limites dos métodos de reservação, evitando tratar método histórico como prescrição universal vigente |
| NBR 16783 | Edição a confirmar; P | Fontes alternativas de água não potável, separação e interfaces entre redes, conforme o alcance implementado |
| NBR 17076 | 2024 identificada no manual; C, QH-013 | Tratamento local de esgoto. Confirmar limites, referências cruzadas e condições de implantação na edição licenciada |
| NBR 7229 e NBR 13969 | Referências legadas; C, QH-013, QH-048 e QH-049 | Resolver substituição/transição e métodos ainda admissíveis. O artigo de sumidouro ainda cita a 13969 de 1997, o que não autoriza defini-la como base vigente |
| Normas de tubulações, conexões, medidores, bombas e demais produtos | Partes/edições a selecionar por componente; P | Registrar material, classe, diâmetros internos, pressão, temperatura, curvas, certificados e aplicação real do produto |
| Regras ambientais, municipais e de concessionárias de água/esgoto | Instrumento por jurisdição a selecionar; P | Abastecimento, destino de efluentes, tratamento local, drenagem e água não potável. Exemplos municipais não autorizam solução universal |

A presença de um método no manual é evidência funcional. A regra do Prumo depende da edição licenciada, do método reconhecido e da hipótese registrada. Por exemplo, padrões de consumo, população e curvas de fabricantes precisam ter origem própria e não podem ser apresentados como valores universais da norma. [QH-004, QH-029, QH-042]

### Gás e incêndio

| Família | Referência pesquisada e classe | Aplicação e pendência principal |
| --- | --- | --- |
| NBR 15526 | 2012 citada pelos manuais; C, QH-015 e QH-016 | Redes GN, GLP e intercambiáveis. Confirmar edição/erratas atuais, pressão de aplicação, propriedades do gás e formulações antes de implementar |
| NBR 13523 | Edição a confirmar; P | Centrais GLP e condições de implantação; verificar relação com regras locais e limites dos produtos |
| NBR 13714 | Família declarada no dimensionamento de hidrantes; C, QH-018 e QH-037 | Hidrantes e mangotinhos, fontes, simultaneidade e limites de atendimento; conferir complementos locais |
| NBR 10897 | 2020 citada pelo manual de integração; C, QH-022 | Sprinklers, risco, área de operação, cálculo hidráulico/tabelas e redes compartilhadas, dentro de cada domínio |
| NBR 12693 | Edição a confirmar; P | Extintores, aplicação e cobertura dos requisitos implementados |
| NBR 17240 | Edição a confirmar; P | Detecção e alarme. A representação de sensores e acionadores não equivale à verificação completa do sistema |
| NBR 10898 | Edição a confirmar; P | Iluminação de emergência, conforme funções de modelagem, cálculo e verificação efetivamente qualificadas |
| NBR 16820 e correlatas de sinalização | Partes/edições e transições a confirmar; P | Sinalização de emergência e interfaces com requisitos locais |
| NBR 9077 | 2025 vista no catálogo; A, [ABNT](https://abntcatalogo.com.br/), QE-036 | Saídas e interfaces de rotas de fuga quando fizerem parte da verificação prometida. Modelar sinalização não comprova projeto integral de evacuação |
| Normas de aparelhos, mangueiras, esguichos, válvulas, bombas e componentes de gás | Selecionar por produto e uso; P | Compatibilidade de materiais, curvas, capacidades, conexão e limites de operação |
| Instruções de Corpos de Bombeiros e regras de concessionárias de gás | Instrumento/revisão por jurisdição a selecionar; P | Determinação de sistemas, risco, simultaneidade, reserva, implantação, distâncias e apresentação exigida |

O [portal de instruções do CBMSC](https://cbm.sc.gov.br/index.php/sci/instrucoes-normativas) e o [portal de resoluções do CBMRS](https://www.bombeiros.rs.gov.br/resolucoes-tecnicas) são fontes oficiais identificadas, QH-038 e QH-039. Cada projeto precisa apontar os documentos efetivamente aplicáveis; notícia de consulta pública ou minuta não equivale à regra vigente.

### Elétrica, fontes e fotovoltaica

| Família | Referência pesquisada e classe | Aplicação e pendência principal |
| --- | --- | --- |
| NBR 5410 | 2004, Versão Corrigida:2008 no catálogo; A, QE-036 | Instalações de baixa tensão, condutores, proteções, circuitos, alimentação e interfaces. Obter texto e qualificar cada regra implementada |
| NBR 17019 | 2022 vista no catálogo; A, [ABNT](https://abntcatalogo.com.br/), QE-036 | Alimentação de veículos elétricos; compatibilidade de circuitos, proteções e equipamento no domínio prometido |
| NBR 14039 | 2021 vista no catálogo; A, [ABNT](https://abntcatalogo.com.br/), QE-036 | Interface de média tensão. A representação de subestação por peças não pode aparecer como dimensionamento de média tensão concluído |
| IEC 60909 e eventual adoção brasileira pertinente | Método citado por biblioteca oficial; C, QE-047; edição/adoção a confirmar | Cálculo de curto-circuito, quando aplicável. A biblioteca não confere automaticamente conformidade NBR 5410 nem verifica todo sistema de proteção |
| NBR 16690 e NBR 16612 | Famílias citadas no catálogo funcional; C, QE-017, QE-019 e QE-027 | Circuitos fotovoltaicos CC/CA, cabos e proteções. Confirmar texto, edição e limites antes de selecionar dispositivos |
| Normas de conjuntos/barramentos blindados, dispositivos de proteção, geradores, QTA, nobreaks, baterias e inversores | Conjunto por componente a completar; P | Qualificar curvas, suportabilidade, características nominais, estados de operação e critérios que dependem de dados do fabricante |
| Regras de fornecimento, conexão e compensação de energia | Publicações vigentes da distribuidora e autoridade competente a selecionar; P | Entrada individual/coletiva, geração conectada, parâmetros econômicos e documentação. Perfil de usina não comprova todo estudo de conexão |

Não confundir potência instalada, demanda, potência térmica, corrente por fase e consumo energético. Os dados trocados entre instalações e estrutura conservam unidade, cenário, origem e revisão. A condição apenas modelada dos componentes de subestação deve permanecer visível em diagramas, IFC e memoriais. [QE-006, QE-010, QE-014, QE-020]

### SPDA e comunicações

| Família | Referência pesquisada e classe | Aplicação e pendência principal |
| --- | --- | --- |
| NBR 5419, partes 1 a 4 | 2026, Versão Corrigida:2026, vistas individualmente no catálogo; A, QE-036 | Risco, medidas de proteção, captação, descidas, aterramento, equipotencialização e proteção interna, conforme parte/método aplicável |
| NBR 14565 | Família citada no catálogo funcional; C, QE-027 | Cabeamento, redes e documentação no domínio declarado; confirmar edição e referências cruzadas |
| NBR 16415 | Edição e partes a confirmar; P | Caminhos e espaços de cabeamento, conforme aplicabilidade do texto autorizado |
| TIA/ISO e padrões de telefonia/operadoras | Edição por método a confirmar; C para referência histórica TIA-EIA568-B, QE-027 e QE-043; demais referências P | Compatibilidade, canais, pares, fibras, percursos, identificação e documentação. Não ativar padrão histórico como regra atual por constar em catálogo |
| Normas de cabos, conectores, racks, equipamentos e redes complementares | Selecionar por aplicação; P | A compatibilidade de desenho e portas não equivale à certificação de desempenho físico de uma rede construída |

### Climatização

| Família | Referência pesquisada e classe | Aplicação e pendência principal |
| --- | --- | --- |
| NBR 16655-3 | Família citada no artigo de carga térmica; C, QE-033 | Método de cálculo por ambiente. Confirmar edição e domínio; não estender automaticamente a todas as tipologias e portes |
| NBR 16655, demais partes pertinentes | Partes e edições a confirmar; P | Sistemas e interfaces de climatização previstos, incluindo instalação e dados necessários à verificação |
| NBR 16401 | Partes, edições e situação atual a confirmar; P; comunicado histórico em QE-054 | Condições de projeto e demais requisitos quando aplicáveis ao sistema oferecido. A suspensão divulgada para a Parte 3:2024 exige conferir a situação de cada parte no catálogo antes da implementação |
| Normas de refrigeração, fluidos, dutos, acessórios, água gelada, condensado e equipamentos | Conjunto por função a completar; P | Segurança, limites de pressão/temperatura, ventilação, materiais e condições específicas de fabricante |
| Métodos psicrométricos e de escoamento | Método, fonte, domínio e versão a qualificar; P | Estados do ar, carga latente, perdas, vazões e circuitos. Uma biblioteca de propriedades não dimensiona por si um sistema Split ou VRF |

## 3. Conflitos de fonte que precisam de tratamento explícito

### Referências hidráulicas e sanitárias antigas

QH-037 ainda relaciona NBR 7198 ao cálculo hidráulico, enquanto QH-004 e QH-006 usam NBR 5626 para água fria/quente. O pacote de regras deve conferir formalmente substituições e referências cruzadas. As hipóteses de métodos históricos podem ser conservadas como tal, com domínio e limitação; não podem assumir aparência de exigência normativa atual.

QH-013 cita NBR 17076:2024 para tanque séptico, enquanto QH-049 conserva NBR 13969/1997 no sumidouro. A [orientação municipal identificada](https://devportal.pmf.sc.gov.br/entidades/smdu/index.php?cms=exemplos+de+phs&menu=9&submenuid=dyn-2893), QH-040, registra atualização dos exemplos. Antes de concluir as issues 153 a 155, é necessário identificar o texto aplicável, a transição, as condições de solo e destino e as regras locais. O programa não pode combinar partes incompatíveis de edições diferentes em uma mesma aprovação.

### SPDA 2026

O catálogo exibe separadamente as quatro partes da NBR 5419 com edição e versão corrigida de 2026. O [manual de atualização](https://suporte.altoqi.com.br/hc/pt-br/o-que-mudou-no-builder-com-a-nbr-54192026-para-projetos-de-spda), QE-022, documenta mudança de parâmetros, dados e resultados de risco. QE-048 contém metodologia multizona vinculada a edição anterior. A issue 201 e a qualificação de SPDA devem implementar a edição selecionada a partir do texto autorizado, com casos próprios, preservação do projeto anterior e migração explícita.

Dados por localidade precisam de origem, data, licença e resolução espacial. Não se pode reutilizar automaticamente a mesma tabela de uma edição anterior nem atribuir dados de um município a outro. Dados geométricos de armadura, telhado e conexões não comprovam continuidade elétrica executada, que depende também de especificações e verificação profissional. [QE-022, QE-024, QE-025]

### Climatização: modelagem e cálculo

QE-032 descreve um escopo anterior de modelagem, enquanto o [artigo de carga térmica](https://suporte.altoqi.com.br/hc/pt-br/c%C3%A1lculo-da-carga-t%C3%A9rmica-por-ambientes-no-altoqi-builder), QE-033, registra cálculo por ambiente disponível a partir da atualização 2026-08. Portanto, esse cálculo faz parte da cobertura obrigatória do Prumo.

QE-034 anuncia dimensionamento de dutos e redes que não é inteiramente demonstrado pelos materiais anteriores. O requisito permanece no planejamento, com método próprio qualificado. Confirmar posteriormente a extensão comercial da referência não dispensa a implementação e evidência assumidas no Prumo. Carga térmica calculada não aprova automaticamente um equipamento: capacidade disponível, condições de seleção e limites de fabricante são verificações próprias. [QE-033 a QE-035]

O [comunicado do CB055 publicado pela ABRAVA](https://abrava.com.br/abnt-cb055-informa-a-suspensao-da-abnt-nbr-16401-32024-confira-o-comunicado/), QE-054, informa suspensão da NBR 16401-3:2024. O [índice oficial do comitê](https://abrava.com.br/category/noticias/noticias-do-comite-cb-55-abnt/) registra a notícia em 28/05/2025. Esse fato histórico não confirma a situação em 14/11/2026 nem a edição aplicável das demais partes. O pacote de climatização deve resolver cada parte, vigência e eventual transição separadamente; não adotar um mesmo ano para toda a série por inferência.

## 4. Registro versionado por projeto e jurisdição

Além do pacote normativo definido em NORMAS.md, cada projeto de instalações registra:

- Local, autoridade competente, concessionárias/operadoras e condições de conexão.
- Código/parte, edição, correção, vigência, transição, fonte oficial e data de conferência de cada instrumento.
- Aplicabilidade por ocupação, porte, risco, sistema, combustível, tensão e regime de operação.
- Origem dos dados de demanda, clima, solo, catálogos e ensaios; domínio e licença de uso.
- Critério escolhido, conflito de interpretação, decisão do responsável e evidência associada.

Um pacote novo não substitui silenciosamente o pacote do projeto. A migração apresenta diferenças de entrada, método e resultado, exige recalcular as capacidades afetadas e invalida os documentos correspondentes. Uma regra manualmente ajustada conserva a fonte, o valor original e a justificativa técnica. A alteração de parâmetros não pode servir para ocultar reprovação.

O catálogo de fontes locais deve distinguir publicação vigente, texto histórico, minuta, manual de aplicação e exemplo. Perfis de concessionárias precisam de região e validade; a escolha automática só é permitida quando seus pressupostos são demonstrados. Quando a informação necessária estiver ausente, o estado será pendente, nunca aprovado por padrão.

## 5. Qualificação e condições de encerramento

Antes do aceite de uma função de instalações, deve existir ligação verificável entre requisito, texto autorizado, interpretação própria, implementação, caso independente, tolerância, evidência e versão do programa. Para cada regra relevante, incluir aplicação válida, rejeição e fronteira de regime ou tabela. As listas abaixo indicam riscos a cobrir, sem reproduzir fórmulas ou limites protegidos:

| Domínio | Evidências que precisam revelar erros relevantes |
| --- | --- |
| Hidráulico e gás | Conservação de massa/energia, pressão absoluta/manométrica quando pertinente, regime, propriedades, perdas, operação de bombas/reguladores e limites de peças |
| Sanitário e pluvial | Contribuição, topologia, ventilação, caimento, nível, método por trecho, destino, condições de solo e balanço de reservação |
| Incêndio | Cenários simultâneos, terminais críticos/favoráveis, alcance real, volume útil, áreas de operação, alimentação conjunta e limites de cada jurisdição |
| Elétrico e fotovoltaico | Demanda, corrente por fase, capacidade corrigida, queda, faltas, atuação/interrupção, faixas de inversor, proteção CC/CA e estados de alimentação |
| SPDA | Edição correta, dados de localidade, risco, cobertura geométrica, continuidade especificada, distância/aterramento/equipotencialização e interfaces com proteção interna |
| Comunicações | Origem/destino, portas, meios físicos, compatibilidade, comprimentos, ocupação e distinção de projeto e certificação em campo |
| Climatização | Cargas sensível/latente, unidades, clima/altitude, estados psicrométricos, método de redes, limites de equipamento e seleção explicitamente verificada |

Os casos comparativos precisam de entradas e referência preparadas de modo independente. EPANET usado através de WNTR não constitui um segundo oráculo independente do mesmo motor; o mesmo princípio vale para outras bibliotecas. Comparação com software comercial é evidência auxiliar, sujeita às mesmas hipóteses de entrada, edição, domínio e licença. [QH-031, QH-032, QE-037 a QE-041]

As campanhas intermediárias usam relatórios nativos do domínio já entregue. Documentos finais, compatibilidade entre disciplinas, responsabilidade profissional e pacote de distribuição são conferidos novamente na integração e na qualificação final. Uma representação BIM, uma planilha preenchida ou uma issue fechada não substituem a prova correspondente.

Nenhuma pendência de edição, acesso autorizado, método, referência independente ou revisão técnica é considerada resolvida pela publicação deste plano. Quando bloquear uma capacidade obrigatória, essa pendência impede seu aceite e, por consequência, o lançamento completo definido na issue 250.
