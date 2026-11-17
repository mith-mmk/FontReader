#!/usr/bin/env python3
"""Read-only local font inventory; no downloads or claim of shaping support."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
from fontTools.ttLib import TTCollection, TTFont


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    rows, errors = [], []
    files = faces = 0
    total = Counter()
    for path in sorted(args.root.rglob('*')):
        if path.suffix.lower() not in ('.ttf', '.otf', '.ttc'):
            continue
        files += 1
        try:
            count = 1
            if path.suffix.lower() == '.ttc':
                collection = TTCollection(path, lazy=True)
                count = len(collection.fonts)
                collection.close()
            for index in range(count):
                font = TTFont(path, fontNumber=index, lazy=True)
                try:
                    faces += 1
                    gdef = font.get('GDEF')
                    listing = getattr(gdef.table, 'LigCaretList', None) if gdef else None
                    formats = Counter(value.Format for glyph in listing.LigGlyph
                                      for value in glyph.CaretValue) if listing else Counter()
                    total.update(formats)
                    if formats:
                        rows.append(dict(font=str(path.relative_to(args.root)), face=index,
                            sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                            formats=dict(formats), has_vmtx='vmtx' in font,
                            has_fvar='fvar' in font, ligature_glyphs=len(listing.LigGlyph)))
                finally:
                    font.close()
        except Exception as error:
            errors.append(dict(font=str(path.relative_to(args.root)), error=str(error)))
    result = dict(schema=1, scope='Installed font files only; table presence is not runtime applicability',
                  file_count=files, face_count=faces, caret_value_formats=dict(total),
                  faces_with_carets=rows, errors=errors)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(dict(files=files, faces=faces, formats=dict(total), errors=len(errors))))


if __name__ == '__main__':
    main()
