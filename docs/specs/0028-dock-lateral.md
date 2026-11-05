# Spec 0028: Dock na lateral

F12 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md). O estudo pedia "os widgets largos viram altos com o dock na esquerda ou na direita". Só que o Dockyard ainda não tem dock na lateral: só existem as bordas de baixo e de cima (`DockEdge { Bottom, Top }`). Por isso a F12 vira o dock lateral inteiro, e os widgets em pé são uma parte dele.

**Status:** aprovada em 2026-11-04, com as propostas do §8. Os oito passos do §5 foram feitos entre 2026-11-04 e 2026-11-05 (PRs #177, #179, #180, #182, #183, #185, #187 e o do passo 8). A pintura foi conferida fora da tela, com o mesmo código do dock, no passo 6 (as faces em pé) e no 7 (as skins). O roteiro ao vivo está em [roteiro-de-verificacao.md](../roteiro-de-verificacao.md) §8.

**Ao vivo, 2026-11-05.** Duas rodadas, sempre com uma cópia dos dados do autor (estilo Flutuante, barra do Windows oculta), na esquerda e na direita.

No build de teste (`dockyard-e2e.exe`), passaram:
- a janela é uma coluna encostada na borda e centrada na altura;
- a ordem: na esquerda de cima para baixo; na direita de baixo para cima, com o relógio no topo;
- a dica fica ao lado do ícone, do lado de dentro;
- o menu do botão direito abre do lado de dentro, centrado no ícone;
- arrastar o primeiro app para depois do segundo troca os dois, e arrastar de volta devolve a ordem.

No build de lançamento (`.medir`, que assume a barra do Windows como o do autor), passaram:
- **a faixa reservada fica do lado da coluna**: na esquerda, a área de trabalho começa em x=54, depois da coluna; na direita, termina em x=2506, antes dela;
- uma janela maximizada não fica por baixo da coluna;
- pelo atalho do dock, ↓ desce e ↑ sobe na coluna. Na direita, o primeiro item fica no pé da coluna, então lá ↓ não tem para onde ir;
- a prévia de um programa aberto abre do lado de dentro, na altura do item;
- o painel da Atividade abre ao lado da coluna, do lado de dentro;
- com ocultar automaticamente não há faixa: a coluna sai pela lateral com o ponteiro longe e volta ao encostar na borda;
- depois de fechar, a área de trabalho e a barra do Windows (estado, visível, sem `WS_EX_LAYERED|WS_EX_TRANSPARENT`) ficam como antes.

A bandeja em duas colunas, o relógio `hh:mm` sobre `dd/mm` e os widgets em pé saíram como nas fotos fora da tela.

Achado na rodada de lançamento: logo depois do teste, a barra do Windows do monitor principal estava sem o `WS_EX_TOPMOST` (estilo 0x80, antes 0x88).
- Um `SetWindowPos(HWND_TOPMOST)` era desfeito pelo Explorer na hora.
- Minutos depois, com outra janela na frente, ela estava de novo em 0x88, sem nada feito.
- O Dockyard pede o `HWND_TOPMOST` ao devolver a barra (`restore_file`). A explicação mais provável é o Explorer: ele tira o "sempre por cima" enquanto acha que há um programa em tela cheia, e a janela maximizada do roteiro cobria o monitor inteiro com a barra oculta.

Ficou por ver:
- as skins na lateral ao vivo (foram conferidas fora da tela);
- o dock em todos os monitores com a borda numa lateral.

## 1. O que muda para quem usa

Em Configurações › Geral › Dock › Posição, as opções passam a ser **Embaixo · Em cima · Esquerda · Direita**, como no PrimoDock ([spec 0004](0004-paridade-primodock.md) §3; [referência](../reference/primodock/interface.md), "Posições").

Com o dock na **esquerda**, ele vira uma **coluna**. De cima para baixo:
1. as bolinhas de espaço de trabalho, lado a lado;
2. o Lançador e os programas fixados;
3. os widgets, **em pé**;
4. os programas abertos;
5. no pé, a bandeja (em duas colunas), os ícones de status e o relógio, com a data em duas linhas curtas.

O indicador do programa aberto fica do lado da borda da tela.

Com o dock na **direita**, a ordem se inverte de cima para baixo, como o PrimoDock faz: relógio e bandeja no topo, Lançador e bolinhas no pé. O indicador fica do lado direito. (Ver Q2: espelho simples ou girado.)

O resto vale como nas bordas de hoje:
- a faixa reservada na lateral, para as janelas maximizadas não ficarem por baixo;
- ocultar automaticamente;
- um dock por monitor;
- tela cheia;
- arrastar para reordenar e soltar arquivos;
- menus, painéis, prévias e dicas abrindo para dentro da tela;
- teclado (setas para cima e para baixo) e Narrador.

## 2. Os widgets em pé

Um widget de 2 ou 3 ícones de largura vira 2 ou 3 ícones de **altura**. Hoje um tile é sempre quadrado (1 ícone) ou deitado (largura × 1). Na lateral ele fica com 1 ícone de largura e N de altura.

Para cada face, há duas saídas (Q3):
- **(a) Girar a face inteira 90°**, como o PrimoDock. O texto e o gráfico ficam de lado, e quem lê inclina a cabeça.
- **(b) Uma face própria em pé**, com o texto na horizontal. Por exemplo:
  - Relógio mundial: uma cidade por linha, em letras pequenas;
  - Atividade: barras verticais;
  - Calendário: blocos empilhados.

A (a) é uma transformação só, na pintura. A (b) é mais legível, mas pede um layout por face. A proposta é **(b) onde couber em uma linha curta, (a) no resto**:

| Face | Proposta |
|---|---|
| Relógio, Layout do teclado, Ciclo, Água, Pomodoro (1 ícone) | sem mudança: são quadrados |
| Progresso do tempo e Atividade (barras, anéis) | (b): as barras ou os anéis empilhados |
| Atividade de rede, gráfico do último minuto | (b): o gráfico em pé, com as taxas embaixo |
| Calendário | (b): os blocos empilhados, com o título em letreiro |
| Tocando agora | (b): a capa em cima, o título em letreiro embaixo; os botões numa coluna |
| Uso do Claude, Clima | (b): os anéis ou o céu em cima, os números embaixo |
| Relógio mundial | (b): uma cidade por ícone de altura, com a hora embaixo do nome |
| Lista de tarefas, Contagem regressiva | (a): o texto girado |

## 3. Onde mexe

Levantamento do código em 2026-11-04. O compilador ajuda: quase todo `match` sobre `DockEdge` é exaustivo, então acrescentar `Left` e `Right` aponta cada lugar. O difícil é o que não passa por `DockEdge` e só supõe o eixo x.

| Área | O que supõe hoje | Peso |
|---|---|---|
| **Layout** (`domain/dock_layout.rs`): `row`, `place_with`, `window_size` | Tudo acumulado em x. A barra tem a altura de um ícone. Os rótulos ficam acima ou abaixo. `Placement.separators` guarda só x, e `Area` tem só `center_x`. | Difícil. É a base de todo o resto. |
| **Ampliação e arrasto** (`dock_layout`: `column_at`, `drop_index`, `drag_pitch`, `make_room`; `native_dock/drag.rs`; `dock_items::dropped_before`; `drop_target.rs`) | 1-D em x. O `lift` é em y. | Médio. Basta trocar o eixo. |
| **Faixa reservada** (`windows/appbar.rs::place`) e a janela (`windows/dock.rs`: `docked_rect`, `dock_rect_with_margin`) | Só `ABE_BOTTOM` e `ABE_TOP`. A janela fica centrada em x, e a espessura vale para top e bottom. | Médio. |
| **Ocultar automaticamente** (`native_dock/autohide.rs`) e o recolhimento (`mod.rs`) | Faixa de 2 px em y, deslocamento só em y. | Fácil. |
| **Relógio** (`STATUS_WIDTH = 76`, `paint.rs::status`) | Hora e data numa caixa de 76 px, mais larga que um ícone (40 a 64 px). | Difícil. Precisa de outro desenho (Q7). |
| **Bandeja** (`tray_cells`, `strip.rs`) e **bolinhas de espaço** (switcher) | Duas linhas por coluna, avançando em x. As bolinhas ficam empilhadas numa faixa de 12 px. A seta da bandeja só aponta para cima ou para baixo. | Médio. |
| **Pintura da barra** (`paint.rs`: `bar`, `shelf`, `light_edge`, `reflection`, separadores, indicador `marks`/`Tile.dot`, `stack_dots`) | Gradiente de cima para baixo. Reflexo abaixo ou acima. O indicador guarda só x. O separador é uma linha vertical. | Médio. |
| **Dicas e letreiro** (`tip_anchor`, `tip_center`, `paint.rs::rolling`) | A dica fica acima ou abaixo, presa em x. O letreiro corta e desloca em x. | Médio (dica) e difícil (letreiro girado). |
| **Faces de widget** (`paint.rs`: Meters, Agenda, Calendar, Weather, Rates, Media; `faces.rs` por `config.width`; `wide_glaze`) | Medem tudo por `u = altura do tile` e decidem "largo" por `largura > u × k`. Num tile alto a escala explode, e o glaze largo só estica em x. Os botões de mídia têm alvo próprio (`media_buttons`). | Difícil, face por face (§2). |
| **Painéis e menus** (`windows/panel.rs::panel_rect`, `domain/menu.rs::place`, `native_dock/menus.rs`, `keyboard_menu_point`) | O painel fica acima ou abaixo, centrado em x. O menu da bandeja força `Bottom`. O CSS do painel anima em `translateY`. | Médio. |
| **Prévias e cartão do Calendário** (`domain/previews.rs::card`, `windows/previews.rs::slide_offset`, `windows/agenda.rs::place`) | Acima ou abaixo, deslizando em y. As setas do cartão são só para cima e para baixo. | Médio. |
| **Teclado** (`native_dock/keys.rs`, `previews::card_keys`) | As quatro setas andam na lista. A seta "rumo ao cartão" (F10) conflita com → num dock à esquerda. | Médio (Q8). |
| **Skins** (`domain/skin.rs::pieces`, `src-tauri/skins/`) | Três colunas cortadas em x. `slice.top` e `slice.bottom` precisam ser 0 (teste). A arte das 10 embutidas é deitada. | Difícil (§4). |
| **Configurações** (`src/core/settings.ts`, `SettingsPanel.tsx`, `ChoiceThumbs.tsx` `ScreenThumb`, i18n, `appearance.ts`) | Só `bottom` e `top`. A miniatura da tela tem o dock em linha. Os textos de ocultar dizem "desce" e "sobe". | Fácil. |
| **Multi-monitor** (`appbar.rs`) | Uma faixa lateral numa borda **interna** entre dois monitores é aceita pelo Windows, mas deixa o dock no meio da área estendida. A proteção contra o vai e vem de `ABM_SETPOS` foi pensada para cima e para baixo. | Médio (Q9). |
| **Testes** | Cerca de 40 testes fixam o horizontal: `dock_layout` (~15), `dock.rs` (~8), `panel_rect` (laços em `[Bottom, Top]`), `previews`, `agenda`, `autohide`, `keys`, `skin`, e um que **trava** `"left"` como inválido (`settings.rs`). | Ganham variantes nas quatro bordas. |

Sem dependência da borda: `monitors.rs`, `fullscreen.rs`, `popup_menu.rs`, `panel_window.rs`, `marquee` (é 1-D) e os hits 2-D (`drop_over`, `strip_hit`).

## 4. Skins de barra

As skins de 9 fatias da spec 0020 (`src-tauri/skins/`) são desenhadas deitadas: as pontas e o meio, numa faixa da altura da barra. Na lateral há três saídas (Q4):
- **girar a imagem** 90° (o desenho de tijolo ou de couro fica de lado);
- **pedir uma versão em pé** a cada skin (o dobro de arte);
- **cair no estilo sem skin** na lateral.

A proposta é **girar** nas embutidas, porque são simétricas o bastante, e as da pasta `Skins` (Q14) podem trazer `vertical` no `skin.json`.

## 5. Ordem dos PRs

Um PR por passo, cada um com teste ao vivo nas quatro bordas:

1. **Layout em dois eixos (domínio):** `dock_layout` passa a falar de "ao longo" e "através" em vez de x e y, com testes nas quatro bordas. Nada muda na tela.
2. **A janela e a faixa:** `DockEdge::Left` e `Right` nas configurações (escondidas da interface), a posição da janela por monitor, a faixa reservada na lateral e ocultar automaticamente.
3. **A pintura da coluna:** a barra, os ícones, o indicador, os separadores, a ampliação no eixo vertical, as dicas e o letreiro.
4. **O strip na lateral:** a bandeja em duas colunas, o status e o relógio de duas linhas, e as bolinhas de espaço.
5. **Arrastos, menus, painéis, prévias e o cartão do Calendário** abrindo para o lado de dentro, mais o teclado e o Narrador.
6. **Widgets em pé** (§2), face por face.
7. **Skins na lateral** (§4).
8. **A opção em Configurações**, com as miniaturas Esquerda e Direita, e o roteiro ao vivo completo.

Até o passo 8, a lateral só é alcançada mudando o `settings.json` à mão, para não aparecer pela metade.

## 6. Riscos

1. **O layout em um eixo só** (`row`, `place_with`, `window_size`). Trocar x por "ao longo da barra" mexe em cada consumidor de `Placement` e em dezenas de testes. Mitigação: o passo 1 do §5 sozinho, sem mudar nada na tela, com os testes de hoje passando sem alteração antes das variantes novas.
2. **As faces em pé.** Cada face mede por `u = altura do tile`, e o letreiro corta em x. Mitigação: o passo 6 face por face, com foto de cada uma. O que não couber em pé fica girado (§2).
3. **O relógio e a bandeja numa coluna de 40 a 64 px.** Pedem um desenho novo, não uma transposição (Q7).
4. **As skins.** Girar uma arte em pixel pode deformá-la (§4, Q4).
5. **A faixa lateral com a barra do Windows embaixo e com dois monitores.** A coluna fica descentrada se medir pelo monitor inteiro, e uma faixa numa borda interna fica no meio da área de trabalho (Q9).

## 7. Verificação

- Testes do layout nas quatro bordas (`dock_layout`), da faixa e da ancoragem dos painéis.
- Fotos de cada widget nas laterais, com o dock em cada um dos oito estilos.
- Ao vivo, nas laterais:
  - um e dois monitores;
  - escala de 150 %;
  - ocultar automaticamente;
  - tela cheia;
  - arrastar e soltar;
  - Narrador;
  - a barra do Windows volta como estava ao fechar.

## 8. Decisões do autor

O autor aprovou as nove propostas em 2026-11-04 ("pode seguir com as propostas").

| # | Pergunta | Proposta |
|---|---|---|
| Q1 | Fazer a lateral agora, nos 8 passos do §5, ou só a spec por enquanto? | Agora, um passo por PR. |
| Q2 | Na direita: espelho simples da esquerda (mesma ordem de cima para baixo) ou o "espelho girado" do PrimoDock (relógio em cima, Lançador no pé)? | Espelho girado, como o PrimoDock. |
| Q3 | Widgets em pé: girar a face (a), face própria (b), ou o misto da tabela do §2? | O misto. |
| Q4 | Skins na lateral: girar, versão própria ou sem skin? | Girar as embutidas; as da pasta podem trazer `vertical`. |
| Q5 | A barra do Windows: com o dock na lateral, as opções Visível, Ocultar automaticamente e Oculta continuam iguais? | Sim. Só o dock muda de borda, como no PrimoDock. |
| Q6 | Largura da coluna: a mesma espessura das outras bordas (ícone + margens) ou um pouco maior, para caber o relógio de duas linhas? | A mesma, com o relógio em letras menores. |
| Q7 | O relógio numa coluna: hora em cima da data (`14:05` / `04/11`), só a hora, ou hora em duas linhas (`14` / `05`)? | Hora em cima da data, em letras menores; a data por extenso fica na dica. |
| Q8 | As setas com o dock na lateral: ↑ e ↓ andam pelos itens, e a seta para dentro da tela (→ na esquerda, ← na direita) abre o cartão de prévias? | Sim; a seta para fora não faz nada. |
| Q9 | Com dois monitores lado a lado, o dock lateral numa borda **interna** (a direita do monitor da esquerda): fica ali mesmo, ou só nas bordas externas? | Fica onde o usuário pediu, como no PrimoDock; a spec só registra que a faixa fica no meio da área estendida. |
