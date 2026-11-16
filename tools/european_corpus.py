"""Bounded European text cases, independent of ISO-derived scalar coverage."""
import unicodedata


def corpus():
    rows = []
    def add(name, text, group, locale='de'):
        rows.append(dict(id=name, text=text, group=group, direction='ltr', locale=locale))
    for text in ('fi', 'fl', 'ff', 'ffi', 'ffl', 'st', 'ct', 'office', 'afflict',
                 'fifi', 'f f i', 'f\u200ci', 'f\u0308i', 'fi\u0308'):
        # Numeric IDs avoid encoding-dependent command-line/report identifiers.
        add(f'ligature-{len(rows):02}', text, 'default-ligature', 'en')
    for i, text in enumerate(('ß', 'ẞ', 'ss', 'SS', 'Straße', 'STRAẞE', 'STRASSE',
                              'Maße', 'Masse', 'groß', 'GRÖẞE', 'æ', 'Æ', 'œ', 'Œ')):
        add(f'letter-{i:02}', text, 'distinct-letter')
    for i, text in enumerate(('ä', 'ö', 'ü', 'Ä', 'Ö', 'Ü', 'für', 'größer',
                              'Mädchen', 'Übergröße', 'ÄÖÜ äöü', 'fünf', 'flüchtig')):
        for form in ('NFC', 'NFD'):
            add(f'umlaut-{i:02}-{form}', unicodedata.normalize(form, text), 'umlaut')
    return rows
