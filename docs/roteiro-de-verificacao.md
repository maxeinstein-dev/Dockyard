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

## Depois da rodada

Cada passo que falhar vira um item com o defeito descrito; os que passarem ficam marcados aqui, com a data. O roadmap aponta para este arquivo.
