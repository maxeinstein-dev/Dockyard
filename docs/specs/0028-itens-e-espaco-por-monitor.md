# Spec 0028: Itens e espaço por monitor

F13 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md), que vem da ideia 5 da [referência](../reference/primodock/comportamentos.md). Decisão do autor para a F13: **spec primeiro**. Este texto para nas decisões dele, em §3. Nenhum código entra antes das respostas.

## 1. O que já existe

A [spec 0013](0013-fase-c-espacos-de-trabalho.md) já faz metade da F13:

- **Espaço ativo por monitor.** Cada monitor guarda o seu espaço em `meta` (`workspace:<nome do monitor>`), e trocar num não mexe no outro. A opção "Trocar em todos os monitores" (`switch_every_monitor`, desligada por padrão) faz a troca valer para todos.
- **Janelas por espaço e por monitor.** Trocar esconde só as janelas daquele monitor.
- **Itens por monitor, já hoje, por um caminho indireto.** Como o dock de cada monitor mostra os itens do seu espaço ativo, deixar o monitor 1 em "Trabalho" e o 2 em "Pessoal" já dá itens diferentes em cada um.

O que ainda **não** existe:

- **Bolinhas por monitor.** Todo dock mostra o mesmo conjunto de espaços, então os do outro monitor também aparecem.
- **Monitor fixo num espaço.** A roda, o atalho `Ctrl+Alt+n` e o menu podem tirar um monitor do espaço "dele".
- **Identidade estável do monitor.** A chave é o nome do dispositivo (`\\.\DISPLAY1`). Ele pode mudar ao trocar de cabo, porta ou ordem, e então o monitor "perde" o seu espaço.
- **Configurações › Espaços** não mostra qual espaço está em cada monitor.

## 2. O que a F13 pode acrescentar

| Peça | O que é | Tamanho |
|---|---|---|
| A. Identidade estável | Guardar o espaço pelo identificador do aparelho (o caminho do monitor no EDID, `DISPLAYCONFIG_TARGET_DEVICE_NAME.monitorDevicePath`), com o nome do dispositivo como reserva. Migra a chave antiga. | M |
| B. Espaços de um monitor só | Um espaço pode pertencer a um monitor. As bolinhas, a roda e o atalho de cada dock só andam pelos espaços dele e pelos "de todos". | M |
| C. Monitor fixo | Um monitor pode ficar preso a um espaço: o dock dele não mostra bolinhas e não troca. É o caso "um monitor para cada coisa", sem troca nenhuma. | P |
| D. Itens por monitor dentro do mesmo espaço | Cada item ganha "mostrar em: todos / este monitor". É um terceiro eixo, além de espaço e monitor. | G |
| E. Configurações › Espaços por monitor | Cada monitor numa linha, com o espaço dele, o "fixo" e os espaços que são dele. | M |

## 3. Decisões do autor (2026-11-04)

1. **Peças que entram:**
   - **A**, identidade estável;
   - **C**, monitor fixo;
   - **E**, Configurações por monitor.

   B (espaços de um monitor só) e D (itens por monitor dentro do espaço) ficam de fora.
2. **Monitor que some e volta:** volta com o mesmo espaço, guardado pelo aparelho.
3. **Janela levada a outro monitor:** passa a ser do espaço de lá. Fecha a pergunta 1 da 0013 §6.
4. **Monitor fixo e janelas:** nada é escondido num monitor fixo, e a janela levada a ele passa a ser do espaço dele.
5. **Onde se escolhe:** Configurações › Espaços, numa seção "Monitores", e o menu do dock, com "Fixar este monitor em <espaço>" e "Soltar este monitor".

## 4. Como ficou

- **A, identidade estável.**
  - `windows/monitors.rs` lê `QueryDisplayConfig` (com o resultado guardado por 2 s). A chave do monitor (`Monitor::key`) passa a ser o caminho do aparelho (`monitorDevicePath`, em minúsculas). Na falta dele, fica o nome do dispositivo.
  - `Monitor` ganha `device`, o nome do dispositivo, e `name`, o nome que o Windows mostra.
  - Na abertura, antes de os docks lerem o espaço, `adopt_monitor_keys` passa o espaço e a fixação guardados sob `\\.\DISPLAYn` para a chave nova (`workspaces_repo::adopt_monitor_key`).
  - Um monitor que some e volta reencontra a sua chave em `meta`.
- **C, monitor fixo.**
  - Fica em `meta` como `workspace-fixed:<chave>`.
  - Numa troca (bolinhas, roda, `Ctrl+Alt+n`, menu), `reached` não devolve nada se o monitor que pediu está fixo, e pula os fixos em "trocar em todos os monitores".
  - O dock de um monitor fixo não mostra bolinhas e não lista espaços no menu.
  - O menu do dock ganha "Fixar este monitor neste espaço" e, quando fixo, "Soltar este monitor".
- **E, Configurações.**
  - Espaços ganha o grupo "Monitores": cada monitor (o principal marcado) com o espaço que mostra e a chave "Fixar … neste espaço". O grupo também entra na busca.
  - Comandos `list_monitors`, `set_monitor_workspace` e `set_monitor_fixed`, só na janela de Configurações. Escolher o espaço em Configurações move até um monitor fixo.

## 5. Verificação

- **Testes de unidade:**
  - `workspaces_repo`: o monitor fixo, e a chave nova com a migração da antiga, feita duas vezes sem mudar nada;
  - o grupo "Monitores" de Configurações: lista, troca de espaço e fixação;
  - a busca: o grupo está no índice.
- **Ao vivo (2026-11-05):** build `.medir` e uma cópia dos dados do autor, com dois monitores.
  - "Novo espaço" pelo menu do dock deu duas bolinhas; a bolinha do "Principal" voltou ao primeiro espaço.
  - No banco da cópia, os dois monitores ficaram guardados pelo caminho do aparelho (`workspace:\\?\display#gsm5c0c#…` e `…#auo9bb0#…`), sem nenhuma chave `\\.\DISPLAYn`.
  - "Fixar este monitor neste espaço": as bolinhas sumiram e a fixação ficou no banco. A roda do mouse sobre o dock, três vezes, não trocou o espaço (a foto mostra o FileZilla e nenhuma bolinha; o roteiro tinha a conferência invertida e acusou falha, já corrigido).
  - "Soltar este monitor": as duas bolinhas voltaram.
  - A barra do Windows ficou como estava.
- **Falta conferir à mão:**
  - religar um monitor e ver o espaço voltar;
  - o grupo "Monitores" de Configurações na janela de verdade;
  - "trocar em todos os monitores" pulando um monitor fixo.
