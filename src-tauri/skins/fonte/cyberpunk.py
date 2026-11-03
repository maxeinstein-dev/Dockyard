"""Gera a skin Cyberpunk (spec 0020 §4): uma barra de asfalto quase preto
com dois cantos cortados em diagonal, um fio ciano em cima e um magenta
embaixo, com brilho, e um "glitch" nas pontas: os fios deslocados, como uma
imagem que falha.

Uso: python cyberpunk.py

Desenhada em vetor (o rasterizador de icon-packs/fonte/vetor.py), em 64 px
de altura, e reduzida pelo dock com suavização (`pixelated: false`). O meio
estica: só os fios passam por ele. Escreve `../cyberpunk/skin.png` e
`../cyberpunk/skin.json`.
"""

import json
import sys
from pathlib import Path

AQUI = Path(__file__).resolve().parent
sys.path.insert(0, str(AQUI.parent.parent / "icon-packs" / "fonte"))
from gerar import codificar  # noqa: E402
from vetor import Mascara, achatar, sobre  # noqa: E402

ALTURA = 64
PONTA = 26  # cada ponta, em pixels
MEIO = 4
LARGURA = PONTA + MEIO + PONTA
CORTE = 14  # os cantos cortados: em cima à esquerda e embaixo à direita

ASFALTO_TOPO = (0x1A, 0x16, 0x24)
ASFALTO_BASE = (0x0B, 0x0A, 0x10)
CIANO = (0x2E, 0xF2, 0xFF)
MAGENTA = (0xFF, 0x2B, 0xD6)


def cor(rgb):
    return tuple(c / 255 for c in rgb)


def cobertura(desenhar):
    m = Mascara(ALTURA)
    desenhar(m)
    return m.cobertura()


def main():
    w, h, c = LARGURA, ALTURA, CORTE
    corpo = f"M{c} 0H{w}V{h - c}L{w - c} {h}H0V{c}Z"
    # Os fios: o ciano a 6 px do topo, o magenta a 6 px da base, cada um
    # parando antes do corte do seu lado.
    ciano = f"M{c + 3} 6H{w - 4}"
    magenta = f"M4 {h - 6}H{w - c - 3}"
    # O "glitch": um pedaço de cada fio repetido fora do lugar, nas pontas.
    falhas_ciano = [f"M{w - 20} 10.5H{w - 9}", f"M{c + 6} 14H{c + 11}"]
    falhas_magenta = [f"M9 {h - 10.5}H20", f"M{w - c - 11} {h - 14}H{w - c - 6}"]
    # A borda: um fio fino, mais claro que o asfalto.
    borda = (
        f"M{c + 0.4} 0.75H{w - 0.75}V{h - c - 0.4}L{w - c - 0.4} {h - 0.75}"
        f"H0.75V{c + 0.4}Z"
    )

    c_corpo = cobertura(lambda m: m.preencher(achatar(corpo)))
    c_borda = cobertura(lambda m: m.tracar(achatar(borda), 1.2))
    c_ciano = cobertura(lambda m: m.tracar(achatar(ciano), 1.6))
    c_ciano_brilho = cobertura(lambda m: m.tracar(achatar(ciano), 5.0))
    c_magenta = cobertura(lambda m: m.tracar(achatar(magenta), 1.6))
    c_magenta_brilho = cobertura(lambda m: m.tracar(achatar(magenta), 5.0))
    c_falhas_c = cobertura(lambda m: [m.tracar(achatar(d), 1.4) for d in falhas_ciano])
    c_falhas_m = cobertura(lambda m: [m.tracar(achatar(d), 1.4) for d in falhas_magenta])

    linhas = []
    for y in range(h):
        f = (y + 0.5) / h
        fundo = tuple(a + (b - a) * f for a, b in zip(cor(ASFALTO_TOPO), cor(ASFALTO_BASE)))
        linha = bytearray()
        for x in range(w):
            px = (0.0, 0.0, 0.0, 0.0)
            a = c_corpo[y][x]
            if a > 0:
                px = sobre(px, fundo, 0.97 * a)
                px = sobre(px, (0.32, 0.28, 0.42), 0.8 * c_borda[y][x] * a)
                px = sobre(px, cor(CIANO), 0.22 * c_ciano_brilho[y][x] * a)
                px = sobre(px, cor(MAGENTA), 0.22 * c_magenta_brilho[y][x] * a)
                px = sobre(px, cor(CIANO), 0.95 * c_ciano[y][x])
                px = sobre(px, cor(MAGENTA), 0.95 * c_magenta[y][x])
                px = sobre(px, cor(CIANO), 0.7 * c_falhas_c[y][x])
                px = sobre(px, cor(MAGENTA), 0.7 * c_falhas_m[y][x])
            r, g, b, al = px
            if al <= 0:
                linha += bytes(4)
            else:
                linha += bytes(min(255, round(v / al * 255)) for v in (r, g, b)) + bytes([min(255, round(al * 255))])
        linhas.append(linha)

    destino = AQUI.parent / "cyberpunk"
    destino.mkdir(exist_ok=True)
    (destino / "skin.png").write_bytes(codificar(w, h, linhas))
    manifesto = {
        "name": "Cyberpunk",
        "image": "skin.png",
        "width": w,
        "height": h,
        "slice": {"top": 0, "right": PONTA, "bottom": 0, "left": PONTA},
        "middle": "stretch",
        "pixelated": False,
    }
    texto = json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n"
    (destino / "skin.json").write_text(texto, encoding="utf-8", newline="\n")
    print(f"cyberpunk: {w}×{h}")


if __name__ == "__main__":
    main()
