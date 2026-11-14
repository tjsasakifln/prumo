# Prumo

**Cálculo estrutural para o Brasil. BIM nativo. Modelagem por peças e conexões compreensíveis. Interface e agentes operando o mesmo projeto.**

O Prumo será um sistema profissional de concepção, análise, dimensionamento, detalhamento e documentação de estruturas. O objetivo é reunir a cobertura funcional pertinente documentada nos ecossistemas AltoQi Eberick, CYPECAD e TQS, com aprendizado reduzido, operação local e controle técnico verificável.

**Estado em 14/11/2026: repositório de especificação e planejamento. O software de cálculo ainda não está implementado.** Este repositório contém a sequência de 120 issues, requisitos, arquitetura proposta, fontes e critérios de lançamento. Nenhum resultado estrutural ou alegação de paridade foi validado nesta sessão.

## Começar pela próxima issue

A implementação segue **#1 até #120**, em ordem. A primeira entrega já precisa criar, salvar, reabrir e consultar um projeto BIM mínimo tanto pela interface quanto por MCP real. A última entrega libera o produto somente depois de demonstrar cobertura, correção, usabilidade e condições comerciais.

- [Fila de implementação](https://github.com/tjsasakifln/prumo/issues?q=is%3Aissue+is%3Aopen+sort%3Acreated-asc)
- [Issue #1: fundação executável, BIM e MCP](https://github.com/tjsasakifln/prumo/issues/1)
- [Issue #120: lançamento completo](https://github.com/tjsasakifln/prumo/issues/120)
- [Sequência com todas as 120 entregas](ROADMAP.md)
- [Instruções para agentes e implementadores](AGENTS.md)

Fechar uma issue exige código integrado, critérios atendidos e evidências adequadas ao domínio. Fechar todas por declaração, cancelar requisitos ou usar resultados simulados não satisfaz o lançamento.

## O que faz parte do produto

| Família | Abrangência planejada |
|---|---|
| Concreto | Vigas, pilares, lajes, paredes, escadas, punção, estabilidade, ELU/ELS, armaduras, protensão, pré-moldados e incêndio |
| Fundações | Sapatas, blocos, radiers, estacas, tubulões, contenções, recalques e interação solo-estrutura |
| Aço e mistos | Perfis, estabilidade, ligações, bases, chumbadores, estruturas mistas, tubulares, perfis delgados, fabricação e incêndio |
| Madeira | Caracterização, barras, estabilidade, ligações, coberturas, pisos, produtos engenheirados, detalhamento e incêndio |
| Sistemas complementares | Alvenaria estrutural, paredes moldadas no local, alumínio com base própria e análise de materiais genéricos |
| Análise | Ações e combinações, vento, sismo, dinâmica, segunda ordem, não linearidades, etapas e efeitos diferidos |
| BIM e entrega | IFC, BCF, IDS, CAD, desenhos, memoriais, quantitativos, revisões e pacote de emissão coerente |
| Agentes e experiência | MCP desde a primeira entrega, comandos transacionais, modelos paramétricos, programação visual e operação por uma mão ou teclado |

A [matriz de paridade](docs/PARIDADE.md) distingue o que foi documentado pelos fabricantes, a abrangência proposta e o que ainda precisa ser comprovado. A [matriz rastreável](planning/coverage.json) liga as famílias às issues de implementação e qualificação.

## Normas e confiabilidade

O núcleo abrange NBR 6118, NBR 8800, série NBR 7190 e correlatas. A pesquisa identificou no catálogo ABNT **NBR 6118:2026** e **NBR 8800:2024 Versão Corrigida:2025**. As demais referências têm nível de confirmação registrado, sem presumir vigência por um manual antigo. [Fonte oficial](https://www.abntcatalogo.com.br/).

Cada projeto preservará a edição, emendas, regras e métodos utilizados. A implementação de regras exige acesso autorizado ao texto técnico e interpretação revisada. Os textos integrais das normas não são redistribuídos neste repositório.

Confiabilidade será demonstrada por soluções analíticas, casos independentes, convergência, equilíbrio, fronteiras normativas, projetos completos e revisão profissional. Concordância entre dois programas é evidência complementar. Duas interfaces do mesmo motor não são duas referências independentes.

## Reutilização de código aberto

**Avaliar reuso é obrigatório em toda issue pertinente.** Candidatos pesquisados incluem IfcOpenShell, web-ifc, Three.js, SDK MCP, PyNite, Eigen, Open CASCADE e motores avançados. Nenhum está homologado para o Prumo apenas por aparecer na lista.

A adoção depende de licença e distribuição compatíveis, versão fixada, procedência, manutenção, segurança, testes próprios, domínio numérico comprovado e possibilidade de substituição. A [política de reuso](docs/REUSO.md) e o [catálogo de candidatos](docs/candidatos-reuso.json) registram as restrições encontradas.

## Documentação

| Documento | Finalidade |
|---|---|
| [PRD](PRD.md) | Objetivo, requisitos, produto e critérios de sucesso |
| [Roadmap](ROADMAP.md) | Ordem de implementação e entregas por fase |
| [Paridade](docs/PARIDADE.md) | Cobertura funcional e limites das fontes |
| [Normas](docs/NORMAS.md) | Edições, acesso e regras de conformidade |
| [Arquitetura](docs/ARQUITETURA.md) | Modelo, camadas, persistência e motores |
| [MCP](docs/MCP.md) | Contratos, permissões e operação por agentes |
| [Experiência de uso](docs/UX.md) | Modelagem intuitiva, acessibilidade e métricas |
| [Reuso](docs/REUSO.md) | Seleção e qualificação de componentes abertos |
| [Validação](docs/VALIDACAO.md) | Ensaios, tolerâncias, evidências e revisão |
| [Lançamento](docs/LANCAMENTO.md) | Condições obrigatórias para publicar a versão comercial |
| [Revisão do planejamento](docs/REVISAO.md) | Falhas encontradas e correções antes do registro das issues |

## Verificação deste planejamento

```bash
python3 scripts/validate_plan.py
```

O verificador confere sequência, dependências, cobertura, referências, textos das issues e consistência do registro publicado. Ele verifica o planejamento. Não calcula estruturas nem certifica o produto futuro.

## Direitos e procedência

As marcas dos sistemas de referência identificam as fontes pesquisadas e não sugerem afiliação. O desenvolvimento será próprio, com componentes cuja licença permita a forma de uso escolhida. A licença comercial do Prumo será definida antes da distribuição. As licenças e os avisos dos componentes incorporados deverão ser preservados.
