# SVG extraction real-data fixtures

These are artwork, not fontcore source, and retain their separate licenses.

- `noto-emoji_u1f600.svg`: unmodified Google Noto Emoji grinning face artwork,
  from https://github.com/googlefonts/noto-emoji/blob/99701e07d3d1fe1bdbcdb14964547f0fd592966f/2D/svg/emoji_u1f600.svg .
  Copyright 2013 Google, Inc., Apache-2.0; see `NOTO-LICENSE`.
  This is a **standalone SVG asset**, not a claim that NotoColorEmoji.ttf has an SVG table.
- `twemoji-glyph5.svg`: unchanged, decompressed SVG table document for glyph 5
  from `TwitterColorEmoji-SVGinOT.ttf`, release v15.1.0 of
  https://github.com/13rac1/twemoji-color-font/releases/tag/v15.1.0 .
  Artwork CC-BY-4.0, attribution: Twitter (2014–2021), Jason Sofonia and
  Justine De Caires (2022–present), Brad Erickson (2016–2024).
  See `TWEMOJI-LICENSE.md` and accompanying complete CC-BY/MIT license texts.
  This extraction changes only the container: the SVG document bytes are unchanged.

Generated examples in `doc/svg-extraction/examples` prune/normalize these inputs;
those derivatives retain the same attribution and licenses. Twemoji's font em is
2048; the Noto asset's artboard is 128. For paintcore previews, use the respective
viewports `0 -1700 2048 2048` and `0 0 128 128`; the Twemoji SVG root intentionally
has overflow visible because its drawing extends above the font baseline.

- `noto-emoji_u1f308.svg`: unmodified Google Noto Emoji rainbow artwork from
  https://github.com/googlefonts/noto-emoji/blob/99701e07d3d1fe1bdbcdb14964547f0fd592966f/2D/svg/emoji_u1f308.svg .
  Copyright Google Inc., Apache-2.0, same `NOTO-LICENSE` as the grinning face.
  SHA256 `23a80becfb38d89ebf3ab8aa57136dce162bc74daac358668694e07499101fba`.
  This standalone asset exercises Illustrator's inert enable-background export metadata.
