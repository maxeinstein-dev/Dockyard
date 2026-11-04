"""Desenha os 12 launchers de jogador (spec 0019 §G4, spec 0020 Q4 e Q12) nos
quatro pacotes de pixel que nasceram antes deles: Retrô 8-bit, Game Boy,
Fósforo e Clássico 98 (estudo 0002, o último item adiado da spec 0020).

Uso: python launchers_pixel.py   (uma vez; depois, edite o texto à mão)

Cada glifo de glifos.txt é traçado na grade do pacote e vira um desenho no
estilo dele, acrescentado ao fim de `<pacote>.txt`; `gerar.py <pacote>` faz
os PNGs. Um desenho que já está no arquivo não é escrito de novo, então rodar
outra vez não desfaz uma edição à mão.

- Retrô 8-bit e Game Boy (16×16): placa arredondada com contorno, na cor da
  paleta mais perto da matiz do app, e o glifo claro dentro.
- Fósforo (32×32): o monitor com bisel e linhas de varredura dos outros, o
  glifo aceso (4) e o halo (3 ao lado, 2 na diagonal).
- Clássico 98 (32×32): o botão com o bisel do 98 e o glifo numa das 16
  cores, com sombra de 1 px.
"""

from pathlib import Path

from vetor import Mascara, achatar, ler_glifos, mover

AQUI = Path(__file__).resolve().parent

LAUNCHERS = [
    "epic",
    "battlenet",
    "riot",
    "ea",
    "ubisoft",
    "gog",
    "faceit",
    "nvidia",
    "amd",
    "overwolf",
    "twitch",
    "medal",
]


def pixels(glifo, lado, largura, limite=0.4):
    """O glifo traçado numa grade de `lado` px: True onde o traço cobre.
    `largura` é a do traço, nas unidades do glifo (24 de lado)."""
    m = Mascara(lado)
    escala = lado / 24
    for d in glifo["caminhos"]:
        m.tracar(mover(achatar(d), escala, 0, 0), largura * escala)
    return [[c >= limite for c in linha] for linha in m.cobertura()]


def faixa(glifo, faixas):
    """A letra da faixa de matiz do app; `None` em `faixas` é a dos apps de
    pouca saturação."""
    h, s, _ = glifo["esmalte"]
    if s < 20:
        return faixas[None]
    h %= 360
    return next(letra for limite, letra in faixas["matiz"] if h < limite)


# --- Retrô 8-bit e Game Boy (16×16) ----------------------------------------

PLACA = [
    "..000000000000..",
    ".0............0.",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    "0..............0",
    ".0SSSSSSSSSSSSS0.",
    "..000000000000..",
]
# A linha 14 tem a sombra da base no lugar dos pontos.
PLACA[14] = ".0SSSSSSSSSSSS0."

RETRO = {
    None: "5",
    "matiz": [(15, "8"), (45, "9"), (70, "a"), (160, "3"), (200, "c"), (250, "c"), (290, "d"), (345, "e"), (361, "8")],
}
# Sombra da base e cor do glifo por cor da placa.
RETRO_SOMBRA = {"5": "0", "8": "2", "9": "4", "a": "9", "3": "1", "c": "1", "d": "2", "e": "2"}
RETRO_GLIFO = {"a": "0"}

GAME_BOY = {
    None: "1",
    "matiz": [(15, "1"), (45, "2"), (70, "2"), (160, "2"), (200, "1"), (250, "1"), (290, "1"), (345, "2"), (361, "1")],
}


def placa(glifo, cor, sombra, tinta):
    linhas = [list(linha.replace(".", "F", 0)) for linha in PLACA]
    for y, linha in enumerate(PLACA):
        for x, ch in enumerate(linha):
            if ch == "." and 0 < y < 14 and 0 < x < 15 and not (y == 1 and x in (1, 14)):
                linhas[y][x] = cor
            elif ch == "S":
                linhas[y][x] = sombra
    # O glifo em 14 px a partir de 1: o desenho (de 3 a 21 das 24
    # unidades) fica dentro da placa.
    traco = pixels(glifo, 14, 1.6, 0.5)
    for gy in range(14):
        for gx in range(14):
            if traco[gy][gx] and linhas[gy + 1][gx + 1] == cor:
                linhas[gy + 1][gx + 1] = tinta
    return ["".join(linha) for linha in linhas]


def retro(glifo):
    cor = faixa(glifo, RETRO)
    return placa(glifo, cor, RETRO_SOMBRA[cor], RETRO_GLIFO.get(cor, "7"))


def game_boy(glifo):
    cor = faixa(glifo, GAME_BOY)
    return placa(glifo, cor, "0" if cor == "1" else "1", "3")


# --- Fósforo e Clássico 98 (32×32) -----------------------------------------


def moldura(texto, id_modelo):
    """As 32 linhas do desenho `id_modelo` do pacote: a moldura que cada
    pacote põe atrás do glifo."""
    linhas, dentro = [], False
    for linha in texto.splitlines():
        if linha.startswith("@"):
            dentro = linha[1:].strip() == id_modelo
            continue
        if dentro and len(linha) == 32 and not linha.startswith("nomes:"):
            linhas.append(list(linha))
        if len(linhas) == 32:
            break
    return linhas


def fosforo(glifo, base):
    linhas = [list(linha) for linha in base]
    for y in range(3, 29):
        for x in range(3, 29):
            linhas[y][x] = "0" if y % 2 else "1"
    # O glifo em 24 px a partir de 4, do tamanho dos outros desenhos.
    traco = pixels(glifo, 24, 1.8)
    aceso = {(gx + 4, gy + 4) for gy in range(24) for gx in range(24) if traco[gy][gx]}
    for x, y in aceso:
        linhas[y][x] = "4"
    for y in range(4, 28):
        for x in range(4, 28):
            if (x, y) in aceso:
                continue
            lado = any((x + dx, y + dy) in aceso for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
            diagonal = any((x + dx, y + dy) in aceso for dx, dy in ((1, 1), (-1, 1), (1, -1), (-1, -1)))
            if lado:
                linhas[y][x] = "3"
            elif diagonal:
                linhas[y][x] = "2"
    return ["".join(linha) for linha in linhas]


CLASSICO = {
    None: "0",
    "matiz": [(15, "9"), (45, "1"), (70, "3"), (160, "2"), (200, "6"), (250, "4"), (290, "5"), (345, "d"), (361, "9")],
}


def classico(glifo, base):
    linhas = [list(linha) for linha in base]
    for y in range(2, 30):
        for x in range(2, 30):
            linhas[y][x] = "7"
    cor = faixa(glifo, CLASSICO)
    traco = pixels(glifo, 24, 2.0)
    tinta = {(gx + 4, gy + 4) for gy in range(24) for gx in range(24) if traco[gy][gx]}
    for x, y in tinta:
        if (x + 1, y + 1) not in tinta:
            linhas[y + 1][x + 1] = "8"
    for x, y in tinta:
        linhas[y][x] = cor
    return ["".join(linha) for linha in linhas]


def acrescentar(pacote, desenhar):
    caminho = AQUI / f"{pacote}.txt"
    texto = caminho.read_text(encoding="utf-8")
    presentes = {linha[1:].strip() for linha in texto.splitlines() if linha.startswith("@")}
    glifos = {g["id"]: g for g in ler_glifos(AQUI / "glifos.txt")}
    novos = []
    for nome in LAUNCHERS:
        if nome in presentes:
            continue
        glifo = glifos[nome]
        novos += ["", f"@{nome}", "nomes: " + " | ".join(glifo["nomes"])]
        novos += desenhar(glifo, texto)
    if novos:
        caminho.write_text(texto.rstrip("\n") + "\n" + "\n".join(novos) + "\n", encoding="utf-8", newline="\n")
    print(f"{pacote}: {len(novos) // 35 if pacote in ('fosforo', 'classico-98') else len(novos) // 19} novos")


def main():
    acrescentar("retro-8bit", lambda g, _: retro(g))
    acrescentar("game-boy", lambda g, _: game_boy(g))
    acrescentar("fosforo", lambda g, texto: fosforo(g, moldura(texto, "steam")))
    acrescentar("classico-98", lambda g, texto: classico(g, moldura(texto, "steam")))


if __name__ == "__main__":
    main()
