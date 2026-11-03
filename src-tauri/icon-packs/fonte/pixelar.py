"""Faz os desenhos do pacote Fliperama (spec 0020 §4) a partir dos glifos de
glifos.txt: cada glifo vira um sprite de 32×32 numa tela de fliperama com
linhas de varredura, e o resultado é o texto de `fliperama.txt`, que se edita
à mão depois e que `gerar.py fliperama` transforma nos PNGs.

Uso: python pixelar.py > fliperama.txt   (uma vez; depois, edite o texto)

O sprite: o glifo traçado grosso numa imagem de 128 px e reduzido a 32 pela
cobertura, na cor de fliperama mais perto da matiz do app, com 1 px de
sombra embaixo e à direita.
"""

from pathlib import Path

from vetor import Mascara, achatar, ler_glifos, mover

AQUI = Path(__file__).resolve().parent
GRADE = 32
FINO = 128
K = FINO // GRADE

PALETA = {
    "0": "#08080C",  # contorno do gabinete
    "B": "#2A2D45",  # moldura da tela
    "n": "#0D0F20",  # tela
    "m": "#141838",  # linha de varredura
    "s": "#030306",  # sombra do sprite
    "r": "#FF3B3B",
    "o": "#FF8A1F",
    "y": "#FFD23F",
    "g": "#3BE36B",
    "c": "#2EF2FF",
    "b": "#4F7DFF",
    "p": "#A45CFF",
    "P": "#FF4FB8",
    "w": "#F2F2F7",
}

# A cor do sprite pela matiz do app (a do Esmalte, em graus).
FAIXAS = [(15, "r"), (45, "o"), (70, "y"), (160, "g"), (200, "c"), (250, "b"), (290, "p"), (345, "P"), (361, "r")]

JOYSTICK = {
    "id": "joystick",
    "nomes": ["reserva"],
    "esmalte": None,
    "caminhos": [
        "M6 16.5h12a1.5 1.5 0 0 1 1.5 1.5v1.5h-15v-1.5a1.5 1.5 0 0 1 1.5-1.5z",
        "M12 16.5v-7",
        "M12 4.5a2.5 2.5 0 1 1 0 5a2.5 2.5 0 1 1 0-5",
    ],
}


def cor_do(glifo):
    if glifo["esmalte"] is None:
        return "y"
    h = glifo["esmalte"][0] % 360
    return next(letra for limite, letra in FAIXAS if h < limite)


def tela():
    """O gabinete: contorno, moldura e a tela com linhas de varredura."""
    linhas = []
    for y in range(GRADE):
        if y in (0, GRADE - 1):
            linhas.append(list("." + "0" * (GRADE - 2) + "."))
        elif y in (1, GRADE - 2):
            linhas.append(list("0" + "B" * (GRADE - 2) + "0"))
        else:
            meio = "m" if y % 2 else "n"
            linhas.append(list("0B" + meio * (GRADE - 4) + "B0"))
    return linhas


def sprite(glifo):
    """Os pixels do glifo na grade de 32: True onde o traço cobre."""
    m = Mascara(FINO)
    lado = FINO * 0.6
    x0 = (FINO - lado) / 2
    escala = lado / 24
    for d in glifo["caminhos"]:
        m.tracar(mover(achatar(d), escala, x0, x0), 2.6 * escala)
    fino = m.cobertura()
    pixels = []
    for gy in range(GRADE):
        linha = []
        for gx in range(GRADE):
            soma = sum(fino[gy * K + dy][gx * K + dx] for dy in range(K) for dx in range(K))
            linha.append(soma / (K * K) >= 0.4)
        pixels.append(linha)
    return pixels


def desenho(glifo):
    linhas = tela()
    pixels = sprite(glifo)
    cor = cor_do(glifo)
    dentro = range(2, GRADE - 2)
    for y in dentro:
        for x in dentro:
            if pixels[y][x]:
                linhas[y][x] = cor
            elif pixels[y - 1][x - 1] and linhas[y][x] in "nm":
                linhas[y][x] = "s"
    return ["".join(linha) for linha in linhas]


def main():
    # A reserva de glifos.txt dá lugar ao joystick.
    glifos = [g for g in ler_glifos(AQUI / "glifos.txt") if "reserva" not in g["nomes"]]
    glifos.append(JOYSTICK)
    saida = [
        "# Pacote Fliperama (spec 0020 §4): os desenhos que viram os PNGs.",
        "# Para gerar de novo: python gerar.py fliperama",
        "# Feitos uma vez por pixelar.py a partir de glifos.txt, e editáveis à",
        "# mão: cada app é um sprite na cor de fliperama mais perto da matiz dele,",
        "# com sombra de 1 px (s), numa tela com linhas de varredura (n, m) e",
        "# moldura (B). A reserva é um joystick.",
        "",
        "pack.name: Fliperama",
        "pack.author: Dockyard",
        "pack.version: 1.0.0",
        "pack.description: Pixel art 32×32: cada app como um sprite numa tela de fliperama.",
        "pack.accent: #FFD23F",
        f"pack.grid: {GRADE}",
        "pack.scale: 4",
        "",
    ]
    saida += [f"cor.{letra}: {valor}" for letra, valor in PALETA.items()]
    for glifo in glifos:
        saida += ["", f"@{glifo['id']}", "nomes: " + " | ".join(glifo["nomes"])]
        saida += desenho(glifo)
    print("\n".join(saida))


if __name__ == "__main__":
    main()
