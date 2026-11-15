# Original kana composition fixtures

`kana-composition.ttc` and `generate_kana_composition.py` are original MIT-licensed synthetic test assets. No third-party font outlines are included. Regenerate with fontTools 4.68.0. Fixed timestamps make output reproducible.

The generator lists all 58 two-scalar kana canonical pairs from UnicodeData 17.0.0. Ten faces cover both forms, composed-only coverage, original-only coverage, .notdef targets, downstream GSUB and vertical alternates, mixed Hangul/kana contextual rollback, whole-grapheme fallback, and incomplete source coverage. Face descriptions are in the generator docstring. Nonzero mark advances intentionally detect accidental generic mark-zeroing. The four Ainu named sequences and all U+31F0–31FF characters are mapped without invented canonical compositions.
