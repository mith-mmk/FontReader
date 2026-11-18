#!/usr/bin/env python3
"""Optional real-artwork pixel oracle; requires CairoSVG and Pillow.

Does not test paintcore. Normalize source roots to OpenType's non-clipped em
viewport, render source/extracted documents with the same outer viewport, and
compare all RGBA channels. No network, no persistent build products.
"""
import io
import json
from pathlib import Path
import xml.etree.ElementTree as ET

import cairosvg
from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
ET.register_namespace("", "http://www.w3.org/2000/svg")
results = []
for name, source, output, em, viewport in [
    ("noto", "noto-emoji_u1f600.svg", "noto-smile.svg", 128, "0 0 128 128"),
    ("twemoji", "twemoji-glyph5.svg", "twemoji-glyph5.svg", 2048, "0 -1700 2048 2048"),
]:
    reference = ET.parse(ROOT / "tests/fixtures/svg-extraction" / source).getroot()
    reference.set("width", str(em))
    reference.set("height", str(em))
    reference.set("overflow", "visible")
    payloads = [ET.tostring(reference, encoding="unicode"),
                (ROOT / "doc/svg-extraction/examples" / output).read_text()]
    images = []
    for payload in payloads:
        document = f'<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="{viewport}">{payload}</svg>'
        images.append(Image.open(io.BytesIO(cairosvg.svg2png(bytestring=document.encode()))).convert("RGBA"))
    diff = ImageChops.difference(*images)
    # RGBA.getbbox() can ignore color differences where alpha difference is zero.
    same = all(channel.getbbox() is None for channel in diff.split())
    nonempty = images[1].getchannel("A").getbbox() is not None
    results.append({"fixture": name, "identical_rgba": same, "nonempty": nonempty})
    assert same and nonempty, results[-1]
print(json.dumps({"renderer": "CairoSVG", "version": cairosvg.__version__, "pixels": [256, 256], "results": results}, indent=2))
