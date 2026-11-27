# Spec 0031: O Ciclo dentro do Calendário

Esta spec vem do [estudo 0003](../estudos/0003-ciclo-no-calendario.md) e substitui o widget Ciclo da [spec 0017](0017-fase-f-widgets.md) (F2b). Ela junta duas ideias do estudo:

- **C:** cruzar a previsão com a agenda;
- **G:** levar o Ciclo para dentro do Calendário.

**Status:** decidida em 2026-11-27. Nenhum código entrou ainda.

## 1. Por que

- **O widget não agrega para o autor.** O anel e o número do tile não ajudam a agir, e o painel é uma ficha de médias.
- **O que importa é planejar.** O autor quer saber se um compromisso dele ou dela cai na TPM ou na menstruação.
- **O Calendário já tem quase tudo:**
  - a agenda dela entra inteira nele ("para ver se os compromissos dela batem com os dele", spec 0017);
  - os eventos 🌸 já ficam escondidos nele;
  - a thread dele já lê as marcas do Ciclo (`windows/calendar.rs`).

**O que não muda:**
- Ela marca **só o 🌸**, no primeiro dia, e estica o evento até o último. Nada nesta spec pede que ela marque outra coisa.
- A previsão segue a mesma: média dos últimos 6 ciclos, 28 e 5 dias sem histórico, e a TPM N dias antes.
- Nada vai para o disco além dos ajustes.

## 2. Decisões do autor (2026-11-27)

1. **O widget Ciclo acaba.** O ciclo passa a ser uma seção do Calendário.
2. **Visibilidade:**
   - no **painel**, tudo;
   - no **cartão** (o mouse em cima), só um ponto colorido ao lado dos eventos que caem em dia previsto, sem palavra;
   - no **tile**, o selo de aviso e, durante a fase, um ponto fixo (item 6).
3. **Horizonte do cruzamento:** de hoje até o fim da próxima menstruação prevista.
4. **Avisos:** vão para o Calendário. São o selo e o balanço às 9h no início da TPM, na véspera e no dia, mais os lembretes livres e a opção "Também no Windows".
5. **Ela não marca mais nada.** O 2º símbolo para a TPM foi descartado.
6. **Ponto fixo durante a fase** (decisão do mesmo dia). No widget Ciclo, o aviso só existe em três dias, e o selo some ao abrir o painel e não volta. Nos outros dias da TPM e da menstruação, o dock não mostra nada. No Calendário, um ponto discreto fica no tile durante toda a fase.

## 3. Como fica

### 3.1 Ajustes do Calendário › Ciclo

- **Mostrar o ciclo no Calendário:** desligado por padrão. Ligado, aparecem os mesmos ajustes do Ciclo de hoje:
  - símbolo das marcas;
  - agenda das marcas;
  - TPM (dias antes);
  - avisos;
  - lembretes.
- No `CalendarConfig`, entra `cycle: Option<CycleConfig>`, em que `None` quer dizer desligado. O `CycleConfig` e a validação dele ficam como estão.
- Uma config antiga sem o campo lê como `None`, porque o `CalendarConfig` já usa `#[serde(default)]`.

### 3.2 Domínio (`domain/cycle.rs`)

São regras puras, com testes antes do código.

- **Dias previstos:** `ahead(periods, today, sensitive) -> Vec<(dia, Expected)>`. Cobre de hoje até o fim da próxima menstruação prevista.
  - **`Expected`:** `Sensitive` (TPM) ou `Period { day }`, com `day` vazio quando é só uma menstruação possível.
  - **Se ela está menstruada hoje,** os dias que faltam dessa menstruação entram antes.
- **Atraso:**
  - Passada a data prevista sem marca nova, `late_by(status)` diz quantos dias de atraso. O `status` não ganhou uma fase nova: ele já conta os dias negativos, e uma fase nova mudaria o widget Ciclo antes da hora.
  - Durante o atraso, `ahead` trata hoje e os próximos `período médio − 1` dias como menstruação possível, com `dia` desconhecido.
  - A spec 0017 não tinha aviso de atraso, e continua sem ter.
- **Coincidência:** `overlaps(início, fim, offset, dias) -> Option<(dia, Expected)>` diz o primeiro dia previsto em que o evento cai, e o que ele é.
  - Vale para evento com hora e para evento de dia inteiro de vários dias.
  - O fim de um evento de dia inteiro é exclusivo, como no Google.

### 3.3 Leitura (`windows/calendar.rs`)

- **Duas leituras.** As duas são das agendas não escondidas no Calendário, as mesmas do painel.
  - **Hoje e amanhã:** continua a cada minuto.
  - **Horizonte:** é nova, a cada 30 minutos (a mesma cadência das marcas). Só roda com o ciclo ligado em algum Calendário.
- **O que fica de fora do horizonte:**
  - os eventos recusados, como no painel;
  - as agendas de feriados do Google (id com `#holiday@group.v.calendar.google.com`);
  - os próprios 🌸.
- **O que fica em memória:** o horizonte, como as outras leituras, e nada vai para o disco. Sem internet, vale a última leitura.
- **Avisos:** `notify` passa a percorrer os Calendários com o ciclo ligado, em vez dos widgets Ciclo. As chaves de aviso já dados seguem as mesmas, para não repetir um aviso no dia da troca.

### 3.4 Painel do Calendário

Com o ciclo ligado, uma seção **Ciclo** abre o painel, antes de hoje e amanhã:

1. **A fase em uma linha.** Os textos são os de hoje, mais "Atrasou {n} dia(s)".
2. **Hoje:** os avisos e os lembretes do dia, quando houver.
3. **Coincidem:** uma linha por evento do horizonte que cai em dia previsto.
   - **A linha mostra:** a data curta, o ponto da cor da agenda, o título e a fase ("TPM", "menstruação · 2º dia" ou "pode ser menstruação" no atraso).
   - **Clique:** abre os detalhes, como os outros eventos.
   - **Sem nenhum evento,** a seção diz "Nada marcado nesses dias."
4. **Próxima prevista:** a data. Embaixo, numa nota: "Com base nos últimos N ciclos" ou o texto dos padrões.

As médias de ciclo e de duração saem do painel. O rodapé "Lido dos eventos 🌸, sem guardar" vai para o fim da seção.

### 3.5 Cartão e tile

- **Cartão** (`windows/agenda.rs`):
  - um ponto na cor do Ciclo (`Glaze::CYCLE`) à direita da hora de cada evento que cai em dia previsto, sem texto;
  - vale para o período que o cartão já mostra;
  - a árvore de UI Automation não ganha nome novo para o ponto.
- **Tile:**
  - o selo e o balanço às 9h nos dias de aviso, como o Ciclo fazia (F2b-3). O selo some ao abrir o painel;
  - **durante toda a TPM e toda a menstruação** (inclusive a possível, no atraso), um ponto fixo na cor do Ciclo, sem palavra, num canto do tile. Abrir o painel não o apaga: ele some quando a fase acaba. O selo, quando houver, fica por cima dele;
  - a regra de quando o ponto aparece vem de `ahead`: hoje é um dia previsto.
- **O que sai:** o anel e o número de dias.

### 3.6 O que sai do app

- **No Rust:**
  - `WidgetKind::Cycle` deixa de ser criável: sai do catálogo e da lista de novos;
  - saem `faces::cycle` e os textos do tile.
- **No front:**
  - `CyclePanel`, `CycleTile` e o registro;
  - `CycleSettings` vira a seção dos ajustes do Calendário.
- **O que fica:** o tipo continua lido do banco só até a migração do §3.7. As fixtures de contrato mudam junto.

### 3.7 Migração

Roda uma vez, com o backup que o banco já faz antes de migrar. Para cada instância de Ciclo:

1. **Na mesma pilha de um Calendário** (spec 0023): o `CycleConfig` vai para esse Calendário, e o Ciclo sai da pilha. Uma pilha que fica com um só widget vira o widget solto, como ao desempilhar.
2. **Senão, com um Calendário no mesmo espaço de trabalho e no mesmo dock:** vai para esse Calendário (o primeiro, na ordem do dock).
3. **Senão:** o Ciclo **vira um Calendário no mesmo lugar**, com o ciclo ligado e a largura padrão.
   - Isso respeita o motivo do widget separado: um Ciclo num espaço que não aparece em reunião não pula para um Calendário de outro espaço.
4. **Um Calendário que já recebeu um ciclo não recebe outro.** O segundo Ciclo segue o passo 3, e o caso fica registrado no log.
5. **Um Ciclo que está em mais de um espaço** (a mesma instância, spec 0023) também segue o passo 3. Assim ele não aparece por um Calendário que só um dos espaços tinha.

O caminho fica para o recorte 2 decidir:
- uma migração SQL (a 7ª, com as funções JSON do SQLite);
- ou um passo em Rust logo depois das migrações, marcado em `meta`.

Os testes cobrem os cinco casos acima.

## 4. Recortes

Cada recorte é um PR com o teste escrito antes do código. A descrição do PR diz qual teste falhava antes. Os recortes de interface medem a memória do dock antes e depois.

1. **Domínio:** `ahead`, `late_by` e `overlaps` em `domain/cycle.rs`, com os testes puros. Nada muda na tela.
2. **Config e migração:** o `cycle` no `CalendarConfig`, a migração do §3.7, as fixtures e a seção Ciclo nos ajustes do Calendário. O widget Ciclo some do catálogo.
3. **Leitura e painel:** o horizonte em `windows/calendar.rs`, `notify` pelos Calendários e a seção Ciclo no `CalendarPanel`.
4. **Cartão, tile e limpeza:**
   - o ponto no cartão, o ponto fixo e o selo no tile do Calendário;
   - a remoção do código do widget Ciclo;
   - os docs: spec 0017 §6c (apontando para esta), o [guia do Atalho](../guia-atalho-ciclo.md), as [limitações](../limitacoes.md) e o [roadmap](../roadmap.md).

## 5. Verificação

- **Testes:** as regras do §3.2 com datas de mentira (inclusive os eventos de dia inteiro de vários dias, a virada de mês e o atraso), a migração nos cinco casos, o painel com e sem coincidências e o contrato das fixtures.
- **Ao vivo** (com o ok do autor na hora, seguindo os roteiros):
  1. o Calendário com o ciclo ligado, na cópia dos dados do autor;
  2. a seção Ciclo do painel;
  3. o ponto no cartão;
  4. o selo num dia de aviso, forçado pelo "Enviar aviso de teste";
  5. a migração do Ciclo que está no dock dele.
- **Memória:** o dock com o Calendário e o ciclo ligado, contra o Calendário e o Ciclo separados de hoje.

## 6. Calibração

A conferência da previsão com as marcas reais fica fora do repositório, porque os dados são dela. A spec não muda a regra: o autor ajusta "TPM: dias antes" quando as marcas mostrarem outra coisa.
