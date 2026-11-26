# Spec 0021 — Economia de bateria

Item F1 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md). O PrimoDock tem a mesma chave em Geral › App ([spec 0004](0004-paridade-primodock.md) §3.7b).

## 1. A configuração

`powerSaver` em `settings.json`, em Configurações › Geral › Comportamento (e no painel de Configurações do dock), como um controle segmentado **Economia de bateria**:

| Valor | Rótulo | Quando economiza |
|---|---|---|
| `off` | Desligada | nunca |
| `onBattery` | Na bateria | enquanto o computador está fora da tomada ou a economia de energia do Windows está ligada |
| `always` | Sempre | sempre |

- **O padrão é Na bateria** (decisão do autor, 2026-11-03). Um `settings.json` de antes desta spec, sem o campo, lê como `onBattery`.
- "Fora da tomada" é `ACLineStatus == 0` em `GetSystemPowerStatus`. Um desktop (sem bateria) ou uma linha que o Windows não sabe dizer (255) nunca conta como bateria.
- **A economia de energia do Windows conta como bateria** (pedido do autor, 2026-11-05): com ela ligada (`SystemStatusFlag == 1` na economia de bateria antiga; no Windows 11, o aviso `GUID_ENERGY_SAVER_STATUS` > 0, por `PowerSettingRegisterNotification`, já que o sinalizador fica 0 e o modo efetivo diz só "melhor bateria"), "Na bateria" economiza mesmo na tomada. "Desligada" continua sem economizar.
- A regra é uma função pura, `domain::power::saving(modo, na_bateria)`, com testes; `windows::power::decide` passa "fora da tomada ou economia do Windows" como `na_bateria`.
- **O ícone da bateria** na ponta do dock fica laranja enquanto a economia do Windows está ligada e o computador está fora da tomada, como o da barra do Windows, e a dica ganha "(economia de energia)".

## 2. O que para

Com a economia ligada:

- **o letreiro** do Tocando agora (o título longo que rola): o texto fica parado no começo e o `MARQUEE_TIMER` não é armado;
- **o pulo ao abrir** um programa (o `OpenAnimation` escolhido: pular, pulsar ou tremer) e o pulo de um widget com aviso (Ciclo, Pomodoro): o selo do aviso continua aparecendo;
- o dock não tem miniaturas animadas nem animação nas prévias de janela hoje, então não há mais nada a parar.

Continua igual o que responde ao usuário: cliques, o efeito ao passar o mouse, arrastar, o auto-ocultar e o relógio, que repinta na virada do minuto. Quem quiser menos movimento ainda tem a opção de animações do Windows, que o dock já segue.

## 3. Intervalos das consultas

`domain::power::every(consulta, economizando)`:

| Consulta | Normal | Economizando |
|---|---|---|
| Status (bateria, rede, volume na ponta do dock) | 3 s | 30 s |
| Atividade (processador, memória e rede dos widgets) | 2 s | 6 s |
| Tocando agora (mídia) | 1 s | 3 s |
| Clima (Open-Meteo) | 30 min | 2 h |
| Atualização (GitHub) | 6 h | 24 h |

Cada consulta fica de 3 a 10 vezes mais rara, e nenhuma para. O que o usuário pede continua imediato: abrir o painel do Clima lê de novo uma cidade com mais de 10 minutos, e "Procurar atualizações" em Sobre procura na hora. A primeira busca de atualização, 15 s depois de abrir, não muda.

## 4. Mudanças ao vivo

- **Na troca da configuração**, o `settings-changed` já faz cada dock ler os dados de novo (`Dock::reload`); ali o dock decide de novo.
- **Na troca da tomada** (ou da economia do Windows), o dock trata `WM_POWERBROADCAST` com `PBT_APMPOWERSTATUSCHANGE`, que o Windows manda às janelas de primeiro nível (as do dock são). Ali ele também relê a bateria, a rede e o volume na hora (`forget_shared` e `read_system`): antes, o ícone da bateria esperava o próximo tique, que fora da tomada é de 30 s (achado do autor, 2026-11-05). Como reserva, caso a mensagem não venha, o tique do status (`read_system`) decide de novo a cada leitura: a chamada é só `GetSystemPowerStatus`, barata. Assim, ao tirar o carregador, a economia entra na hora (ou em até 3 s pela reserva); ao pôr de volta, na hora (ou em até 30 s).
- Quando a decisão muda, o dock arma de novo os timers do status e da atividade com o intervalo novo, limpa os pulos em andamento e repinta (o letreiro para no começo ou volta a rolar).
- As threads de mídia, clima e atualização leem a decisão (`windows::power::saving`, um `AtomicBool`) a cada volta. Para que uma espera longa não segure o intervalo antigo, o Clima e a atualização esperam em pedaços de no máximo 10 minutos (`RECHECK`) e recalculam o que falta.

## 5. Onde mexe

`domain/power.rs` (regra e tabela), `domain/settings.rs` (`PowerSaver`), `commands/settings.rs` (`SettingsInput`), `windows/power.rs` (estado atual), `windows/system_status.rs` (`on_battery`), `windows/native_dock/mod.rs`, `windows/media.rs`, `windows/weather.rs`, `windows/update.rs`, `domain/update.rs`; na interface, `src/core/settings.ts`, `SettingsPanel.tsx` (os dois layouts), `OptionGroup.tsx` (ganhou a linha de dica) e os dicionários.

## 6. Verificação

- Testes de unidade: a regra nos três modos, a tabela (normal igual ao de antes, economizando mais raro e o status a 30 s), o padrão e o campo ausente em `settings.json`, o `SettingsInput` com o campo, a espera da atualização com o intervalo longo, `ACLineStatus`.
- Testes da interface: o controle nos dois layouts, com as três opções, o padrão marcado, a dica e a escrita de `powerSaver`.
- **Verificado ao vivo** (2026-11-03, build de lançamento, cópia dos dados do autor, na tomada): com o dock parado por 97 s, **Sempre** gastou 219 ms de processador e 104 trocas de contexto/s, contra 375 ms e 125/s com **Desligada** (−42 % de processador). O app abriu e saiu pelo menu nos dois modos, e a barra do Windows ficou igual. O log não registra `battery saver on` quando a economia já começa ligada: a linha só sai quando ela muda.
- **Falta ver ao vivo** (precisa tirar o carregador): com um Tocando agora de título longo e Na bateria, tirar o carregador e ver o letreiro parar e o clique num app fixado abrir sem pulo; pôr o carregador e ver o letreiro voltar; com Sempre, o mesmo na tomada; o log mostra `dock: battery saver on/off`.
