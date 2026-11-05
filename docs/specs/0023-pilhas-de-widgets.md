# Spec 0023 — Pilhas de widgets

Item F3 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md), adiado na [0013](0013-fase-c-espacos-de-trabalho.md) e na [0017](0017-fase-f-widgets.md) §5. Como no PrimoDock: dois ou mais widgets num só ladrilho do dock. A roda com Ctrl troca o widget de cima, e pontinhos mostram a posição.

## 1. O que é uma pilha

- Um item do espaço de trabalho, chamado `stack:<id>` na lista do espaço (`workspace_items`), como `widget:<id>` e `spacer:<id>`. O id é um UUID v7.
- **Pertence a um espaço só**, como o espaçador (spec 0022): o do widget em que foi feita. Apagar o espaço apaga a pilha. Cada widget dela é apagado de vez se nenhum outro espaço o tiver.
- Guarda os widgets, em ordem, e qual está **em cima** (`top`). Tem de 2 a 8 widgets (`MAX_STACK`). Um widget que entra numa pilha sai da lista do espaço: ele passa a estar dentro dela.
- O mesmo widget pode estar numa pilha de um espaço e solto em outro espaço. Só é apagado de vez quando nenhum espaço o tem, nem solto nem numa pilha (`held`).

## 2. No dock

- A pilha ocupa o lugar e a largura do widget de cima, e se desenha como ele. O clique abre o painel dele, e o menu é o dele, com as entradas da pilha (§3).
- **Pontinhos:** uma fileira de pontos pequenos na borda de baixo do ladrilho, um por widget. O de cima fica aceso, na cor da marca do tema, e os outros ficam apagados. Isso diz que há mais coisa ali sem ocupar espaço.
- **Roda** em cima da pilha, com ou sem Ctrl: para baixo vai para o próximo, para cima volta ao anterior, dando a volta. A roda lateral (horizontal) faz o mesmo. Desde 2026-11-05 a roda só troca de espaço em cima das bolinhas (spec 0013 §3.2); antes, sem Ctrl, ela trocava de espaço também em cima da pilha.
- **Só o de cima lê dados:** Mídia, Atividade, Rede, Clima, Calendário, Ciclo e Claude só leem o sistema quando estão em cima. A pilha não gasta mais que um widget solto.
- **Narrador:** a pilha é um botão com o nome do widget de cima e a posição: "Nota, 2 de 3 na pilha". A roda na pilha não tem equivalente pelo teclado nesta versão.

## 3. Menus

No menu de um widget, ou de uma pilha, que é o menu do widget de cima:

- **"Empilhar com"** (en: "Stack with"): um título, como o de "Espaços de trabalho", e embaixo os outros widgets e pilhas do espaço mostrado, pelo nome. O menu do dock não tem submenus. Pilhas aparecem como "Pilha: Relógio, Nota". Ao escolher um, o widget ou a pilha do menu entra no escolhido, no lugar do escolhido, **e fica em cima**, para se ver o que mudou. Escolher uma pilha junta as duas. Sem outro widget no espaço, a entrada não aparece. Com a pilha cheia (8), a escolha é recusada e só vai para o log.
- **"Tirar da pilha"** (en: "Take out of the stack"), só numa pilha: o widget de cima sai e fica solto logo depois dela. Se só sobra um, a pilha se desfaz e ele fica no lugar dela.
- **"Remover"** de um widget empilhado tira só ele, da pilha e do espaço. Ele é apagado de vez só se nenhum outro espaço o tiver. Se a pilha fica com um só, ela se desfaz.

## 4. Configurações › Itens do dock

- A pilha aparece como uma linha só, "Pilha: Relógio, Nota", com o mini ladrilho do widget de cima. Ela sobe e desce entre os widgets e os espaçadores (`shifted`: pilha conta como widget).
- Botão **"Desfazer pilha"** (en: "Unstack"): os widgets voltam soltos no lugar dela, em ordem. Nada se perde.
- Comandos novos: `move_stack { id, direction, workspace? }` e `dissolve_stack { id, workspace? }`. Recusas vêm como as dos espaços (`{ code }`).
- Configurar um widget empilhado é pelo painel dele: clique na pilha com ele em cima.

## 5. Banco

- **Migração 6** (`migrations/0006_stacks.sql`), STRICT como as outras:
  - `stacks (id, top, created_at)`;
  - `stack_members (stack_id → stacks ON DELETE CASCADE, widget_id → widget_instances ON DELETE CASCADE, position)`.

  Um banco da versão 5 ganha as tabelas vazias e mantém o resto. O backup `dockmax.db.bak-v5` é feito antes, como em toda migração.
- Um widget apagado sai das pilhas pela cascata. Na leitura, uma pilha com um só widget aparece como pilha de um, sem pontinhos, e uma vazia é pulada. Tirar e remover desfazem a pilha na hora em que ela fica com um.
- Memória: um `StackItem` (id, widgets, de cima) por pilha mostrada. Nenhum bitmap a mais: só o de cima é desenhado.

## 6. Verificação ao vivo: parcial

Rodada de 2026-11-04, no build de lançamento com uma cópia dos dados do autor:

- Botão direito no Ciclo › "Calendário" (embaixo de "Empilhar com"): os dois viraram um ladrilho, no lugar do Calendário, com o Ciclo em cima. A árvore de UI Automation passou de 18 para 17 botões, e a pilha diz "Ciclo, 2 de 2 na pilha".
- Ctrl+roda para baixo em cima da pilha pôs o Calendário em cima ("Calendário, 1 de 2 na pilha"), e o ladrilho passou a ter a largura dele.
- "Tirar da pilha" soltou o Calendário e desfez a pilha: 18 botões de novo.
- A barra do Windows ficou como estava. O log do app não foi lido nesta rodada: o roteiro abria o primeiro `*.log` da pasta, que era um arquivo do WebView2, e não `logs\dockmax.log` (achado em 2026-11-04, já corrigido nos roteiros). O log fica para a próxima conferência.
- Os pontinhos aparecem nos dois ladrilhos. Com a marca branca do autor, o aceso quase não se distinguia dos outros: o de cima ficou maior (raio 2,25) e os outros menores e mais apagados. Essa troca ainda não foi vista ao vivo.

Ainda falta conferir à mão os passos 4 e 5 abaixo, e a roda sem Ctrl trocando de espaço.

Roteiro curto (build de lançamento com cópia dos dados):

1. Botão direito num widget › outro widget embaixo de "Empilhar com": os dois viram um ladrilho, com o que foi empilhado em cima e dois pontinhos. A árvore de UI Automation fica com um botão a menos.
2. A roda em cima, com ou sem Ctrl: troca o de cima, e os pontinhos acompanham. O espaço não muda.
3. "Tirar da pilha": o de cima volta solto logo depois e a pilha se desfaz.
4. Configurações › Itens do dock: a linha "Pilha: …" sobe, desce e se desfaz.
5. Mídia na pilha, embaixo: não lê (nenhum `media` no log de leitura) até ir para cima.
