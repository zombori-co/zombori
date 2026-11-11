"""Boot screen mockup: the logo and the OS name in a pixel font. They fade in smoothly from black,
then a glint sweeps across the sunglasses.
Exports a PNG of the final screen and an animated GIF of the whole sequence."""
from PIL import Image

W, H, S = 160, 100, 6                   # a tiny "virtual screen", shown 6x larger
BG = (13, 11, 20)
WHITE = (255, 246, 228)
OUTLINE = (26, 16, 32)
FPS = 25

FONT = {  # 5x7 glyphs
    'Z': ["#####", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
    'O': [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
    'M': ["#...#", "##.##", "#.#.#", "#.#.#", "#...#", "#...#", "#...#"],
    'B': ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."],
    'R': ["####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#"],
    'I': ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "#####"],
}

logo = Image.open('logo-32.png').convert('RGBA')
LENS_ROWS = (14, 15, 16)                # where the glint can travel
LENS_X = range(9, 24)

# Logo and name are centered as one block on the screen.
LOGO_Y = (H - (32 + 8 + 7)) // 2
TEXT_Y = LOGO_Y + 32 + 8


def fade(color, k):
    """Blend from the background toward `color`; k runs 0.0..1.0."""
    return tuple(round(b + (c - b) * k) for b, c in zip(BG, color))


def draw_logo(img, level, glint_x):
    face = logo.copy()
    if glint_x is not None:             # diagonal glint across the lens
        for dy, y in enumerate(LENS_ROWS):
            for x in (glint_x - dy, glint_x - dy + 1):
                if x in LENS_X and face.getpixel((x, y))[:3] != OUTLINE:
                    face.putpixel((x, y), (*WHITE, 255))
    x0 = (W - 32) // 2
    for y in range(32):
        for x in range(32):
            r, g, b, a = face.getpixel((x, y))
            if a:
                img.putpixel((x0 + x, LOGO_Y + y), fade((r, g, b), level))


def draw_text(img, s, level):
    x0 = (W - (len(s) * 6 - 1)) // 2
    for i, ch in enumerate(s):
        for gy, row in enumerate(FONT[ch]):
            for gx, px in enumerate(row):
                if px == '#':
                    img.putpixel((x0 + i * 6 + gx, TEXT_Y + gy), fade(WHITE, level))


def frame(logo_level, text_level, glint_x=None):
    img = Image.new('RGB', (W, H), BG)
    draw_logo(img, logo_level, glint_x)
    draw_text(img, 'ZOMBORI', text_level)
    return img.resize((W * S, H * S), Image.Resampling.NEAREST)


frame(1, 1).save('bootscreen.png')


def ease(t):
    """Smoothstep: starts and ends gently, so the fade has no visible edges."""
    t = min(max(t, 0.0), 1.0)
    return t * t * (3 - 2 * t)


# Timeline in seconds
BLACK, LOGO_FADE, TEXT_FADE, PAUSE, GLINT, HOLD = 0.6, 1.2, 1.0, 0.7, 1.0, 2.2
text_start = BLACK + LOGO_FADE * 0.6                # the name starts before the logo finishes
glint_start = text_start + TEXT_FADE + PAUSE
total = glint_start + GLINT + HOLD

frames = []
for i in range(round(total * FPS)):
    t = i / FPS
    logo_k = ease((t - BLACK) / LOGO_FADE)
    text_k = ease((t - text_start) / TEXT_FADE)
    g = (t - glint_start) / GLINT
    glint_x = 6 + round(g * 24) if 0 <= g < 1 else None
    frames.append(frame(logo_k, text_k, glint_x))

frames[0].save('bootscreen.gif', save_all=True, append_images=frames[1:],
               duration=1000 // FPS, loop=0, disposal=1)
print(f'{len(frames)} frames at {FPS} fps, {total:.1f}s')
