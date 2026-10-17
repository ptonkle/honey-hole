"""Motifs for Builder's default style pack: the banded sky and the outlined
puffy cloud. Each takes an artlib.Fig and role names, so a pack with other
values for the same roles draws the same shapes in its own colours.

    import style
    motifs = style.load().motifs()
    motifs.sky_bands(fig)
    motifs.cloud(fig, 4.0, 8.5, 0.9)
    motifs.calm_left_third(fig)
"""

PUFFS = [(-1.0, 0.0, 0.55), (-0.45, 0.25, 0.7), (0.2, 0.35, 0.8), (0.85, 0.15, 0.6), (1.35, -0.05, 0.45)]


def sky_bands(fig, low="skylow", mid="sky", high="skyhi", splits=(0.3, 0.78)):
    """Three flat bands across the whole picture: low, mid and high, split at
    these fractions of the height. Bands carry no outline."""
    W, H = fig.size
    fig.band(0, H, mid)
    fig.band(H * splits[1], H, high)
    fig.band(0, H * splits[0], low)


def cloud(fig, x, y, s=1.0, fill="cloud"):
    """A cloud of five puffs on a flat base, outlined as one shape. (x, y) is
    the middle of the base; s scales it (s=1 is about 3.3 cm wide)."""
    with fig.union(fill):
        for dx, dy, r in PUFFS:
            fig.circle(x + dx * s, y + dy * s, r * s)
        fig.rect(x - 1.0 * s, y - 0.15 * s, x + 1.35 * s, y + 0.1 * s)


def clouds(fig, rng, n=4, band=(0.62, 0.9), scale=(0.6, 1.0), fill="cloud", x_from=0.0):
    """n clouds scattered across the upper sky, from x_from (a fraction of the
    width) to the right edge."""
    W, H = fig.size
    for i in range(n):
        lo = x_from + (1 - x_from) * i / n
        hi = x_from + (1 - x_from) * (i + 1) / n
        cloud(fig, W * rng.uniform(lo, hi), H * rng.uniform(*band), rng.uniform(*scale), fill)


def calm_left_third(fig, top=0.55, roles=("skylow", "sky", "skyhi", "cloud", "ink"), min_share=0.95):
    """Mark the upper part (this fraction of the height) of the left third as
    calm, so tools/art/art.py checks that it holds only sky and cloud colours
    and the page can set the title over it."""
    W, H = fig.size
    fig.keep_calm((0, H * (1 - top), W / 3, H), roles, min_share)
