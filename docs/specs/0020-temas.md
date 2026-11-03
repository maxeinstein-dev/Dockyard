# Spec 0020 — Temas

Extensão da fase G ([spec 0019](0019-fase-g-extras.md)): **temas** que vestem o dock de uma vez, como os temas sazonais do PrimoDock, começando por temas gamer.

**Status:** primeira leva concluída em 2026-10-30, com os seis passos entregues e testados ao vivo:
1. galeria de Temas e novos nomes (#109);
2. glifos dos launchers (#111);
3. skins de barra, com Retrô, Clássico 98 e Fósforo (#113);
4. Tático (#114);
5. Medieval 8-bit (#116);
6. Caçador (#117).

Também entrou o "experimentar e voltar" (#115). A segunda leva gamer, com Cyberpunk, Fliperama e Espacial, entrou em 2026-11-03 (§4). Os widgets no tom do tema (Q9) e a pasta `Skins` para barras de terceiros (Q14) entraram em 2026-11-03 (§5 e §6).

**Verificação final** (2026-10-30, build e2e da `main` com os seis passos, cópia dos dados do autor):
- **Os 11 temas:** aplicados pelos cartões, um a um. Cada um perguntou "Manter este tema?", foi mantido e gravou estilo e pacote, e o dock desenhou a barra e os ícones dele. Fotos em [`docs/fotos/g7-11-temas-ao-vivo.png`](../fotos/g7-11-temas-ao-vivo.png).
- **Memória em repouso, somando o processo do dock e os 6 do WebView2** (Configurações fechada, 25 s parado). **Não é a memória do dock sozinho** e não se compara aos 64,5 MB da 0.3.0, que mediram só o processo do dock. Mesmo com Configurações fechada, os processos do WebView2 continuaram abertos, o que fica para a M1 (que mede o dock sozinho) e a M8 (painel e Configurações mais leves) do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md). Aqui vale só a comparação entre os temas:

| Tema | Memória (7 processos) | Diferença |
|---|---|---|
| Padrão (base) | 293,2 MB | — |
| Medieval 8-bit (skin e pacote de pixel) | 294,3 MB | +1,1 MB |
| Caçador (skin e pacote vetoriais) | 297,2 MB | +4,0 MB |

  Uma skin e um pacote somam poucos MB ao conjunto: o bitmap da barra e os PNGs dos ícones.
- **E2E:** `e2e/specs/themes.e2e.ts` aplica o Neon e o mantém, experimenta o Vidro e volta, conferindo o `settings.json` e o cartão marcado.
- **Barra do Windows:** ficou igual ao fim.

## 1. Decisões do autor (2026-10-30)

Tomadas numa sessão de perguntas (Q1–Q22), todas na recomendação:

| # | Decisão |
|---|---|
| Q1 | Temas **inspirados**, com nome e desenho próprios: nada de nome, logo, fonte ou arte de jogo (CS2, Tibia, Monster Hunter, Call of Duty são marcas de terceiros). |
| Q2, Q6 | Um **Tema** é um atalho: aplica de uma vez o estilo da barra, o pacote de ícones e a cor do indicador. Nada novo é guardado: o tema em uso é o que tem as três escolhas do dock, e nenhum quando o usuário muda uma delas ("Personalizado"). |
| Q3 | Primeira leva gamer: **Tático** (FPS), **Medieval 8-bit** (MMORPG) e **Caçador** (fantasia de caça). Cyberpunk, arcade e sci-fi ficam para depois. |
| Q4, Q12 | Os pacotes cobrem os apps de sempre e os launchers de jogador (Epic, Battle.net, Riot, EA, Ubisoft, GOG, Faceit, NVIDIA, AMD, Overwolf, Twitch, Medal), que entram também nos pacotes vetoriais. Os executáveis dos jogos ficam com o ícone do sistema ou a reserva. |
| Q5 | Nomes: Geral › Tema vira **Modo de cor**; Aparência › "Tema" (a galeria de barras) vira **Estilo da barra**; **Temas** é o grupo novo, o primeiro de Aparência do dock. Só textos: `settings.json` não muda. |
| Q7, Q20 | A galeria nasce com **Padrão** e combos do que já existe; os de skin chegam com as skins. |
| Q8 | Um tema não mexe no modo de cor. A barra dos temas gamer é escura nos dois modos. |
| Q9 | Os widgets ficam como estão nesta leva. |
| Q10, Q14, Q21 | A barra dos temas gamer é uma **skin de 9 fatias** (PNG + `skin.json`), embutida, num formato pronto para uma pasta `Skins` no futuro. Com skin, somem só os ajustes que a imagem define (cantos, borda). |
| Q11 | Aplicar um tema troca a cor do indicador pela dele (`null`: a do estilo). |
| Q15 | Os temas são uma lista fixa no código (`src/core/themes.ts`). |
| Q16–Q18 | Os três temas gamer (§4). |
| Q19 | As skins e os pacotes dos temas aparecem também avulsos nas galerias. |
| Q13, Q22 | Ordem: (1) nomes e galeria com combos; (2) glifos dos launchers; (3) skin de 9 fatias, com Retrô, Clássico 98 e Fósforo; (4) Tático; (5) Medieval 8-bit; (6) Caçador. Um PR por passo, com folha comparativa e teste ao vivo. |

## 2. Passo 1: a galeria de Temas

- `src/core/themes.ts`: `THEMES`, a lista dos temas (`id`, `style`, `iconPack`, `indicatorColor`); `choicesOf(theme)` dá as três escolhas, e `themeOf(dock)` dá o tema em uso, ou `null` ("Personalizado"). O pacote e a cor se comparam sem diferenciar maiúsculas.
- Os combos de hoje:

| Tema | Estilo da barra | Ícones | Indicador |
|---|---|---|---|
| Padrão | Nativo | originais | do estilo |
| Neon | Neon | Neon | do estilo |
| Esmalte | Nativo | Esmalte | do estilo |
| Vidro | Liquid Glass | Vidro | do estilo |
| Meia-noite | Meia-noite | Traço | do estilo |

- Configurações › Aparência do dock ganha o grupo **Temas**, o primeiro: um cartão largo por tema (`ChoiceCards`, como os estilos), com o dock em miniatura no estilo do tema, os cinco primeiros apps fixados nos ícones dele (`icon_pack_preview`) e o indicador. Escolher um cartão grava as três escolhas de uma vez. Nenhum cartão marcado quando não há tema em uso, com a linha "Personalizado: …" embaixo. Sem apps fixados (o painel), a miniatura mostra os ladrilhos dos widgets.
- **Experimentar e voltar** (pedido do autor em 2026-10-30, como a troca de resolução do Windows): aplicar um tema é uma tentativa. Abaixo dos cartões aparece "Manter este tema?", com **Manter**, **Voltar ao anterior** e a contagem "Volta ao anterior em 15 s". Sem resposta, as três escolhas de antes voltam sozinhas. Experimentar outro tema durante a contagem recomeça o tempo, e voltar leva às escolhas de antes do primeiro tema experimentado, inclusive uma combinação "Personalizado". Sair da tela mantém o que está aplicado. Os ajustes de cada estilo nunca se perdem, porque ficam guardados por estilo. Testes em `ThemeGallery.test.tsx`: manter, voltar, a contagem que expira (conferida com sabotagem) e dois temas seguidos.
- **Ícones na barra escura** (Q8): as imagens escuras de um pacote (`dark` do `pack.json`) seguem a barra, não só o modo de cor. `DockStyle::bar_dark(modo)`: Meia-noite, Rocket e Neon são barras escuras nos dois modos, Papel é clara nos dois, e as outras seguem o modo. Valem para o dock nativo, o `app_icon` e o `icon_pack_preview`, que recebe o estilo da miniatura (o do tema; o atual quando nulo). Sem isso, Meia-noite + Traço no modo claro punha o grafite do Traço sobre a barra preta.
- Textos: "Modo de cor", "Estilo da barra", "Personalizar este estilo" e "Temas", em português e inglês. O lançador acha Aparência também por "temas" e "estilo da barra", e Configurações por "modo de cor".
- Um pacote que o usuário apagou de `IconPacks` continua no tema: aplicado, o dock mostra os ícones originais e o cartão de ícones o mostra como indisponível, como hoje.
- **Testes:** `themes.test.ts` (cada tema usa um estilo que existe e um pacote embutido que existe; cada tema é reconhecido pelas suas escolhas; nenhum quando uma escolha é do usuário); `settings::bar_dark` (as barras escuras, a clara e as que seguem o modo); `ThemeGallery.test.tsx` (ordem dos cartões, tema em uso marcado, aplicar grava as três escolhas, "Personalizado", miniatura com os ícones do tema, desligado).

- **Verificado ao vivo** (2026-10-30, build de teste e2e, cópia dos dados do autor, modo claro): Aparência mostrou o grupo **Temas** com os 5 cartões e o **Estilo da barra**. Os dados do autor (Flutuante com indicador branco) apareceram sem tema marcado, com "Personalizado". Cada cartão gravou as três escolhas e ficou marcado, e o dock trocou barra e ícones na hora. A Meia-noite mostrou o Traço claro na barra preta, e o Neon os ícones soltos, como pede a barra escura. Trocar o pacote para Argila desmarcou o tema e mostrou "Personalizado". Memória: 295 MB em 7 processos com Configurações aberta. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-temas-ao-vivo.png`](../fotos/g7-temas-ao-vivo.png).

## 3. A skin de 9 fatias (passo 3)

- **Formato:** `src-tauri/skins/<id>/skin.png` e `skin.json`, com o nome, o tamanho da imagem, o corte de 9 fatias (`slice`: `top`, `right`, `bottom`, `left`, em pixels), `middle` (`repeat` ou `stretch`) e `pixelated`. As fatias de cima e de baixo são vazias: a imagem inteira vai na altura da barra, então o corte vira três colunas (as pontas como estão e o meio repetido ou esticado até a largura). Um teste confere que o `skin.json` bate com o tamanho do PNG.
- **Fonte:** `src-tauri/skins/fonte/<id>.txt`, um desenho em texto com paleta, como os pacotes de pixel. `python gerar.py <id>` escreve os dois arquivos.
- **Dock nativo:** `DockStyle` ganha `Retro`, `Classic98` e `Phosphor`. Neles o `Look` tem `skin: true` e cores próprias de separador, tinta do relógio e indicador. A barra é desenhada por `skin_bar` (`paint.rs`): o bitmap da skin, feito uma vez, em pedaços calculados por `domain::skin::pieces`, sem suavização (pixel art) e com bordas duras, para os ladrilhos do meio não mostrarem emenda. Não há preenchimento, borda de luz, contorno nem brilho.
- **Miniatura:** `STYLE_PAINTS` lê os mesmos `skin.json` e `skin.png` e desenha a barra com `border-image` (pontas e meio) e `image-rendering: pixelated`.
- **Ajustes (Q21):** numa skin, "Borda" e "Cantos" somem; "Reflexos" fica.
- **Barra escura:** Retrô e Fósforo são escuras nos dois modos; o Clássico 98 é claro nos dois.
- **As três skins** (16 px de altura, ×4 numa barra de 64 px):

| Skin | Desenho | Indicador | Tema |
|---|---|---|---|
| Retrô | placa azul-noite PICO-8, contorno preto, luz em cima e à esquerda, cantos de pixel | `#FF004D` | Retrô (com o pacote Retrô 8-bit) |
| Clássico 98 | painel elevado do Windows 98 (branco, cinza-claro, cinza, preto) | `#000080` | Clássico 98 (com o pacote Clássico 98) |
| Fósforo | moldura com bisel e tela escura com linhas de varredura | `#3CFF6E` | Fósforo (com o pacote Fósforo) |

- **Testes:** `domain::skin` (as pontas na altura da barra; o meio em ladrilhos com o último cortado; a barra estreita encolhe as pontas; cada skin embutida bate com o PNG e com o estilo); `look`, `settings` (os estilos novos no JSON e na barra escura), `paints.test` (a miniatura marca a skin que `look.rs` marca), `StyleGallery` (a barra de skin com a imagem dela), `LookPanel` (sem borda e cantos numa skin) e `ThemeGallery` (os 8 temas).
- **Verificado ao vivo** (2026-10-30, build de teste e2e, cópia dos dados do autor): Padrão, Retrô, Clássico 98 e Fósforo, aplicados pelos cartões, gravaram estilo e pacote, e o dock desenhou as skins. "Cantos" sumiu nas três. A primeira rodada mostrou finas emendas entre os ladrilhos do meio do Retrô e do Clássico 98. A correção: meio esticado nas skins de meio liso e bordas duras no desenho da skin. Na segunda rodada não houve emenda, nem no Fósforo, que repete o meio. Memória: 296 MB em 7 processos com Configurações aberta. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-skins-ao-vivo.png`](../fotos/g7-skins-ao-vivo.png).

## 4. Os temas gamer (passos 4–6)

- **Tático** (passo 4, feito):
  - **Skin** `tactical` (`skins/fonte/tactical.py`, em vetor): 52×64 px, pontas de 24 px e meio esticado, suavizada (`pixelated: false`). Corpo chanfrado a 45° em gradiente carvão-oliva (`#2D3125` → `#191B13`), borda `#4A5038`, fio de areia (`#D9C9A3`) a 5,5 px do topo e colchetes de mira nas pontas. Separador areia a 30%, relógio areia, indicador laranja de alerta `#FF8A1F`. Barra escura nos dois modos.
  - **Pacote** `tatico` (`fonte/tatico.txt`, estilo de camadas `tatico`): o glifo de contorno em areia sobre placa octogonal (quadrado chanfrado em 29% do lado) em gradiente oliva (`#353A2A` → `#20241A`), com fio de areia a 40% na borda e sombra curta. A reserva é uma retícula de mira (`apps/mira.png`): o `vetor.py` passa a aceitar `reserva-id` e `reserva` no `.txt` do pacote, para trocar o glifo da reserva.
  - **Tema** `tactical`: skin Tático, pacote Tático e indicador do estilo.
  - **Verificado ao vivo** (2026-10-30, build e2e, cópia dos dados do autor): o cartão Tático gravou estilo `tactical` e pacote `tatico`, e o dock desenhou a barra chanfrada com o fio de areia, os ícones nas placas octogonais e as marcas laranja. "Cantos" sumiu. Memória: 294 MB em 7 processos com Configurações aberta. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-tatico-ao-vivo.png`](../fotos/g7-tatico-ao-vivo.png).
  - Com os glifos dos launchers (passo 2), o pacote cobre 55 apps, e o teste dos launchers inclui o Tático.
- **Medieval 8-bit** (passo 5, feito):
  - **Skin** `medieval` (`skins/fonte/medieval.txt`, pixel art 16×16): moldura de madeira escura com cantoneiras de ferro e rebites nas pontas (4 px cada) e o meio de tijolos de pedra, que se repete (8 px). Separador madeira, relógio pergaminho `#E8D9B0`, indicador ouro `#E8B84A`. Barra escura nos dois modos.
  - **Pacote** `medieval` (`fonte/medieval.txt`, pixel art 32×32, `gerar.py`): cada app num espaço de inventário de pedra com bisel. O objeto sai do glifo, com contorno escuro, corpo na cor terrosa mais próxima da matiz do app (vermelho, laranja, ouro, verde, verde-água, azul, roxo, vinho ou marrom), detalhes em pergaminho e sombra de 1 px. Um glifo sem área fechada leva o traço na cor do app. Cobre os 55 glifos. A reserva é um baú (`apps/bau.png`). Os desenhos partiram dos glifos e se editam no texto.
  - **Tema** `medieval`: skin Medieval, pacote Medieval e indicador do estilo.
  - **Verificado ao vivo** (2026-10-30, build e2e com o "experimentar e voltar", cópia dos dados do autor): o Medieval aplicado mostrou a moldura de madeira, os tijolos de pedra, os ícones em espaços de inventário e as marcas ouro, sem "Cantos". Os casos do "experimentar e voltar" também passaram: Manter tirou a pergunta; o Tático sem resposta voltou sozinho ao Medieval depois da contagem; e Voltar ao anterior no Neon voltou ao Medieval. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-medieval-ao-vivo.png`](../fotos/g7-medieval-ao-vivo.png).
- **Caçador** (passo 6, feito):
  - **Skin** `hunter` (`skins/fonte/hunter.py`, em vetor): 60×64 px, pontas de 26 px e meio de 8 px que se repete, para os pontos da costura não esticarem. Couro curtido em gradiente (`#7A4E2C` → `#4A2E18`), vinco escuro na borda, costura pespontada cor de linha (`#E8D2A6`) a 7,5 px de cima e de baixo, e pontas de osso com rebite. Separador linha a 40%, relógio linha, indicador âmbar `#F2A33A`. Barra escura nos dois modos.
  - **Pacote** `cacador` (`fonte/cacador.txt`, estilo de camadas `cacador`): medalhão com aro de osso (`#EDE4CC` → `#B9AB8C`) e couro dentro (`#7A4E2C` → `#4A2E18`), com vinco escuro. O glifo, de traço 2,2, é gravado em âmbar, com sombra embaixo e à direita e luz em cima e à esquerda. Cobre os 55 glifos. A reserva é uma pegada de garra (`apps/garra.png`). O `disco` das camadas passa a aceitar `recuo`, como o octógono.
  - **Tema** `hunter`: skin Caçador, pacote Caçador e indicador do estilo.
  - **Verificado ao vivo** (2026-10-30, build e2e, cópia dos dados do autor): o cartão Caçador perguntou "Manter este tema?", gravou estilo `hunter` e pacote `cacador`, e o dock desenhou o couro com a costura, as pontas de osso, os medalhões gravados e as marcas âmbar, sem "Cantos". Memória: 298 MB em 7 processos com Configurações aberta. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-cacador-ao-vivo.png`](../fotos/g7-cacador-ao-vivo.png).
- **Segunda leva** (2026-11-03, pedido do autor depois da primeira): a mesma receita, com desenho próprio e sem nome, logo ou arte de jogo.
  - **Cyberpunk:**
    - **Skin** `cyberpunk` (`skins/fonte/cyberpunk.py`, em vetor): 56×64 px, pontas de 26 px e meio esticado, suavizada. Asfalto quase preto em gradiente (`#1A1624` → `#0B0A10`), com dois cantos cortados em diagonal (em cima à esquerda e embaixo à direita). Fio ciano (`#2EF2FF`) a 6 px do topo e fio magenta (`#FF2BD6`) a 6 px da base, os dois com brilho, e pedaços deles deslocados nas pontas, como uma imagem que falha. Separador magenta a 35%, relógio ciano, indicador magenta. Barra escura nos dois modos.
    - **Pacote** `cyberpunk` (`fonte/cyberpunk.txt`, estilo de camadas `cyberpunk`): placa hexagonal deitada (a forma nova `hexagono` das camadas), quase preta, com aro magenta e o glifo em néon ciano, com brilho e núcleo claro. Cobre os 55 glifos. A reserva é um chip (`apps/chip.png`).
  - **Fliperama:**
    - **Skin** `arcade` (`skins/fonte/arcade.txt`, pixel art 16×16): moldura de gabinete preta com friso cromado, painel azul-noite, as listras de corrida, um pouco apagadas (vermelho `#B83030`, laranja `#B8661C` e amarelo `#B8962E`) e uma entrada de ficha acesa em cada ponta (4 px). O meio se repete (8 px). Separador cromado, relógio branco (a data passa sobre as listras, um pouco apagadas) e indicador amarelo. Barra escura nos dois modos.
    - **Pacote** `fliperama` (`fonte/fliperama.txt`, pixel art 32×32, `gerar.py`): cada app é um sprite na cor de fliperama mais perto da matiz dele (vermelho, laranja, amarelo, verde, ciano, azul, roxo ou rosa), com 1 px de sombra, numa tela escura com linhas de varredura e moldura. Os desenhos foram feitos uma vez por `fonte/pixelar.py` a partir de `glifos.txt` e se editam no texto, como os do Medieval. A reserva é um joystick (`apps/joystick.png`).
  - **Espacial:**
    - **Skin** `scifi` (`skins/fonte/scifi.py`, em vetor): 56×64 px, pontas de 26 px e meio esticado, suavizada. Casco de metal azul-acinzentado em gradiente (`#3A4458` → `#1C2230`) com os quatro cantos chanfrados, filete de luz no topo, faixa de luz ciano (`#4FD8FF`) perto da base, com brilho, e divisas que apontam para fora e rebites nas pontas. Separador ciano a 30%, relógio aço claro (`#9FB2CC`), indicador ciano. Barra escura nos dois modos.
    - **Pacote** `espacial` (`fonte/espacial.txt`, estilo de camadas `espacial`): escotilha com aro de metal (`#7A879C` → `#3A4458`), anel ciano, vidro azul-marinho (`#13234A` → `#070E22`) e o glifo branco com brilho ciano. Cobre os 55 glifos. A reserva é um planeta com anel (`apps/planeta.png`).
  - **Temas** `cyberpunk`, `arcade` e `scifi`: cada um com a skin e o pacote dele e o indicador do estilo. Aparecem na galeria depois do Caçador, e as skins e os pacotes aparecem também avulsos (Q19).
  - **Verificado ao vivo** (2026-11-03, build e2e da branch dos temas, cópia dos dados do autor): em Configurações › Aparência do dock, cada cartão aplicado e mantido gravou o estilo e o pacote dele (`cyberpunk/cyberpunk`, `arcade/fliperama`, `scifi/espacial`), e o dock foi fotografado em cada um. A foto do Fliperama mostrou a data difícil de ler sobre as listras: as listras ficaram mais apagadas e o relógio passou a branco (`a9e7950`), antes do merge.

## 5. A pasta `Skins` (Q14)

Decisão do autor (2026-11-03): **pasta + aviso**. Barras de terceiros entram por uma pasta, sem instalar nada e sem reiniciar.

- **Onde:** `Skins`, dentro da pasta de dados do app (`%APPDATA%\dev.maxsueleinstein.dockmax\Skins`). Em Configurações › Aparência do dock, embaixo da galeria de estilos, o botão **"Abrir pasta Skins"** abre a pasta no Explorador e a cria se ela não existir.
- **Formato:** uma subpasta por skin, com `skin.json` e `skin.png`, no mesmo formato das skins embutidas (§3). O `skin.json` aceita três campos opcionais, que as embutidas não usam:
  - `dark` (`true` ou `false`): se a barra é escura, para os ícones escuros de um pacote. Sem o campo, a barra conta como escura.
  - `ink` (`#rrggbb`): a tinta do relógio. Os separadores ficam nela, a 35%.
  - `indicator` (`#rrggbb`): a cor da marca embaixo de um app aberto.
  
  Uma cor ausente ou que não seja `#rrggbb` fica a do dock.
- **Validação** (`domain::skin::read_folder`), só nas subpastas diretas:
  - o nome da pasta tem de 1 a 40 caracteres entre letras, números, espaço, `_`, `.` e `-`, e não começa com ponto;
  - o `skin.json` existe, tem até 64 KB e segue o formato;
  - `image` é exatamente `skin.png`;
  - largura e altura vão de 1 a 4096 px, e as pontas (`left` + `right`) são mais estreitas que a imagem;
  - o `skin.png` existe dentro da pasta (um atalho que leve para fora não vale), tem até 2 MB, é um PNG, e o cabeçalho dele tem o tamanho que o `skin.json` diz.
  
  Uma pasta que falha em alguma regra não vira cartão: um aviso embaixo da galeria diz qual pasta e por quê. Uma pasta `Skins` que não existe é só vazia.
- **Na galeria:** cada skin válida é um cartão depois dos estilos embutidos, em ordem de nome da pasta, com a miniatura e o `name` do `skin.json`. A pasta é lida quando Configurações (ou a Aparência no painel) abre. Escolher uma skin grava `style: "custom"` e `customSkin` com o nome da pasta. Uma skin escolhida que sumiu continua como cartão "Indisponível".
- **No dock:** o estilo `Custom` desenha a skin da pasta como as embutidas (`skin_bar`). O dock lê a skin fora da pintura (`read_data`) e a guarda em cache; só relê o PNG quando o tamanho ou a data de um dos dois arquivos muda, e então refaz o bitmap. Se a skin escolhida sumir ou ficar inválida, o dock pinta a barra como o Nativo e registra uma vez no log, sem o nome da pasta.
- **Ajustes:** como nas embutidas, "Borda" e "Cantos" somem.
- **Testes:** `domain::skin` (cada motivo de recusa, uma skin válida, pasta ausente, ordem por nome, cabeçalho do PNG, BOM, cache relido quando o arquivo muda, campos opcionais), `settings` (`customSkin` no JSON e na validação, barra escura do `Custom`), `look` (cores da skin e volta ao Nativo), `commands::skins` (o cartão vai como `data:` URL), `StyleGallery` (cartões depois dos estilos, escolha, indisponível) e `LookPanel` (grava `custom` e `customSkin`, aviso das pastas recusadas, botão da pasta).
- **Verificado ao vivo** (2026-11-03, build de lançamento, cópia dos dados do autor): com uma skin válida ("Teste de pasta", a imagem do Cyberpunk com tinta `#FFE45E`) e uma quebrada na pasta, Configurações › Aparência do dock mostrou o cartão da skin (um botão de opção da galeria), o aviso "Pasta quebrada: o skin.json não é válido" e o botão "Abrir pasta Skins"; escolher o cartão gravou `style: "custom"` e `customSkin: "Teste de pasta"`, e o dock passou a usar a imagem, com o relógio na tinta do `skin.json`. A barra do Windows ficou igual. Falta ver ao vivo a volta ao Nativo ao apagar a pasta.

## 6. Os widgets no tom do tema (Q9)

Decidido pelo autor em 2026-11-03: os widgets seguem a **cor de destaque** do estilo em uso; o desenho e o vidro de cada widget ficam como estão.

- O que enche o anel (Ciclo, Pomodoro, Água, Claude), a barra de um medidor (Atividade) e os ponteiros do relógio de ponteiros é a cor do indicador em uso: a escolhida pelo usuário, ou a do estilo, ou a de destaque do sistema.
- A cor é clareada em direção ao branco só o quanto precisa para ter contraste de 3:1 (o mínimo da WCAG para gráficos) contra o topo do vidro do widget (`Glaze::mark`). Como as cores dos estilos são tão claras quanto os vidros, quase sempre fica um tom da cor, não ela pura.
- Texto, números e glifos seguem brancos.
- **Verificado ao vivo** (2026-11-03, build de lançamento, cópia dos dados do autor sem cor de indicador escolhida): Atividade e Ciclo em verde no Fósforo, laranja no Tático, rosa no Cyberpunk e amarelo no Fliperama; com o indicador branco escolhido pelo autor, tudo segue branco. Foto em [`docs/fotos/q9-widgets-ao-vivo.png`](../fotos/q9-widgets-ao-vivo.png) (antes à esquerda, depois à direita).
