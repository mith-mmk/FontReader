"""Generate original MIT mark-coordinate fixture with fontTools 4.65.0.

Run from repository root; also regenerates the unchanged vertical-flow fixtures.
No vertical substitutions: AB -> B is only used to exercise ligature attachment.
"""
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from generate_vertical_flow import build, ROOT

font = build(False).font
addOpenTypeFeaturesFromString(font, """
languagesystem DFLT dflt;
markClass acute <anchor 100 700> @TOP;
feature liga { sub A B by B; } liga;
feature mark {
    pos base A <anchor 300 700> mark @TOP;
    pos base B <anchor 300 500> mark @TOP;
    pos ligature B <anchor 100 500> mark @TOP
        ligComponent <anchor 800 500> mark @TOP;
} mark;
feature mkmk { pos mark acute <anchor 100 800> mark @TOP; } mkmk;
""")
font.save(ROOT / "mark-coordinates.ttf")
