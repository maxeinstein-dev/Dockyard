# Spec 0024 — Ícone por programa

Item F4 do [estudo 0002](../estudos/0002-melhorias-e-funcionalidades.md): o recorte B4b da [spec 0007](0007-fase-b4-icones.md), nunca começado. Os pacotes não cobrem todo programa (os jogos ficam na reserva, Q4 da 0020). Esta escolha deixa o usuário fechar o buraco sem esperar um pacote novo.

## 1. O que se escolhe

No menu de um programa fixado, uma seção **"Ícone"** (en: "Icon"), com uma opção marcada:

- **"O do dock"**: segue o pacote do dock, como antes. É o padrão, e sai da lista de escolhas.
- **"Original do Windows"**: o ícone que o Windows dá, qualquer que seja o pacote.
- **Um pacote**, um item por pacote válido da pasta `IconPacks`, pelo nome: a imagem que ele tem para o programa. Usa a imagem escura com o dock escuro, e a de reserva do pacote se ele não cobre o programa. Sem nenhuma, fica o do Windows.
- **"Escolher imagem…"**: abre o diálogo "Abrir" do Windows, para PNG ou ICO.

As setas do teclado passam pelas opções, como pelos outros itens do menu.

## 2. A imagem do usuário

- O arquivo escolhido é lido **uma vez**, na hora, fora da janela do dock, com até 2 MB.
- Um PNG fica como está. De um ICO vale a maior imagem: uma PNG guardada dentro dele, ou um bitmap de 32 bits convertido em PNG. Um ICO antigo, sem alfa, usa a máscara dele para os cantos transparentes.
- O tamanho tem de estar entre 16 e 1024 px de lado, como nos pacotes. Imagem recusada (grande demais, sem ser imagem, tamanho fora) só vai para o log: o menu já fechou quando o diálogo abre.
- Fica guardada como PNG em `<pasta de dados>\AppIcons\<uuid>.png`, com um nome dado pelo Dockyard. O dock nunca lê o arquivo do usuário de novo. A imagem que nenhum programa usa mais é apagada.

## 3. Onde fica a escolha

- Nas configurações: `dock.appIcons`, um mapa da chave do programa para a escolha. A chave é o id dele na pasta Apps, em minúsculas. A escolha é `{ "kind": "original" }`, `{ "kind": "pack", "pack": "<id>" }` ou `{ "kind": "image", "file": "<uuid>.png" }`. O campo fica fora do arquivo enquanto vazio.
- Sem migração do banco: a escolha é aparência, como o pacote do dock.
- A validação recusa mais de 500 escolhas, uma chave que não é id de programa ou não está em minúsculas, um pacote com id inválido e um arquivo que não tem o formato de nome do Dockyard.
- Configurações não mexe no campo, mas o leva de volta ao salvar.

## 4. No dock

- A escolha vence o pacote do dock (`apps::dock_icon_png`).
- O dock guarda a imagem pela origem (`icon_key`): a escolha, o pacote ou o Windows. Trocar a escolha troca a chave, e a imagem é lida de novo na hora. A imagem escura só vale para um pacote.
- O ícone dos painéis (a lista de Apps do dock) continua o do Windows e do pacote, como antes: a escolha é do dock.

## 5. Comandos

Na janela dos painéis:

- `app_icon_options { id }`: a escolha do programa fixado e os pacotes.
- `set_app_icon { id, icon }`: Original, um pacote ou `null`. Imagem só vem do comando abaixo.
- `choose_app_icon_image { id }`: abre o diálogo e guarda a imagem. Devolve `false` se o diálogo foi cancelado ou a imagem recusada.

## 6. Verificação ao vivo: parcial

Feita em 2026-11-04 (build de lançamento `.medir`, cópia dos dados do autor), com o FileZilla:

- A linha "Ícone: O do dock" aparece no menu e abre a lista.
- "Escolher imagem…" abre o diálogo "Escolher o ícone de FileZilla". Um PNG magenta de 128 px foi aceito.
- O pixel do FileZilla no dock ficou magenta, e a imagem foi guardada em `AppIcons`.
- A linha passou a dizer "Ícone: Imagem própria".
- "O do dock" devolveu o ícone de antes e apagou a imagem de `AppIcons`.
- O log tem `icons: choosing an image for a program` seguido de `icons: image chosen: true`. Não houve WARN nem ERROR.
- A barra do Windows ficou como estava.

Ainda falta conferir à mão:

1. Escolher um pacote: o ícone do programa muda e os dos outros não.
2. "Original do Windows" com um pacote no dock: só esse programa volta ao ícone do Windows.
3. Trocar o pacote do dock em Configurações não mexe na imagem própria.
4. Escolher um ICO.
