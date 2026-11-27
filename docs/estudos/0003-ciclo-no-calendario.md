# Estudo 0003 — O Ciclo dentro do Calendário

Este é um levantamento de 2026-11-27, feito só como estudo: nada do app mudou. As fontes foram a [spec 0017](../specs/0017-fase-f-widgets.md) §6c (Calendário e Ciclo), o [guia do Atalho](../guia-atalho-ciclo.md) e o código no commit `3abd41a`: `src-tauri/src/domain/cycle.rs`, `src-tauri/src/windows/calendar.rs`, `src-tauri/src/windows/native_dock/faces.rs` e `src/widgets/cycle/`.

## 1. O problema

O widget Ciclo funciona, mas não agrega valor para o autor.

- **O anel e o número não ajudam a agir.** O tile mostra a fração do ciclo e os dias até a próxima menstruação (ou `•2` durante ela). O que o autor precisa no dia a dia é saber se algum plano cai num dia difícil, e não quantos dias faltam.
- **O painel é uma ficha de estatística.** Ciclo médio, duração média e a última data interessam a quem menstrua, não a quem planeja junto.
- **O atraso não aparece.** Depois da data prevista, sem nova marca, a fase fica em "prevista para estes dias" e o tile mostra `0` indefinidamente (`days.max(0)` em `faces.rs`).
- **A TPM é um número fixo** de dias antes, ajustado à mão. Isso continua, porque ela não vai marcar nada além do 🌸.

O que funciona e fica:
- ela marca só o 🌸, no primeiro dia, e estica o evento até o último dia;
- a previsão é a média dos últimos 6 ciclos;
- os avisos aparecem como selo e balanço no dock.

## 2. As opções que foram avaliadas

| | Ideia | Decisão |
|---|---|---|
| A | Previsão como faixa de dias e atraso explícito | Entra só o atraso (§3.6) |
| B | Um 2º símbolo para ela marcar a TPM | **Descartada.** Ela não deve ter de marcar mais nada. |
| **C** | **Cruzar a previsão com a agenda** | **Escolhida** |
| D | Lista "como cuidar hoje" por fase | Fora por ora |
| E | Linha do tempo com as fases no anel | Fora, porque o anel sai |
| F | O autor também marca o 🌸 | Fora |
| **G** | **Acabar com o widget próprio e levar o Ciclo para o Calendário** | **Escolhida** |

C e G se encaixam porque o Calendário já lê a agenda dela inteira, "para ver se os compromissos dela batem com os dele" (spec 0017), e já esconde os eventos 🌸.

## 3. A proposta

### 3.1 O que sai

- O widget **Ciclo** deixa o catálogo, junto com o tile, o anel nativo (`faces::cycle`), o painel e os ajustes.
- As instâncias que já existem são migradas para o Calendário (§3.5).

### 3.2 Ajustes do Calendário: uma seção "Ciclo"

- **Mostrar o ciclo no Calendário:** desligado por padrão. A migração liga para quem já tinha o Ciclo.
- Os ajustes do Ciclo vêm iguais para esta seção: símbolo das marcas, agenda das marcas, TPM (dias antes), avisos (TPM, véspera, dia, "também no Windows") e lembretes livres.

### 3.3 Painel do Calendário (decisão do autor: painel e um ponto no cartão)

No topo, antes de hoje e amanhã, entra uma seção **Ciclo**:

1. **A fase em uma linha**, como no painel atual: "TPM · a menstruação deve vir em 3 dias".
2. **Hoje**: os avisos e os lembretes do dia, como hoje.
3. **Coincidem**: os compromissos das agendas ligadas no Calendário (as dele e a dela) que caem nos dias previstos. Cada linha tem a data, a cor da agenda, o título e a fase: "TPM" ou "menstruação · 2º dia". Um clique abre os detalhes, como em qualquer evento do painel.
4. **Próxima prevista**: uma linha com a data. As médias saem do painel principal e ficam numa linha de rodapé ("com base nos últimos N ciclos").

### 3.4 Cartão e tile

- **No cartão** (o mouse em cima do Calendário), um ponto na cor do Ciclo aparece ao lado de cada evento que cai em dia previsto, **sem nenhuma palavra**. Vale só para os eventos que o cartão já mostra no período escolhido.
- **No tile**, o selo e o balanço às 9h nos dias de aviso, como o Ciclo faz hoje (decisão do autor: os avisos vão para o Calendário). O motivo aparece só no painel, e o selo some ao abri-lo.

### 3.5 Migração dos dados

- O `CycleConfig` de cada instância de Ciclo vai para dentro da config do **primeiro Calendário do mesmo dock**, com "mostrar" ligado.
- **Se o dock não tiver Calendário**, a instância de Ciclo vira um Calendário no mesmo lugar, com o ciclo ligado.
- **Se houver dois Ciclos no mesmo dock**, vale o primeiro, e o fato fica registrado no log.
- A migração roda uma vez, como as outras do SQLite ([ADR 0008](../adr/0008-persistencia-sqlite-e-widgets.md)), e as fixtures de contrato mudam junto.

### 3.6 Até onde olhar e o atraso

- **Horizonte** (decisão do autor): de hoje até o fim da próxima menstruação prevista, cerca de 2 a 4 semanas.
- O Calendário hoje lê só hoje e amanhã, a cada minuto. O horizonte vira **uma segunda leitura**, das mesmas agendas, a cada 30 minutos (a cadência da leitura das marcas) e só com o ciclo ligado. Como hoje, nada vai para o disco.
- **Atraso:** passada a data prevista sem nova marca, a linha diz "Atrasou N dias" e a lista Coincidem passa a cobrir os próximos dias como menstruação possível. O tile não muda: não há aviso de atraso, como a spec 0017 já decidia.

## 4. Recortes (cada um com TDD, e os de interface medindo a memória do dock)

1. **Domínio:** os dias previstos (TPM e menstruação) de hoje até o fim da próxima, o atraso, e se um evento (com hora ou de dia inteiro, de vários dias) cai neles. São regras puras em `domain/cycle.rs`, com testes primeiro.
2. **Config e migração:** a seção Ciclo no `CalendarConfig`, a migração do SQLite e as fixtures. Os testes da migração cobrem os casos do §3.5.
3. **Leitura e painel:** a segunda leitura do horizonte em `windows/calendar.rs` e a seção Ciclo no `CalendarPanel`, com testes do painel.
4. **Cartão, tile e limpeza:** o ponto no cartão (`windows/agenda.rs`), os avisos movidos para o tile do Calendário, a remoção do widget Ciclo e a atualização dos docs (spec 0017 §6c, o guia do Atalho, as limitações e o roadmap).

A spec que saiu deste estudo é a [0031](../specs/0031-ciclo-no-calendario.md), que também fecha os pontos abaixo.

## 5. Pontos em aberto

- **Calibrar a TPM:** a conferência com as marcas reais fica fora do repositório ([spec 0031](../specs/0031-ciclo-no-calendario.md) §6).
- **Lista Coincidem:** decidir se ela mostra os eventos recusados (hoje o painel os esconde) e os de dia inteiro que não são compromisso, como feriados.
