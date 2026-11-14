# Fixtures de incompatibilidade

`future-format.prumo` é um ZIP clássico mínimo, sem compressão, produzido com
`zipfile` da biblioteca padrão do Python, independente do escritor Rust do
produto. Contém apenas `manifest.json` com `format_version: 2` e um campo novo.
A leitura deve retornar versão incompatível antes de interpretar o restante do
manifesto ou exigir o banco. Não representa um projeto futuro implementado.

Data ZIP fixa: 01/01/1980. Modo de entrada: arquivo regular, 0600. Nenhum dado de
usuário, cálculo, norma licenciada ou autorização está presente. Os testes de
armazenamento, interface e MCP compartilham esses bytes.
