from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont


OUTPUT = Path(__file__).parent
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"


def draw_lines(image, lines, color, font_size, position=(36, 30), spacing=20):
    draw = ImageDraw.Draw(image)
    font = ImageFont.truetype(FONT, font_size)
    x, y = position
    for line in lines:
        draw.text((x, y), line, fill=color, font=font)
        y += font_size + spacing


clean = Image.new("RGB", (760, 190), "white")
draw_lines(clean, ["Snipsy OCR sample.", "Save 1 PNG."], "#161616", 36)
clean.save(OUTPUT / "clean-lines.png")

settings = Image.new("RGB", (960, 190), "white")
settings_draw = ImageDraw.Draw(settings)
settings_draw.rectangle((10, 10, 949, 179), outline="#c9c9c9", width=2)
draw_lines(
    settings,
    ["Window 2: Settings", "Scale 150% | DPI-aware."],
    "#202020",
    34,
    position=(34, 28),
    spacing=18,
)
settings.save(OUTPUT / "settings-text.png")

low_contrast = Image.new("RGB", (760, 180), "#eeeeee")
draw_lines(
    low_contrast,
    ["Capture at 08:42", "Status: READY"],
    "#777777",
    30,
    position=(34, 30),
    spacing=18,
)
low_contrast.filter(ImageFilter.GaussianBlur(radius=0.45)).save(
    OUTPUT / "low-contrast.png"
)
