# Spec 0026: Exportar e importar as configurações

F8 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md). Leva o dock montado para outro computador, ou guarda um ponto de volta antes de experimentar. Antes, o único jeito era copiar a pasta de dados com o app fechado ([limitações](../limitacoes.md)).

## 1. Onde fica

Fica em Configurações › Sobre, no bloco "Cópia das configurações". A janela de Configurações não fecha quando o diálogo do Windows pega o foco, e o painel Sobre do dock fecha. Por isso a confirmação da importação mora em Configurações.

- **"Exportar…"** abre o "Salvar como" do Windows e grava `Dockyard-<data>.dockyard`. Depois diz "Cópia exportada.".
- **"Importar…"** abre o "Abrir" do Windows, lê o arquivo e o confere. Em seguida mostra o que ele traz: a data, a versão do Dockyard que gravou, e quantos espaços, apps, widgets e ícones próprios. Ao lado ficam "Importar e reiniciar" e "Cancelar". Até aqui nada muda.
- **"Importar e reiniciar"** deixa a importação esperando e reinicia o Dockyard. Ele inicia um Dockyard novo com `--depois-de <pid>` e sai pelo mesmo caminho do "Sair", que devolve a barra. O novo espera o antigo terminar antes de qualquer coisa. O `restart()` do Tauri não serve aqui: chamado da linha principal, ele inicia o novo com o antigo ainda vivo. No primeiro teste ao vivo, o Dockyard novo ficou sem dock.

Rust mostra os diálogos e lê e grava os arquivos. A página nunca recebe nem manda um caminho.

## 2. O arquivo

O `.dockyard` é um JSON (formato 1) com estes campos:

- `format`;
- `app`: a versão do Dockyard que gravou;
- `exportedAt`;
- `schema`: o `user_version` do banco;
- `settings`: as configurações em uso;
- `database`: o banco SQLite inteiro, em base64. Ele traz espaços, apps fixados, widgets, notas, espaçadores, pilhas e o histórico do lançador. É uma cópia consistente feita com `VACUUM INTO`, sem o `-wal`;
- `appIcons`: as imagens escolhidas como ícone (spec 0024), em base64.

**Não vão na cópia:**
- os tokens das contas Google, que ficam no Gerenciador de Credenciais do Windows. Depois de importar, é preciso conectar a conta de novo;
- os pacotes de ícones e as skins instalados, que podem ser grandes e se instalam de novo.

## 3. A conferência

Um arquivo só é aceito se passar por tudo isto:

- **Tamanho:** até 32 MB.
- **JSON:** precisa ser um JSON deste formato.
- **Versão:** `format` e `schema` não podem ser maiores que os deste Dockyard. Cópia de um Dockyard mais novo pede para atualizar antes.
- **Banco:** o banco abre de verdade numa pasta de rascunho. Um banco de versão antiga passa pelas migrações ali; se o SQLite o achar corrompido, a cópia é recusada.
- **Configurações:** são lidas sem cair nos padrões.
- **Ícones:** cada ícone tem nome de imagem escolhida (`<uuid>.png`), até 2 MB e cabeçalho PNG.

Qualquer falha diz o motivo e deixa tudo como estava.

## 4. A importação na abertura

A importação espera em `Importacao-pendente`, na pasta de dados. Ela é escrita à parte e depois renomeada, então entra inteira ou não entra.

Na abertura seguinte, antes de ler as configurações e o banco, o app faz esta troca:
- move `dockmax.db` (com `-wal` e `-shm`), `settings.json` e `AppIcons` para `Antes-de-importar-<hora UTC>`;
- põe os arquivos importados no lugar.

Se uma das trocas falhar, tudo volta como estava e o log registra. Um banco de versão antiga é migrado como em qualquer abertura, com a cópia `bak-v<n>` de sempre.

## 5. Verificação

**Testes de unidade (`persistence/transfer.rs`):**
- ida e volta dão os mesmos widgets, os mesmos ícones e configurações legíveis, sem sobrar rascunho;
- o que estava antes vai para `Antes-de-importar-…`;
- arquivo que não é JSON, schema ou formato mais novo, e banco que não é banco: nada muda;
- uma cópia de schema 5 é aceita;
- um ícone com nome fora do padrão (`../dockmax.db`) é recusado.

**Ao vivo: feita em 2026-11-04**, com o build de lançamento `.medir` e uma cópia dos dados do autor:

- "Exportar…" gravou a cópia, de 119 KB e schema 6, sem tokens. Sobre disse "Cópia exportada.".
- O FileZilla foi tirado do dock.
- "Importar…" mostrou o resumo, e "Importar e reiniciar" funcionou: o Dockyard antigo saiu em uns 15 s e o novo, que esperou por ele, mostrou o dock em uns 2 s.
- O FileZilla voltou para o dock. O que havia antes ficou em `Antes-de-importar-<hora>` e não sobrou importação pendente.
- A barra do Windows ficou como estava.

Nas duas primeiras rodadas, o `restart()` do Tauri e depois a saída pedida da linha principal deixaram dois Dockyards vivos ao mesmo tempo, e o novo ficou sem dock. Isso foi corrigido em §1.

Ainda falta conferir à mão: importar num computador sem dados do Dockyard, e importar uma cópia com ícones próprios.
