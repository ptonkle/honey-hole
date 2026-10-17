"""The project's pictures, registered by name for tools/art/art.py.

Each entry maps a name to a function that takes a seeded random.Random and
returns an artlib.Fig. The name is the output path under web/art/ without
".png"; its first part may be a folder. Colours are role names from the
style pack (data/art.json), never colour values. Larger projects may split
pictures into tools/art/art_<group>.py modules, each with its own PICTURES.

A tiny example, left commented out so a new project ships with no art:

    # from artlib import Fig
    #
    # def badge(rng):
    #     fig = Fig("icon")                      # size from data/art.json
    #     fig.circle(1.0, 1.0, 0.7, "gold")      # outlined in ink by Fig
    #     with fig.frame(1.0, 1.0, s=0.4, rot=rng.uniform(-10, 10)):
    #         fig.poly([(-1, -1), (1, -1), (0, 1)], "teal")
    #     return fig
    #
    # PICTURES["icons/badge"] = badge
"""

PICTURES = {}
