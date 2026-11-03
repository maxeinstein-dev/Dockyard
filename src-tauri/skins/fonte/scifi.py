"""Gera a skin Espacial (spec 0020 §4): o casco de uma nave, em metal
azul-acinzentado com os quatro cantos chanfrados, um filete de luz em cima,
uma faixa de luz ciano perto da base, e nas pontas divisas que apontam para
fora e rebites.

Uso: python scifi.py

Desenhada em vetor (o rasterizador de icon-packs/fonte/vetor.py), em 64 px
de altura, e reduzida pelo dock com suavização (`pixelated: false`). O meio
estica: só o filete e a faixa passam por ele. Escreve `../scifi/skin.png` e
`../scifi/skin.json`.
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
CHANFRO = 8

CASCO_TOPO = (0x3A, 0x44, 0x58)
CASCO_BASE = (0x1C, 0x22, 0x30)
FILETE = (0x9F, 0xB2, 0xCC)
CIANO = (0x4F, 0xD8, 0xFF)
REBITE = (0x6E, 0x7C, 0x94)


def cor(rgb):
    return tuple(c / 255 for c in rgb)


def cobertura(desenhar):
    m = Mascara(ALTURA)
    desenhar(m)
    return m.cobertura()


def main():
    w, h, c = LARGURA, ALTURA, CHANFRO
    corpo = f"M{c} 0H{w - c}L{w} {c}V{h - c}L{w - c} {h}H{c}L0 {h - c}V{c}Z"
    filete = f"M{c + 1} 1.5H{w - c - 1}"
    faixa = f"M14 {h - 9}H{w - 14}"
    # As divisas: duas setas em cada ponta, apontando para fora.
    divisas = [
        "M12 26L7 32L12 38",
        "M17 26L12 32L17 38",
        f"M{w - 12} 26L{w - 7} 32L{w - 12} 38",
        f"M{w - 17} 26L{w - 12} 32L{w - 17} 38",
    ]
    rebites = []
    for x in (6, w - 6):
        for y in (12, h - 16):
            rebites.append(f"M{x - 1.6} {y}a1.6 1.6 0 1 1 3.2 0a1.6 1.6 0 1 1 -3.2 0")

    c_corpo = cobertura(lambda m: m.preencher(achatar(corpo)))
    c_borda = cobertura(lambda m: m.tracar(achatar(corpo), 1.4))
    c_filete = cobertura(lambda m: m.tracar(achatar(filete), 1.2))
    c_faixa = cobertura(lambda m: m.tracar(achatar(faixa), 2.0))
    c_brilho = cobertura(lambda m: m.tracar(achatar(faixa), 7.0))
    c_divisas = cobertura(lambda m: [m.tracar(achatar(d), 1.6) for d in divisas])
    c_rebites = cobertura(lambda m: [m.preencher(achatar(d)) for d in rebites])

    linhas = []
    for y in range(h):
        f = (y + 0.5) / h
        fundo = tuple(a + (b - a) * f for a, b in zip(cor(CASCO_TOPO), cor(CASCO_BASE)))
        linha = bytearray()
        for x in range(w):
            px = (0.0, 0.0, 0.0, 0.0)
            a = c_corpo[y][x]
            if a > 0:
                px = sobre(px, fundo, 0.97 * a)
                px = sobre(px, (0.07, 0.09, 0.13), 0.8 * c_borda[y][x] * a)
                px = sobre(px, cor(FILETE), 0.55 * c_filete[y][x])
                px = sobre(px, cor(CIANO), 0.25 * c_brilho[y][x] * a)
                px = sobre(px, cor(CIANO), 0.95 * c_faixa[y][x])
                px = sobre(px, cor(CIANO), 0.6 * c_divisas[y][x])
                px = sobre(px, cor(REBITE), 0.9 * c_rebites[y][x])
            r, g, b, al = px
            if al <= 0:
                linha += bytes(4)
            else:
                linha += bytes(min(255, round(v / al * 255)) for v in (r, g, b)) + bytes([min(255, round(al * 255))])
        linhas.append(linha)

    destino = AQUI.parent / "scifi"
    destino.mkdir(exist_ok=True)
    (destino / "skin.png").write_bytes(codificar(w, h, linhas))
    manifesto = {
        "name": "Espacial",
        "image": "skin.png",
        "width": w,
        "height": h,
        "slice": {"top": 0, "right": PONTA, "bottom": 0, "left": PONTA},
        "middle": "stretch",
        "pixelated": False,
    }
    texto = json.dumps(manifesto, ensure_ascii=False, indent=2) + "\n"
    (destino / "skin.json").write_text(texto, encoding="utf-8", newline="\n")
    print(f"scifi: {w}×{h}")


if __name__ == "__main__":
    main()
