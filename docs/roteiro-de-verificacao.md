# Roteiro de verificação depois da fase D

A rodada única de verificações que o autor decidiu fazer depois da fase D (roadmap, "Verificações pendentes", 2026-10-26). Reúne tudo o que as specs e os PRs marcaram como **não verificado**. Cada passo diz o que fazer e o que deve acontecer.

Use o build de release mais novo, com o mouse e o teclado de verdade. Ao fechar o Dockyard, confira se a barra do Windows voltou como estava. Quando algo falhar, anote o passo, o que aconteceu e, se der, uma captura.

**Legenda:** `[x]` confirmado (com a data e quem confirmou) · `[ ]` a fazer.

## Já confirmado pelo autor (2026-10-26)

- [x] Arrastar para reordenar no dock nativo, com o mouse de verdade: o ícone sobe, os outros abrem espaço, e ao soltar ele desliza até o lugar ([PR #28](https://github.com/maxeinstein-dev/Dockyard/pull/28)).
- [x] Arrastar widgets e apps abertos, e misturar apps e widgets pelo arrasto ([PR #26](https://github.com/maxeinstein-dev/Dockyard/pull/26)).
- [x] Arrastar um ícone do painel da bandeja para o dock ([PR #26](https://github.com/maxeinstein-dev/Dockyard/pull/26)).
- [x] O botão direito num ícone da bandeja no dock abre o menu do programa em cima do ícone ([PR #26](https://github.com/maxeinstein-dev/Dockyard/pull/26)).
- [x] Um dock em cada monitor, nos dois monitores do autor ([PR #27](https://github.com/maxeinstein-dev/Dockyard/pull/27)).
- [x] Ocultar automaticamente: desce, sobe na borda e fica à mostra com o ponteiro em cima ([PR #29](https://github.com/maxeinstein-dev/Dockyard/pull/29)).
- [x] Abrir o DockMax pelo menu Iniciar traz o dock de volta.

## Resultado da rodada de 2026-10-27

O autor rodou os blocos 1 a 4 com o build do main depois do #29.

- **Passou:** tela cheia no segundo monitor com o principal em uso: só o dock do segundo some.
- **Não se repetiu ao testar de novo:** o programa fixado duplicado e o dock voltando por cima da tela cheia no principal. Se voltarem, a sonda `spike_open_and_full_screen` (em `windows/open_apps.rs`) mostra o estado da máquina.
- **Falhou:**
  - Um programa fixado, quando aberto, aparece duas vezes: no grupo dos fixados e no dos abertos.
  - O ícone do próprio DockMax não aparece na bandeja. Causa: o `TaskbarCreated` que a bandeja hospedada reenvia fazia o DockMax pedir as faixas de novo, o que abaixa a bandeja hospedada por um instante, bem quando o ícone do DockMax era recriado: ele ia para o Explorer oculto. **Corrigido** (as faixas só são pedidas de novo quando o Explorer reinicia); falta o autor confirmar.
  - O menu de um ícone da bandeja clicado no dock do segundo monitor abre no monitor principal. Causa: o FortiClient e outros programas "system aware" enxergam a tela na escala do monitor principal, e a posição do ícone ia em pixels físicos (o segundo monitor está em 125 %). **Corrigido no #34**, confirmado pelo autor.
  - Os painéis do Windows (Wi-Fi, volume e bateria) abrem sempre no monitor principal. O Windows só os abre pelo atalho Win+A, sem escolha de monitor.
  - Com tela cheia no monitor principal e o segundo monitor em uso, o dock do principal volta por cima do vídeo.
- **Não feito:** o instalador (não há outra máquina) e o uso longo.
- **Retirado:** "Ícones dos apps". É um seletor de pacotes lidos de uma pasta, e o DockMax não trazia pacote embutido, então só havia "Original". *Atualizado em 2026-10-30:* hoje vêm 15 pacotes embutidos (spec 0019 G4 e spec 0020), e a seção foi vista ao vivo com os 12 de então ([spec 0019](specs/0019-fase-g-extras.md) §7).

## Resultado da rodada de 2026-11-03 (M11)

Com o Dockyard 0.9.0. Uma parte rodou por roteiro no build de teste (`dockyard-e2e.exe`, com uma cópia dos dados e sem mexer na barra do Windows); o resto o autor fez com o mouse de verdade. Os resultados ficam em texto, sem capturas de tela.

- **Por roteiro, passou:**
  - O programa fixado duplicado (que falhou em 2026-10-27) não aparece mais: cada fixado aparece uma vez só.
  - Tela cheia no monitor principal: só o dock dele some, e continua oculto com o foco no segundo monitor.
  - Clique esquerdo: abre o fixado fechado, traz para a frente, minimiza, e alterna entre duas janelas.
  - Configurações aberta e fechada oito vezes, inclusive depois de o WebView2 aquecido sair e com dois cliques seguidos: abriu todas, em cerca de 0,5 s.
- **Pelo autor, passou:** a atualização automática para a 0.9.0, os cliques do meio e direito, a aparência, as áreas de trabalho virtuais, ligar e desligar monitores e o Narrador.
- **Falhou e foi corrigido:**
  - Com o dock estreito, a dica e os nomes dos programas ficavam cortados: [PR #145](https://github.com/maxeinstein-dev/Dockyard/pull/145).
  - Um `.exe` solto no dock entrava sempre no fim, e não onde foi solto: [PR #147](https://github.com/maxeinstein-dev/Dockyard/pull/147). Soltar um já fixado, soltar vários de uma vez e soltar no segundo monitor: [PR #150](https://github.com/maxeinstein-dev/Dockyard/pull/150). Confirmados pelo autor.
  - Configurações e o menu do botão direito num fixado não abriam. Causa: o disco estava cheio, e o WebView2 não conseguia criar a janela. Desde o [PR #148](https://github.com/maxeinstein-dev/Dockyard/pull/148), se o WebView2 falhar, o dock mostra um menu nativo no lugar e avisa uma vez. Confirmado pelo autor.
- **Não deu para ver:** o Bloco de Notas não pergunta se quer salvar, porque salva sozinho.
- **Uso longo:** 2 h 25 min com o build de teste, com uma amostra a cada 5 min.
  - Memória privada do dock: 96,8 MB aos 5 min e 97,9 MB no fim.
  - CPU em repouso: de 0,03 % a 0,05 %.
  - WebView2: 5 processos e 126 a 127 MB do começo ao fim.

## Resultado da 0.9.2 (2026-11-04)

O autor atualizou da 0.9.1 para a 0.9.2 pelo aviso do próprio app.

- **Passou:**
  - O painel do Calendário abre ao clicar no widget ([PR #156](https://github.com/maxeinstein-dev/Dockyard/pull/156)).
  - O título do compromisso corre como letreiro, com letras maiores ([PR #159](https://github.com/maxeinstein-dev/Dockyard/pull/159)).
- **Esperado:** a atualização ainda mostrou a janela do instalador, porque quem instala é a versão antiga (a 0.9.1, no modo passivo).

## 7. Pendências da 0.9.2

- [ ] **Atualização sem janela** ([PR #157](https://github.com/maxeinstein-dev/Dockyard/pull/157)): ao atualizar da 0.9.2 para a próxima, o instalador roda sem janela e o Dockyard volta sozinho.
- [ ] **Entrar no Claude pelo painel Uso do Claude** ([PR #158](https://github.com/maxeinstein-dev/Dockyard/pull/158)): o terminal abre com o login e fecha sozinho quando ele termina, e o painel passa a mostrar o uso. Se o login falhar, o terminal fica aberto com o erro. *Não testado: o botão só aparece com o Claude Code deslogado ou com o acesso vencido, e o autor preferiu não deslogar para não interromper as sessões em andamento. Testar na próxima vez que o acesso vencer.*

## 1. Cliques no dock (uns 10 minutos)

- [x] **Esquerdo num programa fixado fechado:** abre o programa, com a animação de abertura escolhida. (Roteiro, 2026-11-03.)
- [x] **Esquerdo num programa aberto que não está na frente:** traz a janela para a frente. (Roteiro, 2026-11-03.)
- [x] **Esquerdo de novo, com ele na frente:** minimiza. (Roteiro, 2026-11-03.)
- [x] **Esquerdo num programa com várias janelas:** passa para a próxima janela a cada clique. (Roteiro, 2026-11-03.)
- [x] **Meio:** abre uma nova instância (ou nada, se "Botão do meio" estiver em "Nada"). (Autor, 2026-11-03.)
- [x] **Direito num programa fixado:** abre o menu com a Jump List, "Fechar janela" e "Remover do dock". (Autor, 2026-11-03, depois do #148.)
- [x] **Direito num programa aberto não fixado:** abre o menu com nova instância, fechar janela e "Fixar no dock". (Autor, 2026-11-03.)
- [ ] **Fechar janela pelo menu**, num programa com algo não salvo (o Bloco de Notas com texto novo, por exemplo): o programa pergunta se quer salvar, e nada se perde. *O Bloco de Notas salva sozinho; falta um programa que pergunte.*
- [x] **Direito no espaço vazio do dock:** o menu tem Gerenciador de Tarefas, Configurações, Ocultar automaticamente (com a marca) e Sair, e cada um faz o que diz. (Autor, 2026-11-03, depois do #148.)
- [x] **Menu do ícone do Dockyard na área de notificação:** os mesmos itens, e a marca de "Ocultar automaticamente" acompanha a configuração. (Autor, 2026-11-03.)

## 2. Aparência do dock nativo (uns 15 minutos)

- [x] Os três efeitos ao passar o mouse (Ampliação, Escala, Nenhum), com o mouse passando devagar e depressa. (Autor, 2026-11-03.)
- [x] Os oito estilos, um por um, e os reflexos onde o estilo tem. (Autor, 2026-11-03.)
- [x] O dock na **borda de cima**: posição, faixa reservada, menus abrindo para baixo, arrasto, ocultar automaticamente. (Autor, 2026-11-03.)
- [x] **Escala de 150 %** num monitor: o dock nítido, no tamanho certo e com a faixa certa. (Autor, 2026-11-04.)
- [x] As opções novas de Configurações (Dock em todos os monitores, Ocultar automaticamente e os dois tempos), vistas no app de Configurações de verdade. (Autor, 2026-11-03.)

## 3. Arquivos e janelas (uns 10 minutos)

- [x] **Soltar no dock**, vindo do Explorador de Arquivos, um `.exe` e um atalho `.lnk`: os dois são fixados. Um `.txt` é ignorado. (Autor, 2026-11-03, depois do #147 e do #150.)
- [x] **Áreas de trabalho virtuais do Windows** (Win+Tab › Nova área de trabalho): o dock aparece nas duas, e os programas abertos de cada área se comportam como esperado. (Autor, 2026-11-03.)

## 4. Dois monitores (uns 10 minutos)

- [ ] O mouse no **segundo dock**: abrir um painel (abre no monitor dele), um menu e um arrasto. *Soltar um arquivo no segundo dock foi confirmado em 2026-11-03 (#150); falta o painel e o menu.*
- [x] **Ligar e desligar um monitor** com o Dockyard aberto: o dock dele some e volta, e a faixa é devolvida. (Autor, 2026-11-03.)
- [x] Desligar e religar "Dock em todos os monitores" com o Dockyard aberto. (Autor, 2026-11-03.)
- [x] **Tela cheia de verdade no segundo monitor** (um vídeo do YouTube em tela cheia): só o dock daquele monitor some, e usar o outro monitor não o traz de volta por cima do vídeo. (Autor, 2026-10-27.)
- [x] **Tela cheia no monitor principal**, idem. (Roteiro, 2026-11-03.) Um jogo em tela cheia exclusiva não foi testado.

## 5. Instalador (uns 20 minutos, numa máquina limpa ou virtual)

- [ ] Instalar numa máquina limpa ou numa máquina virtual com Windows 11.
- [ ] Instalar no **Windows 10**: o que funciona e o que não.
- [x] Atualizar por cima de uma versão anterior **com o Dockyard aberto**. (Autor, 2026-11-03: para a 0.9.0, pelo aviso de atualização do próprio app.)
- [ ] Desinstalar: o programa sai, os dados ficam (a pasta em `%APPDATA%`), e a barra do Windows volta.

## 6. Longo prazo

- [x] **Uso longo (algumas horas)**: memória privada e CPU em repouso no começo e no fim (o Gerenciador de Tarefas, aba Detalhes, coluna "Memória (conjunto de trabalho privado)"). (Roteiro, 2026-11-03: 2 h 25 min, números na rodada acima.)
- [x] **Leitor de tela (Narrador, Ctrl+Win+Enter):** o dock é lido como barra de ferramentas, cada item pelo nome, e "Invocar" age como o clique. Os painéis e o app de Configurações são navegáveis pelo teclado. (Autor, 2026-11-03.)

## 8. Dock na lateral (uns 20 minutos, spec 0028)

> **Já visto ao vivo em 2026-11-05** ([spec 0028](specs/0028-dock-lateral.md), PR #194), na esquerda e na direita:
> - a coluna e a ordem, a dica e o menu do lado de dentro, e o arrasto de apps;
> - a faixa reservada ao lado da coluna, e a janela maximizada fora dela;
> - ↑ e ↓ pelo atalho, e a prévia e o painel do lado de dentro;
> - o ocultar automaticamente sem faixa, e a barra do Windows devolvida.
>
> A bandeja, o relógio e os widgets em pé foram vistos em fotos fora da tela. Daqui, falta sobretudo o 9 (skins ao vivo), o 11 (todos os monitores com a borda na lateral), a bandeja, o relógio e os widgets em pé na tela de verdade.

Em Configurações › Geral › Dock › Posição, passe por **Esquerda** e **Direita**, e no fim volte para a posição de antes. Em cada lateral:

1. **A coluna:**
   - na esquerda, de cima para baixo: bolinhas, Lançador, apps, widgets, bandeja e relógio;
   - na direita, a ordem ao contrário: relógio no topo, bolinhas no pé;
   - o indicador do app aberto fica do lado da tela;
   - os separadores atravessam a coluna;
   - o reflexo sai para o lado da tela.
2. **A dica e a ampliação:** passe o mouse pela coluna. A dica fica ao lado do ícone, do lado de dentro da tela, e a ampliação cresce ao longo da coluna.
3. **O relógio:** a hora sobre a data curta (`dd/mm`); com o mouse em cima, a dica mostra a data completa.
4. **A bandeja e as bolinhas:** a bandeja em duas colunas que descem; as bolinhas lado a lado; clicar numa bolinha troca de espaço.
5. **Arrastar e soltar:**
   - arraste um app para cima e para baixo: os vizinhos abrem espaço e ele fica onde foi solto;
   - solte um arquivo entre dois apps fixados: ele é fixado ali.
6. **Menus, painel e prévias:**
   - o menu do botão direito abre para o lado de dentro;
   - o painel de um widget abre ao lado da coluna, crescendo do lado dela;
   - a prévia de um app aberto desliza vinda da coluna;
   - o cartão do Calendário fica ao lado.
7. **Teclado:**
   - pelo atalho do dock, ↑ e ↓ andam na coluna (na direita, ↑ sobe na tela);
   - a seta para dentro da tela abre as prévias;
   - o Esc volta.
8. **Widgets em pé:**
   - Atividade, Uso do Claude, Atividade de rede, Calendário, Tocando agora (os botões numa coluna embaixo) e Clima;
   - o Relógio mundial com uma cidade por ícone;
   - a Lista de tarefas e a Contagem regressiva com o texto girado.
9. **Skins:**
   - uma embutida (Medieval, por exemplo) gira com a coluna;
   - uma skin da pasta com `vertical` usa a arte em pé.
10. **Ocultar automaticamente e a faixa reservada:**
    - com ocultar automaticamente, a coluna sai pela lateral e volta ao encostar o ponteiro nela;
    - sem ele, uma janela maximizada não fica por baixo da coluna.
11. **Dois monitores**, se houver: a coluna fica na borda de fora de cada monitor.

Volte à posição de antes. A barra do Windows tem de ficar como estava.

## 9. Antes da 1.0 (F1 a F13 do estudo 0002)

O que os testes ao vivo das funcionalidades não cobriram. Cada passo diz de onde vem (spec ou PR). O dock lateral está no §8. Os passos que pedem o log se referem a `%LOCALAPPDATA%\dev.maxsueleinstein.dockmax\logs\dockmax.log`. O "primeiro `*.log`" da pasta é do WebView2.

**F1 Economia de bateria** (spec 0021 §4)
- [ ] Com "Na bateria" e o Tocando agora de título longo, tire o carregador: o letreiro para, e um app fixado abre sem pulo. Ponha o carregador de volta: o letreiro volta.
- [ ] Com "Sempre", na tomada: o letreiro fica parado e o clique abre sem pulo.
- [ ] No log aparecem `dock: battery saver on/off`.

**F2 Espaçadores** (spec 0022 §7)
- [ ] O espaçador pequeno, pelo menu do dock, abre um vão de ¼ de ícone. Até hoje só o grande foi visto.
- [ ] O mouse sobre o vão: nada amplia, sem dica.
- [ ] Arrastar o vão entre dois apps. Ele fica só no espaço dele.
- [ ] Configurações › Itens: os espaçadores sobem, descem e saem.
- [ ] Narrador: as setas pulam o vão.
- [ ] Nas bordas de cima e de baixo, nos três tamanhos.
- [ ] O log, sem WARN.

**F3 Pilhas de widgets** (spec 0023 §6)
- [ ] Configurações › Itens: a linha "Pilha: …" sobe, desce e se desfaz.
- [ ] O Mídia por baixo de uma pilha não lê nada até ficar por cima.
- [ ] A roda sem Ctrl em cima da pilha troca de espaço, como antes.
- [ ] Os pontinhos: o de cima, maior, se distingue dos outros.
- [ ] O log, sem WARN.

**F4 Ícone por app** (spec 0024 §6)
- [ ] Um pacote só para um app: os outros não mudam.
- [ ] "Original do Windows" com um pacote no dock: só aquele app volta.
- [ ] Trocar o pacote do dock não mexe numa imagem própria.
- [ ] Escolher um ICO.

**F5 Downloads, Lixeira e pastas** (spec 0025 §6)
- [ ] Soltar uma pasta no dock: ela entra onde foi solta.
- [ ] No painel da pasta, um arquivo abre e "Abrir no Explorador" abre a pasta.
- [ ] "Esvaziar a Lixeira…" › "Não": nada é esvaziado.

**F6 Busca em Configurações** (PR #163, spec 0029 §11)
- [ ] O campo na barra lateral e a marca na linha encontrada.
- [ ] Buscar algo do Avançado: o grupo abre e o foco vai à linha.
- [ ] Buscar linhas de Apps e de Contas.
- [ ] Teclado e Narrador no campo.

**F7 Espaços e programas** (PRs #164 e #168)
- [ ] Duplicar um espaço: o novo tem os mesmos itens.
- [ ] "Nova janela" no menu de um app aberto.
- [ ] "Procurar…" fixa um `.exe` de uma pasta portátil.

**F8 Exportar e importar** (spec 0026)
- [ ] Importar a cópia num computador sem dados do Dockyard.
- [ ] Importar uma cópia com ícones próprios: eles aparecem no dock.

**F9 Widgets novos** (PRs #169, #171, #173 e #175)
- [ ] Layout do teclado: o tile desenhado, e um clique troca o layout.
- [ ] Relógio mundial: o cartão largo com vários fusos.
- [ ] Capturas de tela: o painel e o arrasto de uma captura para fora.
- [ ] Arquivos recentes: o painel e o arrasto de um arquivo.
- [ ] A memória com cada um no dock.

**F10 Prévias pelo teclado e Espiar** (spec 0027)
- [ ] Narrador lendo a miniatura escolhida pelo teclado. A UI Automation ainda dá o item do dock como focado, e o anúncio vem do evento de foco do cartão.
- [ ] O Espiar com o ponteiro numa miniatura.

**F11 Selos** (spec 0016 §8 e §9)
- [ ] O "9+" do Discord desenhado no dock.
- [ ] Um ícone da bandeja com selo: um programa só na bandeja, sem item no dock.
- [ ] Um selo sem número de verdade (um app que escreva só "New messages").
- [ ] Um selo mudando com a barra do Windows recolhida: atualiza em até 10 s.
- [ ] O Narrador lendo "…, N notificações" no item.

**F12 Dock lateral** (spec 0028): o que falta do §8 deste roteiro (a nota no topo dele diz o que já foi visto), e mais:
- [ ] Uma skin embutida (a Medieval, por exemplo) na lateral, girando com a coluna, e uma skin da pasta Skins com `vertical` no `skin.json`, usando a arte em pé (spec 0020, PR #187).
- [ ] "Dock em todos os monitores" com a borda numa lateral: a coluna na borda de fora de cada monitor.
- [ ] Os widgets em pé na coluna, de perto:
  - Atividade, Uso do Claude, Atividade de rede, Calendário e Clima;
  - Tocando agora, com os botões numa coluna embaixo;
  - Relógio mundial, com uma cidade por ícone;
  - Lista de tarefas e Contagem regressiva, com o texto girado.
- [ ] Escala de 150%, um app em tela cheia e o Narrador nas duas laterais.

**F13 Monitores** (spec 0030 §5)
- [ ] Desligar e religar um monitor: o espaço dele volta.
- [ ] Configurações › Espaços › Monitores: a lista, trocar o espaço e fixar.
- [ ] "Trocar em todos os monitores" pula um monitor fixo.

**Configurações reorganizadas** (spec 0029)
- [ ] A barra lateral com as faixas: setas, Home, End e a parada do Tab.
- [ ] O foco vai ao título ao trocar de seção.
- [ ] O índice no topo das seções com três grupos ou mais.
- [ ] O Avançado fechado, que abre pelo índice e pela busca.
- [ ] As galerias de tema e estilo em duas fileiras, com "Mostrar todos".
- [ ] As dicas enxutas e a etiqueta "Padrão".
- [ ] Comportamento e Lançador como seções, e o lançador achando as seções novas.
- [ ] O Narrador na barra lateral e no índice.
- [ ] "Dock em todos os monitores" em Geral › Dock (PR #192, Q13).
- [ ] "Trocar de espaço em todos os monitores" no alto de Espaços › Monitores, e nenhum switch de monitor em Comportamento (PR #192, Q13).
- [ ] Espaços: a linha "Como trocar de espaço", com a dica "Ctrl+Alt+1 a 9, a roda do mouse no dock ou as bolinhas", no lugar do parágrafo longo, e a nota "Editar aqui não muda o que o dock mostra." no cartão do espaço (PR #192, Q14).

**Barra do Windows** (PR #195): a conferência de "Depois da rodada", no fim de cada rodada, nos dois monitores.

## Depois da rodada

**A barra do Windows, no fim de toda rodada ao vivo** (roteiro manual ou script):
1. Troque a janela da frente para uma que não esteja em tela cheia. Enquanto o Explorer acha que há um programa em tela cheia, ele tira a barra de cima de propósito; foi o que se viu em 2026-11-05, depois de uma janela maximizada com a barra oculta.
2. Confira, contra o que era antes da rodada:
   - o estado de ocultar automaticamente (`ABM_GETSTATE`);
   - se a `Shell_TrayWnd` está visível;
   - o estilo estendido inteiro de cada barra, incluindo o "sempre por cima" (`WS_EX_TOPMOST`, 0x8), e sem `WS_EX_LAYERED|WS_EX_TRANSPARENT` (0x80020).
3. Se algo ficou diferente, a rodada falha e diz o quê. Não force o "sempre por cima" à mão numa barra cujo estilo de antes não foi lido. Depois de fechar, o vigia do Dockyard ainda põe a barra de volta em cima por 10 segundos, então confira depois disso.


Cada passo que falhar vira um item com o defeito descrito; os que passarem ficam marcados aqui, com a data. O roadmap aponta para este arquivo.
