# Spec 0029: Configurações reorganizadas

Configurações cresceu com a busca (F6), os espaçadores, as pilhas e o ícone por app (F2–F4) e a cópia das configurações (F8, [spec 0026](0026-exportar-importar.md)). Agora a F13 traz Monitores. O pedido do autor é uma janela "mais limpa, minimalista e melhor dividida, para que, ao ler a aba, a pessoa já entenda tudo que estará lá, mas sem parecer uma listagem infinita".

Esta spec muda a **arquitetura de informação** da janela da [spec 0011](0011-arquitetura-hibrida.md) §3. A linguagem visual continua a da [spec 0014](0014-rebranding-configuracoes-e-menus.md) §3: placa, cabeçalho e grupo com linhas.

**Status:** aprovada em 2026-11-04, com as decisões do grilling (Q1–Q12, §9). Os passos do §10 entram um por PR.

## 1. Onde estamos

| Seção | Grupos | Linhas no padrão |
|---|---|---|
| Geral | 6: Dock, Windows, Comportamento, Lançador, Idioma, Inicialização | ~21, até ~26 |
| Espaços de trabalho | nenhum `SettingsGroup`; parágrafo longo, fichas, cartão do espaço | ~8 |
| Aparência do dock | 7 | 12–13, quase todas galerias (14 temas, 17 estilos e skins) |
| Apps do dock | nenhum; fixados, busca, catálogo | listas |
| Widgets (`items`) | nenhum; em uso, catálogo de 5 prateleiras | listas |
| Contas | 2 | ~4 |
| Sobre | Sobre, Cópia das configurações | ~6 |

O que pesa hoje:
- **A Geral virou a gaveta de tudo.** Só Comportamento tem 10 linhas fixas, e chega a 14. O Lançador traz uma lista com formulário embutido.
- **Não há recolhimento nem camada avançada.** Hoje só existem as linhas que aparecem quando um switch é ligado.
- **O topo de cada seção é uma frase só.** Não diz o que vem embaixo.
- **Busca:** o índice é escrito à mão (`src/settings/search.ts`). O teste (`search.test.tsx`) só confere as linhas visíveis no padrão: as linhas condicionais, Apps e Contas escapam.
- **Barra lateral:** só anda por Tab, e trocar de seção não move o foco.

## 2. O mapa novo

A barra lateral ganha **três faixas**, cada uma com um título pequeno. A pessoa lê três blocos, não dez nomes:

| Faixa | Seção (nome curto) | id | Grupos |
|---|---|---|---|
| **Dock** | Geral | `general` | Dock · Windows · Idioma e inicialização |
| | Aparência | `look` | Temas · Estilo da barra · Ao passar o mouse · Indicador · Ícones · *Avançado* |
| | Comportamento | `behavior` (novo) | Mostrar e ocultar · Prévias · Apps no dock · Energia · *Avançado* |
| **Conteúdo** | Espaços | `workspaces` | (fichas e cartão do espaço) |
| | Apps | `apps` | (fixados e catálogo) |
| | Widgets | `items` | (em uso e catálogo) |
| **Mais** | Lançador | `launcher` (novo) | Lançador · Busca na web · *Avançado* |
| | Monitores | `monitors` (F13) | definido pela F13 |
| | Contas | `accounts` | App do Google · Contas Google |
| | Sobre | `about` | Sobre · Cópia das configurações |

- **Ordem:** a da tabela, de cima para baixo. A janela continua abrindo na Geral.
- **Nomes curtos:**
  - "Aparência do dock" vira **Aparência**;
  - "Apps do dock" vira **Apps**;
  - "Espaços de trabalho" vira **Espaços**.

  A faixa já diz do que se trata.
- **ids:**
  - continuam os de hoje, inclusive `items`, para não quebrar `__DOCKYARD_SECTION__` nem o lançador;
  - `behavior` e `launcher` são novos;
  - `monitors` vem da F13.
- **Rust:** `Section` em `src-tauri/src/domain/launcher_pages.rs` acompanha a lista, como pede o comentário de `src/settings/sections.ts`. O lançador passa a achar "Comportamento" e "Lançador" como páginas.

### 2.1 De onde vem cada linha da Geral de hoje

| Linha hoje | Vai para |
|---|---|
| Modo de cor, Posição, Tamanho, Opacidade | Geral › Dock |
| Barra de tarefas do Windows, Ícones da bandeja no dock | Geral › Windows |
| Idioma, Iniciar com o Windows | Geral › Idioma e inicialização |
| Ocultar automaticamente, com os dois atrasos | Comportamento › Mostrar e ocultar |
| Prévia ao passar o mouse, com o atraso e "Só com 2 ou mais janelas" | Comportamento › Prévias |
| Mostrar apps abertos, Mostrar selos de notificação | Comportamento › Apps no dock |
| Economia de bateria | Comportamento › Energia |
| Botão do meio, Atalho do dock | Comportamento › Avançado |
| Lançador (switch), Atalho do lançador, aviso de atalho em uso | Lançador › Lançador |
| Busca na web | Lançador › Busca na web |
| Histórico do lançador | Lançador › Avançado |
| Dock em todos os monitores, Trocar de espaço em todos os monitores | **Monitores** (F13) |
| Indicador na cor do espaço | **Aparência › Indicador**, ao lado de "Cor do indicador" |

Até a F13 mergear, os dois switches de monitor ficam em Comportamento, num grupo "Monitores", e saem de lá no PR em que a seção Monitores entrar. Assim nenhum ajuste fica sem lugar.

## 3. O topo de cada seção

- **Frase de apoio:** continua (`settings.app.leads`) e ganha texto novo onde a seção mudou:
  - Geral: "Onde o dock fica e como ele convive com o Windows."
  - Comportamento: "Quando o dock aparece e o que ele mostra."
  - Lançador: "Abra apps e busque na web pelo teclado."
  - Monitores: o texto é da F13.
- **Índice dos grupos:** fica embaixo da frase. São links em linha, separados por " · ", por exemplo "Temas · Estilo da barra · Ao passar o mouse · Indicador · Ícones · Avançado".
  - Cada link rola até o grupo e põe o foco no título dele.
  - **O índice só aparece quando a seção tem 3 grupos ou mais, sem contar o Avançado.** Hoje isso vale para Geral, Aparência e Comportamento.
  - Quando aparece, o Avançado entra como último link e, ao ser clicado, também se abre.
- O índice é um `nav` com nome ("Nesta seção"), separado do `nav` da barra lateral.

## 4. Avançado

**A regra:** vai para o Avançado o ajuste fino ou raramente mexido. Uma linha que só aparece com um switch ligado continua ao lado desse switch.

- Cada seção que precisa tem **um** grupo "Avançado" no fim, **fechado por padrão** (`<details>`/`<summary>`, ou um botão com `aria-expanded`).
- O resumo fechado diz o que tem dentro, por exemplo "Avançado: botão do meio, atalho do dock", para nada ficar escondido sem pista.
- **Aberto ou fechado:** o estado vale só enquanto a janela está aberta e volta a fechado na próxima vez. Não vai para o banco.

| Seção | Avançado |
|---|---|
| Comportamento | Botão do meio, Atalho do dock |
| Aparência | Personalizar este estilo (Borda, Cantos, Reflexos, Restaurar); Animação de abertura; Abrir pasta Skins, Abrir pasta de pacotes, Recarregar pacotes |
| Lançador | Histórico do lançador (Limpar o histórico) |
| Geral, Espaços, Apps, Widgets, Monitores, Contas, Sobre | sem Avançado |

Dois casos especiais na Aparência:
- **Problemas de skin ou de pacote:** a lista de problemas da pasta (`SkinsFolderNotice`, problemas dos pacotes) **continua visível** embaixo da galeria, mesmo com os botões de pasta no Avançado. Erro não fica escondido.
- **Com uma skin, Borda e Cantos somem**, como hoje ([spec 0020](0020-temas.md) Q21). Se o "Personalizar este estilo" ficar vazio, ele some do Avançado.

## 5. Galerias com "Mostrar todos"

São as galerias de Tema e de Estilo da barra (e qualquer `ChoiceCards` com mais de 2 fileiras).

- Cada uma mostra **até 2 fileiras**. Embaixo vem o botão "Mostrar todos ({n})", que abre o resto ali mesmo e vira "Mostrar menos".
- **A escolha atual está sempre visível.** Se ela cairia fora das 2 fileiras, entra no lugar do último cartão visível.
- **Teclado:** com as setas, passar da última fileira visível expande a galeria, e o foco segue para o próximo cartão. O `radiogroup` continua um só.
- **Busca:** se a busca levar a um cartão escondido, a galeria se expande.

## 6. Dicas enxutas

**A regra:** no máximo 1 linha, sem repetir o rótulo. A tabela vale para pt-BR e en.

| Chave | Hoje | Fica |
|---|---|---|
| `autoHideHint` | O dock desce quando o ponteiro sai e sobe ao encostar na borda da tela | Desce quando o ponteiro sai e sobe ao encostar na borda |
| `autoHideRevealHint` | Quanto tempo o ponteiro fica na borda antes de o dock subir | Com o ponteiro na borda |
| `autoHideDelayHint` | Quanto tempo o ponteiro fica fora antes de o dock descer | Com o ponteiro fora do dock |
| `previewDelayHint` | Quanto tempo parado antes de a prévia aparecer | *cortada* |
| `previewsOnlySeveralHint` | Um app com uma janela só não mostra a prévia | *cortada* |
| `launchAtLoginHint` | Abre o Dockyard ao entrar no Windows | *cortada* |
| `powerSaverHint` | Para o letreiro e o pulo ao abrir e lê o sistema com menos frequência | Menos animação e menos leituras do sistema |
| `dockHotkey.hint` | Leva o teclado ao dock: as setas andam…, Enter abre, Shift+F10…, Esc volta… | Leva o teclado ao dock; Esc volta ao programa de antes |
| `launcher.hint` | Busca e abre apps de qualquer lugar; também em Buscar apps…, no menu do dock | Busca e abre apps de qualquer lugar |
| `launcher.hotkey.hint` | Clique no botão e pressione a nova combinação | *cortada* (o botão e o estado "Pressione a combinação…" já dizem) |
| `launcher.keywords.hint` | No lançador, digite a palavra-chave…: g dockyard. A primeira da lista é a busca padrão… | Digite a palavra-chave e o que buscar: g dockyard |
| `look.cornersHint` | O quanto a barra é arredondada | *cortada* |
| `look.iconPackHint` | Troque os ícones de todos os apps por um pacote da pasta de pacotes | Troca os ícones de todos os apps de uma vez |

**Ficam como estão:** `showOpenAppsHint`, `showBadgesHint`, `indicatorFollowsWorkspaceHint`, `previewsHint`, `historyHint`, `reflectionsHint`, `restoreHint`, `skinsHint` e `taskbarHints`. As dicas de monitor ficam com a F13.

Duas consequências dos cortes:
- **"Busca padrão":** a regra saiu da dica da busca na web. Na lista, a primeira palavra-chave ganha a etiqueta **"Padrão"**, e "Tornar padrão" continua nas outras.
- **Atalho do dock:** os detalhes do teclado do dock (setas, Enter, Shift+F10) saíram da dica. Eles continuam no tour e no `docs/`.

## 7. Espaços

Esta parte só entra depois do merge da F13, que mexe em `WorkspacesPanel` e em Configurações › Espaços.

- O parágrafo longo (`workspaces.lead`) sai. No lugar fica a frase de apoio, como nas outras seções.
- A explicação de como trocar (Ctrl+Alt+n, a roda do mouse, as bolinhas) vira uma linha "Como trocar de espaço", com dica de 1 linha: "Ctrl+Alt+1 a 9, a roda do mouse no dock ou as bolinhas".

## 8. Busca e leitor de tela

1. **Busca no Avançado e na galeria:** a busca acha o que está recolhido. Ela abre o Avançado (ou expande a galeria), rola, realça e foca, como hoje.
2. **O índice cobre tudo:** inclui as linhas que só aparecem com um switch ligado (atrasos de mostrar e ocultar, atraso da prévia, "Só com 2 ou mais janelas", escala do hover, atalho do lançador), Apps e Contas.
   - O teste de `search.test.tsx` passa a renderizar cada seção **com os switches ligados e o Avançado aberto**, e com Contas num estado que mostre o painel.
   - Ele continua exigindo, nos dois sentidos, que índice e tela batam.
3. **Resultados:** mostram "Seção › Grupo". A faixa não entra, porque é só visual. Para uma linha do Avançado, o grupo é o do Avançado ("Comportamento › Avançado").
4. **Barra lateral pelo teclado:** cada faixa é um grupo com nome (`role="group"` e `aria-labelledby` no título da faixa). As setas ↑/↓ andam entre as seções da barra inteira, atravessando faixas; Home e End vão à primeira e à última. A barra é uma parada de Tab só, com roving tabindex.
5. **Foco ao trocar de seção pelo teclado:** vai para o `h1#panel-title` (com `tabIndex=-1`), para o leitor de tela anunciar a seção. Pelo mouse, o foco fica onde está.
6. **Busca sem depender do texto:** a busca continua achando a linha pelo rótulo visível (`findRow`). As faixas, o índice e o Avançado não mudam o mecanismo, só onde a linha está. Trocar o mecanismo por data-attributes fica fora desta spec.

## 9. Decisões (grilling de 2026-11-04)

| # | Decisão |
|---|---|
| Q1 | A Geral se desmembra em Geral, Comportamento e Lançador. A barra lateral ganha faixas: Dock, Conteúdo e Mais. Apps e Widgets ficam separados |
| Q2 | Um grupo "Avançado" recolhido no fim de cada seção que precisar |
| Q3 | Frase de apoio e índice dos grupos. O índice só aparece com 3 grupos ou mais |
| Q4 | Configurações guarda todo ajuste persistente, e os menus são atalhos que podem repetir. Ações sobre um item específico (ícone do app, tamanho do widget, empilhar) ficam só no menu. Nada sai de Configurações |
| Q5 | Busca no recolhido, índice completo e teste correspondente, resultados "Seção › Grupo", setas na barra lateral, foco no título |
| Q6 | A repartição do §2.1. Monitores vai para a F13, e "Indicador na cor do espaço" vai para Aparência › Indicador |
| Q7 | O conteúdo do Avançado do §4 |
| Q8 | Faixas Dock, Conteúdo e Mais; ordem e nomes curtos do §2; abertura na Geral |
| Q9 | Galerias com 2 fileiras e "Mostrar todos" (§5) |
| Q10 | Dicas de no máximo 1 linha, conforme a tabela do §6 |
| Q11 | O parágrafo de Espaços vira frase de apoio e uma linha "Como trocar de espaço", depois da F13 |
| Q12 | Um PR por passo (§10) |

## 10. Entrega

| Passo | O que entra | Depende de |
|---|---|---|
| **P1** | As faixas na barra lateral e o mapa do §2, com Comportamento e Lançador como seções, `launcher_pages.rs` e os textos pt-BR e en. Também entram o índice no topo (§3), o teclado e o foco (§8.4–5), o índice de busca completo e o teste novo (§8.2), e a atualização do `DESIGN.md` com a janela de Configurações (barra, faixas, grupo, índice, Avançado), que hoje só descreve o painel antigo | — |
| **P2** | O grupo Avançado (§4), as dicas enxutas (§6) e a etiqueta "Padrão" | P1 |
| **P3** | As galerias com "Mostrar todos" (§5) e Espaços (§7) | P1; a parte de Espaços espera o merge da F13 |

**Coordenação com a F13:**
- A seção `monitors` e o seu conteúdo são da F13.
- Se a F13 mergear antes do P1, o P1 já põe Monitores na faixa "Mais" e tira de Comportamento os dois switches de monitor.
- Se mergear depois, ela faz isso no próprio PR.

## 11. Testes

- **`SettingsApp.test.tsx`:**
  - as faixas e a ordem das seções;
  - setas, Home e End e uma parada de Tab na barra;
  - o foco no título ao trocar pelo teclado;
  - o índice com 3 grupos ou mais;
  - o Avançado fechado por padrão, que se abre pelo índice e pela busca;
  - a galeria com 2 fileiras, com a escolha atual sempre visível e expandindo pelas setas.
- **`search.test.tsx`:** a cobertura completa do §8.2 e um pulo para uma linha dentro do Avançado.
- **Rust:** `launcher_pages.rs`, com as seções novas achadas pelo lançador e a lista igual à de `sections.ts`.
- **Visual:** `SettingsSections.test.ts` continua guardando os estilos presos a `.settings-card`. Os estilos novos (faixa, índice, Avançado) ficam presos à janela de Configurações.
- **Teste ao vivo:** nenhum sem o ok do autor na hora.

## 12. Fora do escopo

- Mudar o mecanismo da busca para data-attributes.
- Mexer nos menus do dock ou tirar deles ajustes repetidos.
- O painel antigo de Configurações no dock (`SettingsPanel` sem `grouped`).
- O conteúdo da seção Monitores (F13).
