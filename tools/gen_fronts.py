#!/usr/bin/env python3
"""The tier LED strip on each recycling factory's front texture.

Upstream's factory model composes a base cube with a `mekanism:block/factory/
front_led/<tier>` child: a 6.04 x 1 block-unit panel at the top of the front
face whose UV (`[0, <row>, 6, <row>+1]`) picks one row of Mekanism's
`block/factory/led.png`. That texture's rows 0-3 are the four tiers' LED
colours, and `basic`/`advanced`/`elite`/`ultimate` use rows 0/1/2/3
(`assets/mekanism/models/block/factory/front_led/<tier>.json`).

Horizon has no `neoforge:composite` model loader, so this port draws the cube
with the mod's own five face textures (slice 1). This script bakes the tier row
into the front face where the composite's panel sits: six pixels wide, one
pixel tall, at the top-centre of the 16x16 face.

The palette is Mekanism's (`factory/led.png` rows 0-3, exact RGB); the base art
is Mekanism: MoreMachine's own `recycling_factory_front.png` (MIT). Run from the
port root:

    python3 tools/gen_fronts.py
"""
from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
FRONT = ROOT / "assets/mekmm/textures/block/factory/recycling"

# (row of Mekanism's factory/led.png, bright, dark)
TIERS = {
    "basic": ((39, 255, 161), (42, 224, 151)),  # led.png row 0
    "advanced": ((221, 117, 93), (206, 80, 69)),  # row 1
    "elite": ((117, 229, 229), (44, 222, 223)),  # row 2
    "ultimate": ((200, 90, 203), (185, 78, 196)),  # row 3
}

# The panel's place on the front face: u 5..10, the second row from the top.
LED_X = range(5, 11)
LED_Y = 1


def main() -> int:
    source = Image.open(FRONT / "recycling_factory_front.png").convert("RGBA")
    if source.size != (16, 16):
        print(f"{FRONT}/recycling_factory_front.png is {source.size}, expected 16x16", file=sys.stderr)
        return 1
    for tier, (bright, dark) in TIERS.items():
        image = source.copy()
        for offset, x in enumerate(LED_X):
            colour = bright if offset % 2 == 0 else dark
            image.putpixel((x, LED_Y), (*colour, 255))
        # The base tier keeps the carried file name (it is the texture slice 1
        # already shipped); the higher tiers get their own.
        name = "recycling_factory_front.png" if tier == "basic" else f"recycling_factory_front_{tier}.png"
        image.save(FRONT / name)
        print(f"wrote {name} ({tier}: rgb{bright})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
