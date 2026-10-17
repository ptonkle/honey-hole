# Default style

This is Builder's fallback style. It is used only when a brief names no style. A brief that names a style, or a project that supplies its own pack, replaces it entirely.

The pack was drawn by eye from one reference picture: a warm city street under a mustard sky. That picture is not distributed with Builder, and its rights belong to its owner. Nothing in this pack reproduces it; the rules below describe a look, and the colours are values picked to match it.

## What a picture looks like

- A warm mustard sky in three flat horizontal bands: lighter at the top (`skyhi`), the main sky in the middle (`sky`), deeper near the horizon (`skylow`). The bands meet in straight lines with no outline and no gradient.
- Puffy clouds made of overlapping round puffs on a flat base, filled `cloud` and outlined as one shape. Use `motifs.cloud`, which draws the outline around the whole cloud and none between the puffs.
- Buildings in salmon, coral and pink (`wallA` to `wallF`). Neighbouring buildings take different wall roles. Windows are `window` when lit and `windark` when dark, mixed on each wall.
- Plants, awnings and machines in sage and teal (`greenA` to `greenD`, `teal`). Metal parts in `metal` and `metaldark`, timber in `wood`, paving and kerbs in `stone`.
- Flat fills only. No gradients, shading, drop shadows, glow or texture. A highlight, if any, is a second flat shape.
- Every filled shape is outlined in `ink`, with round joins and round caps. The art pipeline does this for you: `Fig` has no way to fill a shape without its outline, apart from the sky bands.
- Dense clutter on the rooftops and walls: water tanks, air conditioners, pipes, antennas, dishes, vents, laundry lines, wires between buildings, small signs made of coloured blocks.
- One oversized subject in the street, much larger than the people and buildings around it, with debris and small objects around its feet.

## Composition

- Keep the upper part of the left third calm: sky and clouds only. The page sets the title there. Mark it with `motifs.calm_left_third(fig)`, and the art check measures it on the rendered PNG.
- Put the subject right of centre, standing on the street. Let it overlap buildings so it reads as large.
- Run the street across the bottom quarter. Scatter debris on it, more near the subject and less toward the edges.
- Let buildings and clutter fill the middle band from the subject to the right edge, and thin out toward the left so the calm area meets the sky without a hard edge.
- A cutout (a unit or an icon) has a transparent background, keeps clear of the edges that `data/art.json` lists, and is centred.

## Sizes and line weights

Picture sizes, pixel sizes, transparency and file caps are in the project's `data/art.json`. The ink width depends on the kind of picture, from `ink.json`:

| Kind | Ink width |
| --- | --- |
| scene | 1.15 pt |
| unit | 1.5 pt |
| icon | 1.8 pt |

Smaller pictures get the heavier line so the outline survives being shown small. Thin interior lines (window mullions, wires, cracks) may be lighter, down to about half the ink width, and stay in `ink`.

## Palette by role

The values are in `palette.json`. Pictures name roles; they never name colour values.

| Role | Use |
| --- | --- |
| `ink` | every outline and every interior line |
| `sky`, `skyhi`, `skylow` | the three sky bands |
| `cloud` | clouds |
| `wallA` to `wallF` | building walls, awnings, large flat surfaces |
| `greenA` to `greenD`, `teal` | plants, machines, signs, awnings |
| `window`, `windark` | lit and dark windows |
| `metal`, `metaldark` | tanks, pipes, rails, machines |
| `wood` | crates, doors, poles |
| `stone` | street, kerbs, steps |
| `paper`, `cream` | posters, laundry, pale details |
| `gold` | small bright accents, used sparingly |

Mixes in xcolor's form, such as `wallA!60!windark`, are allowed when both names are roles.

## What not to do

- No words, letters or digits in any picture. Signs are coloured blocks and bars. The pipeline refuses TeX that could set text.
- No real person, brand, logo or known character.
- No colour outside the palette, and no colour value in picture code.
- No gradients, blur, photographic texture or soft shadows.
- No unoutlined filled shape apart from the sky bands.
- Nothing busy in the calm area, and no subject cut off by the frame.
- No pixel art and no generated images. Every picture is TikZ text written by a script.
