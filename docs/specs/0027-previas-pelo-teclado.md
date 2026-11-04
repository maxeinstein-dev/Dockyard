# Spec 0027: Prévias pelo teclado e o Espiar

F10 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md). A [spec 0015](0015-fase-e-previas.md) §5 deixou de fora duas coisas:
- **o cartão pelo teclado**, porque na época o dock nunca tomava o foco;
- **o Espiar**, porque `DwmpActivateLivePreview` não é documentada.

A M14 deu ao dock o foco pelo atalho, e isso destrava a primeira.

## 1. O cartão pelo teclado

Com o dock pelo atalho (M14) e um programa escolhido pelas setas, a **seta na direção do cartão** abre as prévias das janelas dele. Num dock embaixo é a seta para cima; num dock em cima, para baixo. O cartão abre mesmo com uma janela só, porque foi pedido. Programa sem janela: a seta faz o que fazia antes.

No cartão aberto assim, a primeira miniatura vem escolhida. O teclado continua no dock, já que o cartão não toma o foco, e o dock repassa as teclas a ele (`domain/previews.rs::card_key`):

| Tecla | O que faz |
|---|---|
| ← e → | andam pelas miniaturas, sem dar a volta nas pontas |
| Home e End | vão para a primeira e para a última |
| Enter e Espaço | trazem a janela escolhida para a frente e fecham o cartão |
| Delete | pede para a janela fechar (o programa pode perguntar algo). O cartão continua com as outras, ou sai se não sobrar nenhuma |
| Esc e a seta de volta (para baixo num dock embaixo) | fecham o cartão; o programa continua escolhido no dock |

Mais regras do cartão aberto pelo teclado:
- **Destaque:** a miniatura escolhida tem o fundo de realce e um anel na cor do foco do Windows (`COLOR_HIGHLIGHT`, que segue o alto contraste).
- **Leitor de tela:** ele acompanha a miniatura pelo nome da janela.
- **Ponteiro:** o cartão não fecha por o ponteiro estar longe.

## 2. O Espiar: spike primeiro

O Espiar é o que a barra do Windows faz ao passar o mouse numa miniatura: as outras janelas ficam transparentes e só aquela aparece. O único caminho é `DwmpActivateLivePreview`, o ordinal 113 do `dwmapi.dll`. A assinatura conhecida pela comunidade é `(BOOL on, HWND peek, HWND topmost, UINT type, UINT_PTR)`.

Antes de prometer, um spike:
- duas janelas do próprio roteiro no mesmo lugar, a de cima vermelha e a de baixo verde;
- pede o Espiar da verde e lê o pixel do meio, com os tipos 1 e 3;
- depois desliga e confere que o vermelho voltou.

O resultado fica escrito aqui. O Espiar só entra no cartão se o spike mostrar que funciona e desliga limpo. Mesmo assim, entraria atrás de uma checagem que o deixa de fora se a função não existir.

**Resultado do spike (2026-11-04, Windows 11 build 26300):**
- **Funcionou.** Com os tipos 1 e 3, ligar devolveu `S_OK` e só a janela verde ficou à vista. Desligar devolveu `S_OK` e o vermelho voltou.

**Como entrou no cartão:**
- **Quando liga:** depois de 500 ms com o ponteiro numa miniatura, ou com ela escolhida pelo teclado, chama `DwmpActivateLivePreview(1, janela, cartão, 1, 0)`. O cartão fica por cima do que se vê.
- **Quando desliga:** quando a miniatura muda, quando o cartão sai (inclusive ao trazer uma janela para a frente) e quando o app sai (`release_on_exit`).
- **Sem a função:** onde o ordinal não existe, o cartão fica sem o Espiar.

## 3. Verificação

- **Testes de unidade (`domain/previews.rs`):**
  - as setas andam sem dar a volta;
  - Enter abre, Delete fecha, Esc e a seta de volta saem;
  - a seta de ida segue a borda do dock.
- **Ao vivo (pendente):** o Espiar também é conferido: com uma janela do roteiro por cima, a escolhida aparece. com uma janela do próprio roteiro, o PowerShell dele, o roteiro confere:
  1. o atalho e as setas chegam ao item;
  2. a seta para cima abre o cartão, com o leitor de tela na miniatura;
  3. o cartão fica aberto com o ponteiro longe;
  4. Esc fecha e o item segue escolhido;
  5. Enter traz a janela para a frente;
  6. Delete a fecha, e o cartão sai junto.
