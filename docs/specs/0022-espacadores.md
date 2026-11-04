# Spec 0022 — Espaçadores no dock

Item F2 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md), adiado na [0013](0013-fase-c-espacos-de-trabalho.md) §5 e na [0017](0017-fase-f-widgets.md) §5. Como no PrimoDock ([referência](../reference/primodock/interface.md)): espaços vazios entre os itens do dock, pequenos ou grandes, **por espaço de trabalho**, para separar o dock em grupos.

## 1. O que é um espaçador

- Um item do espaço de trabalho, chamado `spacer:<id>` na lista do espaço (`workspace_items`), como `app:<id>` e `widget:<id>`. O id é um UUID v7, como o de um widget.
- **Pertence a um espaço só**, o em que foi criado. Sair dele é ser apagado: não há "compartilhar" espaçador, e apagar o espaço apaga os espaçadores dele.
- Dois tamanhos, medidos ao longo da barra, em relação ao ícone do tamanho em uso (`SpacerSize::length`):

  | Tamanho | Espaço | Pequeno (40) | Médio (48) | Grande (64) |
  | --- | --- | --- | --- | --- |
  | Pequeno | ¼ de ícone | 10 px | 12 px | 16 px |
  | Grande | ½ ícone | 20 px | 24 px | 32 px |

  Mais os dois vãos (`gap`) que todo item tem dos lados. O comprimento é ao longo da barra: hoje o dock só fica em cima ou embaixo (`DockEdge::Top`/`Bottom`); um dock lateral (F12) usaria o mesmo número como altura.

## 2. Invisível

- Não desenha nada: sem vidro, sem marca de aberto, sem selo, sem dica (tooltip).
- O ponteiro nunca "pousa" nele (`dock_layout::hit`): não amplia, não é clicado, não abre prévia. Ele também **não cresce** quando o vizinho amplia, e assim interrompe a ampliação: o item do outro lado do espaçador não cresce como vizinho.
- Só aparece enquanto é arrastado: uma placa vazia do tamanho dele, para se ver o que se move.
- **Narrador (UI Automation):** fica de fora da árvore do dock (`publish` não o publica) e o teclado do dock (M14) passa por cima dele. Não é botão, e expô-lo como separador só acrescentaria uma parada muda.

## 3. Menus

- **Menu do dock** (botão direito no espaço vazio do dock ou num widget), logo depois de "Buscar apps…" e "Gerenciador de Tarefas": **"Adicionar espaçador pequeno"** e **"Adicionar espaçador grande"** (en: "Add small spacer", "Add large spacer"). Só no dock, que mostra um espaço; o menu do ícone da bandeja não os tem.
- **Onde o novo vai:** logo depois do item em que o menu abriu (um widget ou outro espaçador); aberto no espaço vazio (as pontas da barra, os separadores), no fim do espaço (`spacer_place`). O botão direito num programa abre o menu do programa, que não tem a entrada: põe-se o espaçador no fim e arrasta-se.
- **Menu do espaçador** (botão direito em cima dele, que acha o espaçador por `spacer_hit`): **"Remover espaçador"** (en: "Remove spacer"), e depois o menu do dock, como o "Remover do dock" de um widget.

## 4. Arrastar

- O espaçador é arrastado como um widget: **dentro do primeiro grupo** (programas fixados, widgets e espaçadores, misturados). Não entra no grupo dos programas abertos nem passa pelos separadores; a ordem nova vai para o espaço mostrado (`reorder_workspace`), como a de qualquer item.
- Arquivos soltos no dock e fixados ficam antes do item mais perto, e esse item pode ser um espaçador.

## 5. Configurações › Itens do dock

- A lista mostra os widgets e os espaçadores do espaço, **misturados na ordem do dock**: "Espaçador pequeno" e "Espaçador grande", com um contorno tracejado no lugar do mini ladrilho.
- Subir e descer andam entre os itens da lista (widgets e espaçadores contam como um tipo só, em `workspaces::shifted`); os programas continuam andando só entre os programas, em Apps do dock.
- Remover um espaçador é na hora (não há o que perder). Comandos novos: `remove_spacer { id, workspace? }` e `move_spacer { id, direction, workspace? }`, na janela de Configurações; recusas vêm como as dos espaços (`{ code }`) e mostram a mensagem do código.
- Não há botões de adicionar espaçador em Configurações (o PrimoDock tem): por ora, só pelo menu do dock.

## 6. Banco

- **Migração 5** (`migrations/0005_spacers.sql`): a tabela `spacers` (`id`, `size` com `CHECK` em `small`/`large`, `created_at`), STRICT como as outras. Um banco da versão 4 ganha a tabela vazia e mantém os espaços como estavam; o backup `dockmax.db.bak-v4` é feito antes, como em toda migração.
- O lugar do espaçador é o da sua chave em `workspace_items`; `remove_spacer` apaga a linha e a chave numa transação.
- Memória: um `Option<SpacerSize>` por item na forma do dock e um `Spacer` (id e tamanho) por espaçador. Nenhum bitmap, nenhuma leitura extra.

## 7. Verificação ao vivo: parcial

Rodada de 2026-11-03, no build de lançamento com uma cópia dos dados do autor e o dock embaixo, centralizado:

- "Adicionar espaçador grande" no menu do Calendário abriu um vão logo depois dele. Nas fotos, a distância entre o Calendário e o Ciclo passou de ~7 px para ~33 px (½ ícone). Como o dock é centralizado, o Ciclo andou só +13 px: a outra metade foi para a esquerda.
- A árvore de UI Automation continuou com 19 botões: o Narrador não vê o vão.
- O botão direito no vão mostrou "Remover espaçador", e o item voltou ao lugar exato (x=1373).
- A barra do Windows ficou como estava. O log do app não foi lido nesta rodada: o roteiro abria o primeiro `*.log` da pasta, que era um arquivo do WebView2, e não `logs\dockmax.log` (achado em 2026-11-04, já corrigido nos roteiros). O log fica para a próxima conferência.

Ainda falta conferir à mão os passos 2, 3, 5, 6 e 7 abaixo e o espaçador pequeno.

Roteiro curto (build e2e com cópia dos dados):

1. Botão direito no espaço vazio do dock › "Adicionar espaçador pequeno": aparece um vão de ¼ de ícone no fim do dock. Repetir com "grande" num widget: o vão de ½ ícone aparece logo depois do widget.
2. Passar o ponteiro sobre o vão: nada amplia nem mostra dica; o vizinho ampliado não empurra o vão.
3. Arrastar o vão para entre dois programas: a placa vazia acompanha o ponteiro e o vão fica onde foi solto; trocar de espaço e voltar: continua lá, e o outro espaço não o tem.
4. Botão direito no vão › "Remover espaçador": some.
5. Configurações › Itens do dock: os espaçadores aparecem entre os widgets; subir, descer e remover funcionam e o dock acompanha.
6. Narrador com o atalho do dock: as setas passam de um item ao outro sem parar no vão.
7. Nas bordas de cima e de baixo, nos três tamanhos.
