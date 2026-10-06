# Original Vlak material

Original game and graphics: Miroslav Němeček, 1993.
The author explicitly allows reuse of the source, graphics, and sounds for any
purpose, including commercial use:
https://www.breatharian.eu/sw/vlak/index.html

- `original-sprites.png`: the full 320×200 historical sprite sheet supplied by
  the user. It includes three pickup animation frames, four wagon orientations,
  three locomotive animation phases, crash frames, and six gate frames.
- `original-title.png`: the 320×200 title screen supplied by the user. Its wall
  texture is also used to reproduce the original small brick pattern.
- `alternate-sprites.png`: the additional 320×48 strip supplied by the user.
  Its airplane and passenger icons cover pickups absent from the full sheet;
  they use static frames and icons in place of missing wagon variants.
- `original-levels.scn`: all 50 original scene layouts transcribed from the
  author's `HRY/VLAK/V100/SOURCE/SCENY.ASM` at
  https://github.com/Panda381/DOS-Progs/blob/main/HRY/VLAK/V100/SOURCE/SCENY.ASM
  with passwords and solutions from
  https://www.breatharian.eu/sw/vlak/dwn/MACRO.TXT

The clipboard PNGs contain compression artifacts from their historical image
source. The renderer maps their colors to the nearest of the sixteen EGA colors
to remove those artifacts. This preserves the supplied artwork; it is a recovery
from the supplied images, not a claim of byte-identical original source pixels.
The bitmap UI font is newly implemented and includes Czech diacritics.
