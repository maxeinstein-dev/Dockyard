# Spec 0025: Downloads, Lixeira e pastas no dock

F5 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md). Uma pasta fica no dock como um programa fixado e, ao clicar, mostra os itens mais recentes dela. Downloads e Lixeira entram prontos pelo menu do dock.

## 1. Como uma pasta entra no dock

- **Soltar a pasta no dock**, como já se solta um `.exe` ou um atalho (spec 0006 B2). Entra onde o ponteiro soltou.
- **Pelo menu do dock** (botão direito num espaço vazio ou num item):
  - "Adicionar Downloads" fixa a pasta Downloads do usuário (`FOLDERID_Downloads`);
  - "Adicionar Lixeira" fixa a Lixeira.
  - As duas entram logo depois do item onde o menu abriu e só aparecem enquanto o espaço mostrado não tem esse item.

O item é um programa fixado como os outros: mesma tabela, mesmos espaços, arrastar, remover e "Ícone" (spec 0024). O id dele diz o que ele é (`domain/folders.rs::place_of`):

- **Lixeira:** o id é `::{645FF040-5081-101B-9F08-00AA002F954E}`, o nome que o shell dá a ela.
- **Pasta:** o id é um caminho num disco (`C:\...`) que não termina em `.exe` nem em `.lnk`. Caminho de rede, caminho relativo e caminho com `..` ficam de fora.
- **Programa:** qualquer outro id, como era antes.

## 2. O clique

- **Pasta:** o clique abre o painel do item, sem abrir o Explorador.
  - O painel mostra "Mais recentes": os 24 itens alterados por último, entre arquivos e subpastas, numa grade de 4 colunas, cada um com um desenho pelo tipo e o nome.
  - Itens ocultos e de sistema ficam de fora, como o Explorador faz.
  - Clicar num item abre o arquivo com o programa do Windows, ou a subpasta no Explorador.
  - Depois da grade vem "Abrir no Explorador", seguido das ações de sempre ("Remover do dock", "Ícone").
- **Lixeira:** o clique abre a Lixeira no Explorador.
  - O menu dela tem "Esvaziar a Lixeira…". Quem confirma é o próprio Windows, com a pergunta e o número de itens. "Não" não esvazia nada.
- **Botão direito:** abre o mesmo painel nos três casos.

## 3. Leitura da pasta

- A pasta só é lida quando o painel abre, numa thread fora do dock. Só a lista dela é lida, sem descer às subpastas.
- No máximo 5.000 itens são olhados. Com mais que isso, o painel avisa que mostra os mais recentes desses.
- O painel nunca recebe um caminho, só uma chave por item, como nas Jump Lists (spec 0006 §6). Rust guarda o que cada chave abre até o próximo menu.
- Pasta que sumiu: o painel diz que não conseguiu ler e oferece remover.
- O tipo vem da extensão: pasta, imagem, vídeo, áudio, documento, compactado, programa ou outro. Nenhum ícone do shell é extraído para a grade, para que extensões do shell de terceiros não carreguem no processo do app.

## 4. A Lixeira no dock

- O ícone vem do Windows, que já desenha a Lixeira cheia ou vazia.
- O dock confere o estado (`SHQueryRecycleBin`, fora da thread do dock) a cada minuto enquanto mostra a Lixeira, e logo depois de "Esvaziar". O estado faz parte da chave do ícone, então o ícone muda quando a Lixeira enche ou esvazia.

## 5. Comandos

- `app_menu { id }` passa a responder também:
  - `place`: `program`, `folder` ou `recycleBin`;
  - `more`: a pasta tinha mais itens que os olhados.
  - Para uma pasta, a resposta tem uma seção `files`, e cada entrada traz o tipo (`kind`). O item é aberto pela chave, com `run_app_entry`.
- `empty_recycle_bin { id }`: só para o item que é a Lixeira. Devolve `false` quando a resposta foi não.

## 6. Verificação ao vivo: parcial

Feita em 2026-11-04 com o build de lançamento `.medir` e uma cópia dos dados do autor:

- "Adicionar Downloads", no menu do dock, pôs Downloads no dock.
- O clique em Downloads abriu o painel com "Mais recentes", e o item mais novo da pasta estava nele, junto com "Abrir no Explorador".
- "Adicionar Lixeira" pôs a Lixeira no dock.
- Com a Lixeira vazia antes, autorizado pelo autor: um arquivo vazio foi mandado para a Lixeira, e o ícone mudou para cheia em uns 20 s.
- "Esvaziar a Lixeira…" trouxe a pergunta do Windows ("Excluir Arquivo"). "Sim" esvaziou a Lixeira, e o ícone voltou para vazia.
- Depois disso, o menu do dock não oferecia mais Downloads nem Lixeira.
- O log não teve WARN nem ERROR, e a barra do Windows ficou como estava.

A pergunta do Windows é uma janela do próprio app, cujos botões a UI Automation não enxerga. O roteiro respondeu com Enter, que é o botão "Sim".

Ainda falta conferir à mão:

1. Soltar uma pasta no dock: ela entra onde foi solta (o soltar tem testes de unidade).
2. Clicar num arquivo do painel abre o arquivo, e "Abrir no Explorador" abre a pasta.
3. "Não" na pergunta de esvaziar não esvazia nada.
