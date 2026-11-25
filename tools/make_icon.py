"""Draws the Shiftella app icon: a warm coffee cup under a crescent moon.

Usage:
    python3 -I tools/make_icon.py <Inter-Bold.ttf> <out.png> [--rgba <dir>]

`out.png` is a 1024x1024 macOS-style icon (used for the .icns in the app bundle).
With `--rgba <dir>` it also writes `icon_16.rgba`, `icon_32.rgba` and `icon_64.rgba`
(raw RGBA, row-major) — the window/Dock icon compiled into the game binary, so
macroquad does not show its default icon while the game runs.
"""

import math
import os
import sys

from PIL import Image, ImageDraw, ImageFilter

SIZE = 1024
MARGIN = 100  # macOS icon grid padding
RADIUS = 185
INNER = SIZE - 2 * MARGIN

INDIGO = (0x63, 0x66, 0xF1)  # Shiftella indigo 500
NIGHT = (0x2A, 0x10, 0x5C)  # deep violet night
AMBER = (255, 196, 107)
CREAM = (255, 236, 200)


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def soft(layer_size, draw_fn, blur):
    """Draws into a transparent layer and blurs it (for glows and steam)."""
    layer = Image.new("RGBA", layer_size, (0, 0, 0, 0))
    draw_fn(ImageDraw.Draw(layer))
    return layer.filter(ImageFilter.GaussianBlur(blur))


def tile_image() -> Image.Image:
    grad = Image.new("RGB", (INNER, INNER))
    gd = ImageDraw.Draw(grad)
    for y in range(INNER):
        gd.line([(0, y), (INNER, y)], fill=lerp(INDIGO, NIGHT, y / INNER))
    tile = grad.convert("RGBA")
    mask = Image.new("L", (INNER, INNER), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, INNER - 1, INNER - 1], RADIUS, fill=255)

    def clip(layer):
        return Image.composite(layer, Image.new("RGBA", layer.size), mask)

    cx = INNER / 2
    # warm lamp glow behind the cup
    tile.alpha_composite(clip(soft(tile.size, lambda d: d.ellipse(
        [INNER * 0.18, INNER * 0.36, INNER * 0.82, INNER * 0.98], fill=AMBER + (115,)), 70)))

    # steam: two soft curls rising from the cup
    def steam(d):
        for i, x0 in enumerate([cx - 62, cx + 18]):
            pts = [(x0 + 26 * math.sin(t / 8.5 + i * 1.7), 392 - t * 2.3) for t in range(0, 66)]
            d.line(pts, fill=(255, 255, 255, 190), width=24, joint="curve")
    tile.alpha_composite(clip(soft(tile.size, steam, 5)))

    d = ImageDraw.Draw(tile)
    # saucer
    d.ellipse([cx - 232, 636, cx + 232, 702], fill=(226, 220, 244, 255))
    d.ellipse([cx - 200, 640, cx + 200, 680], fill=(240, 236, 250, 255))
    # cup body
    d.rounded_rectangle([cx - 170, 430, cx + 170, 662], 72, fill=(255, 255, 255, 255))
    d.rectangle([cx - 170, 430, cx + 170, 520], fill=(255, 255, 255, 255))
    # handle
    d.ellipse([cx + 128, 470, cx + 262, 604], outline=(255, 255, 255, 255), width=34)
    # coffee
    d.ellipse([cx - 160, 410, cx + 160, 462], fill=(120, 70, 40, 255))
    d.ellipse([cx - 148, 413, cx + 148, 452], fill=(176, 110, 64, 255))
    # a tiny indigo band on the cup (Shiftella touch)
    d.rectangle([cx - 170, 560, cx + 170, 582], fill=INDIGO + (255,))

    # crescent moon (mask: circle minus offset circle, no painted patch)
    mx, my, mr = INNER * 0.68, INNER * 0.19, INNER * 0.1
    moon_mask = Image.new("L", tile.size, 0)
    md = ImageDraw.Draw(moon_mask)
    md.ellipse([mx - mr, my - mr, mx + mr, my + mr], fill=255)
    md.ellipse([mx - mr * 0.45, my - mr * 1.15, mx + mr * 1.5, my + mr * 0.75], fill=0)
    moon = Image.new("RGBA", tile.size, CREAM + (255,))
    moon.putalpha(moon_mask)
    tile.alpha_composite(moon)

    # lit top edge, like Shiftella's glass panels
    edge = Image.new("RGBA", tile.size, (0, 0, 0, 0))
    ImageDraw.Draw(edge).rounded_rectangle(
        [2, 2, INNER - 3, INNER - 3], RADIUS - 2, outline=(255, 255, 255, 70), width=4)
    fade = Image.linear_gradient("L").resize(tile.size).point(lambda v: 255 - v)
    edge.putalpha(Image.composite(edge.getchannel("A"), Image.new("L", edge.size, 0), fade))
    tile.alpha_composite(edge)

    tile.putalpha(Image.composite(tile.getchannel("A"), Image.new("L", tile.size, 0), mask))
    return tile


def icon_image() -> Image.Image:
    icon = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    shadow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        [MARGIN, MARGIN + 18, SIZE - MARGIN, SIZE - MARGIN + 18], RADIUS, fill=(10, 6, 30, 140))
    icon.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(22)))
    icon.alpha_composite(tile_image(), (MARGIN, MARGIN))
    return icon


def main(argv) -> None:
    if len(argv) not in (3, 5) or (len(argv) == 5 and argv[3] != "--rgba"):
        sys.exit("usage: make_icon.py <Inter-Bold.ttf> <out.png> [--rgba <dir>]")
    icon = icon_image()
    icon.save(argv[2])
    if len(argv) == 5:
        out_dir = argv[4]
        os.makedirs(out_dir, exist_ok=True)
        # small sizes: use the tile without the outer padding so it stays legible
        tile = tile_image()
        for s in (16, 32, 64):
            im = tile.resize((s, s), Image.LANCZOS)
            with open(os.path.join(out_dir, f"icon_{s}.rgba"), "wb") as f:
                f.write(im.tobytes())


if __name__ == "__main__":
    main(sys.argv)
