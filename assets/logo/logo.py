"""Hand-designed 32x32 pixel-art logo. Shapes are placed by code, details by hand."""
import math
from PIL import Image

N = 32
PALETTE = {
    '.': None,
    'K': (26, 16, 32),     # outline
    'H': (74, 42, 30),     # hair dark / beard
    'h': (122, 74, 44),    # hair mid
    'l': (184, 126, 74),   # hair highlight
    'S': (238, 182, 146),  # skin
    's': (196, 128, 102),  # skin shadow
    'O': (255, 122, 26),   # lens
    'P': (255, 176, 72),   # lens light
    'o': (196, 70, 16),    # lens dark
    'W': (255, 246, 228),  # teeth / glint
    'M': (110, 26, 30),    # mouth
    'T': (30, 34, 52),     # tee
    't': (52, 58, 82),     # tee light
}

g = [['.'] * N for _ in range(N)]

def put(x, y, c):
    if 0 <= x < N and 0 <= y < N:
        g[y][x] = c

def inside_ellipse(x, y, cx, cy, rx, ry):
    return ((x + .5 - cx) / rx) ** 2 + ((y + .5 - cy) / ry) ** 2 <= 1

# --- tee + neck -------------------------------------------------------------
for y in range(26, N):
    for x in range(N):
        half = 9 + (y - 26) * 2          # shoulders widen downward
        if abs(x + .5 - 16) <= min(half, 14):
            put(x, y, 'T')
for y in range(26, N):                   # light catching the top of the shoulders
    row = [x for x in range(N) if g[y][x] == 'T']
    if row:
        put(row[0], y, 't')
        if y < 29:
            put(row[0] + 1, y, 't')
for x, y in [(12, 27), (13, 28), (14, 29), (15, 29), (16, 29), (17, 29), (18, 28), (19, 27)]:
    put(x, y, 'l')                       # thin chain

# --- hair cloud: ellipse + ring of curls ----------------------------------------
curls = [(16 + 12.5 * math.cos(a), 12 + 10.5 * math.sin(a))
         for a in [i * math.tau / 14 for i in range(14)]]
for y in range(N):
    for x in range(N):
        if y > 24:
            continue
        if inside_ellipse(x, y, 16, 12, 12.5, 10.5) or any(
                (x + .5 - cx) ** 2 + (y + .5 - cy) ** 2 <= 2.6 ** 2 for cx, cy in curls):
            put(x, y, 'h')

# --- face -----------------------------------------------------------------------
for y in range(N):
    for x in range(N):
        if inside_ellipse(x, y, 16, 17.5, 7, 9) and y <= 26:
            put(x, y, 'S')
for y in range(10, 26):                  # shadow on the right side of the face
    for x in range(16, 24):
        if g[y][x] == 'S' and not inside_ellipse(x, y, 15.2, 17.5, 6.6, 8.8):
            put(x, y, 's')

# fringe: curls hanging over the forehead
for x in range(8, 25):
    bottom = 11 + (1 if x % 3 == 1 else 0) - (1 if x % 3 == 0 else 0)
    for y in range(7, bottom + 1):
        if g[y][x] in 'Ss':
            put(x, y, 'h')

# hair shading: light comes from the upper left, so the lower right and the hair
# right next to the face fall into shadow
for y in range(N):
    for x in range(N):
        if g[y][x] == 'h':
            beside_face = any(g[y + dy][x + dx] in 'Ss'
                              for dx in (-1, 0, 1) for dy in (-1, 0, 1)
                              if 0 <= x + dx < N and 0 <= y + dy < N)
            if (x - 16) * 0.55 + (y - 12) * 0.85 > 7.5 or beside_face and y > 12:
                put(x, y, 'H')

# curls: each one is a small ring, lit on its upper-left half, shadowed on the lower right
#   . l l .
#   l . . H
#   . H H .
def curl(cx, cy):
    cells = [(1, 0, 'l'), (2, 0, 'l'), (0, 1, 'l'), (3, 1, 'H'), (1, 2, 'H'), (2, 2, 'H')]
    if all(0 <= cy + dy < N and 0 <= cx + dx < N and g[cy + dy][cx + dx] in 'hH'
           for dx, dy, _ in cells):
        for dx, dy, c in cells:
            base = g[cy + dy][cx + dx]
            put(cx + dx, cy + dy, c if not (base == 'H' and c == 'l') else 'h')

for cy in range(1, 24, 4):
    for cx in range(1 + (2 if (cy // 4) % 2 else 0), 30, 5):
        jitter = (cx * 7 + cy * 13) % 5          # fixed pseudo-random offsets break up the grid
        curl(cx + (jitter % 3) - 1, cy + (1 if jitter > 2 else 0))

# --- sunglasses: one-piece orange wraparound shield --------------------------------
for x in range(8, 25):
    put(x, 13, 'K')                      # top frame
for y in (14, 15, 16):
    for x in range(8, 25):
        put(x, y, 'K' if x in (8, 24) else ('P' if y == 14 else 'O'))
for x in range(9, 24):
    put(x, 16, 'o')
for x in (15, 16, 17):                   # nose bridge notch
    put(x, 16, 'K')
put(16, 17, 's')
for x in range(9, 15):
    put(x, 17, 'K')
for x in range(18, 24):
    put(x, 17, 'K')
put(10, 14, 'W'); put(11, 14, 'W'); put(10, 15, 'P')       # glints
put(19, 14, 'W'); put(19, 15, 'P')

# --- nose, mustache, grin, goatee ---------------------------------------------------
put(16, 18, 's')
for x in range(13, 20):
    put(x, 19, 'H')                      # thin mustache
put(12, 19, 's'); put(20, 19, 's')       # smile lines
put(12, 20, 'K')
for x in range(13, 20):
    put(x, 20, 'W')                      # teeth
put(20, 20, 'K')
put(13, 21, 'K')
for x in range(14, 19):
    put(x, 21, 'M')
put(19, 21, 'K')
for x in range(14, 19):
    put(x, 22, 'K' if x in (14, 18) else 's')
for y, xs in [(23, range(15, 18)), (24, range(14, 19)), (25, range(15, 18))]:
    for x in xs:
        put(x, y, 'H')                   # goatee

# --- outline everything that touches the transparent background ------------------
filled = [[g[y][x] != '.' for x in range(N)] for y in range(N)]
for y in range(N):
    for x in range(N):
        if not filled[y][x] and any(
                0 <= x + dx < N and 0 <= y + dy < N and filled[y + dy][x + dx]
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
            put(x, y, 'K')

# --- export ----------------------------------------------------------------------------
img = Image.new('RGBA', (N, N))
for y in range(N):
    for x in range(N):
        c = PALETTE[g[y][x]]
        img.putpixel((x, y), (*c, 255) if c else (0, 0, 0, 0))
img.save('logo-32.png')
img.resize((N * 12, N * 12), Image.Resampling.NEAREST).save('logo-32-x12.png')
print('\n'.join(''.join(r) for r in g))
