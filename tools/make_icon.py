"""Рисует иконку приложения Shiftella (1024×1024 PNG).

Запуск: python3 -I tools/make_icon.py <путь к Inter-Bold.ttf> <выходной .png>

Ночное окно кафе в цветах Shiftella: градиент индиго → фиолетовый,
белая «S», тёплая лампа и полумесяц.
"""

import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

SIZE = 1024
MARGIN = 100  # отступ, как у системных иконок macOS
RADIUS = 185


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def main(font_path: str, out_path: str) -> None:
    top = (0x63, 0x66, 0xF1)  # indigo 500
    bottom = (0x3B, 0x15, 0x76)  # violet 900
    inner = SIZE - 2 * MARGIN

    # Градиентный фон
    grad = Image.new("RGB", (inner, inner))
    gd = ImageDraw.Draw(grad)
    for y in range(inner):
        gd.line([(0, y), (inner, y)], fill=lerp(top, bottom, y / inner))

    mask = Image.new("L", (inner, inner), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, inner - 1, inner - 1], RADIUS, fill=255)

    icon = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))

    # Мягкая тень под плиткой
    shadow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        [MARGIN, MARGIN + 18, SIZE - MARGIN, SIZE - MARGIN + 18], RADIUS, fill=(10, 6, 30, 140)
    )
    icon.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(22)))

    tile = Image.new("RGBA", (inner, inner))
    tile.paste(grad, (0, 0))
    tile.putalpha(mask)

    d = ImageDraw.Draw(tile)
    # Тёплое пятно лампы
    glow = Image.new("RGBA", (inner, inner), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse([inner * 0.52, inner * 0.08, inner * 0.98, inner * 0.54], fill=(255, 196, 107, 120))
    glow = glow.filter(ImageFilter.GaussianBlur(60))
    tile.alpha_composite(Image.composite(glow, Image.new("RGBA", glow.size), mask))

    # Полумесяц
    cx, cy, r = inner * 0.75, inner * 0.24, inner * 0.085
    # маска «круг минус смещённый круг» — без закрашенной заплатки на фоне
    moon_mask = Image.new("L", (inner, inner), 0)
    md = ImageDraw.Draw(moon_mask)
    md.ellipse([cx - r, cy - r, cx + r, cy + r], fill=255)
    md.ellipse([cx - r * 0.45, cy - r * 1.15, cx + r * 1.5, cy + r * 0.75], fill=0)
    moon = Image.new("RGBA", (inner, inner), (255, 236, 200, 255))
    moon.putalpha(moon_mask)
    tile.alpha_composite(moon)

    # Буква S
    font = ImageFont.truetype(font_path, int(inner * 0.62))
    text = "S"
    box = d.textbbox((0, 0), text, font=font)
    tw, th = box[2] - box[0], box[3] - box[1]
    x = (inner - tw) / 2 - box[0]
    y = (inner - th) / 2 - box[1] + inner * 0.03
    d.text((x + 6, y + 10), text, font=font, fill=(20, 10, 50, 110))
    d.text((x, y), text, font=font, fill=(255, 255, 255, 255))

    # Светлая кромка сверху, как у стеклянных панелей Shiftella
    edge = Image.new("RGBA", (inner, inner), (0, 0, 0, 0))
    ImageDraw.Draw(edge).rounded_rectangle([2, 2, inner - 3, inner - 3], RADIUS - 2, outline=(255, 255, 255, 70), width=4)
    fade = Image.linear_gradient("L").resize((inner, inner)).point(lambda v: 255 - v)
    edge.putalpha(Image.composite(edge.getchannel("A"), Image.new("L", edge.size, 0), fade))
    tile.alpha_composite(edge)

    icon.alpha_composite(tile, (MARGIN, MARGIN))
    icon.save(out_path)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit("usage: make_icon.py <Inter-Bold.ttf> <out.png>")
    main(sys.argv[1], sys.argv[2])
