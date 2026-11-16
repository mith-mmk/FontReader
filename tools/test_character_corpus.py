import unittest
import unicodedata
from compare_character_corpus import corpus, same, same_union


class CorpusTest(unittest.TestCase):
    def test_unique_and_provenance(self):
        rows = corpus()
        self.assertEqual(len(rows), len({r['id'] for r in rows}))
        for row in rows:
            if row['group'] != 'iso-scalar':
                continue
            self.assertFalse(unicodedata.category(row['text']).startswith('C'))
            for part, byte in row['origins']:
                self.assertNotIn(part, (6, 12))
                self.assertEqual(bytes([byte]).decode(f'iso8859_{part}'), row['text'])
        self.assertTrue(any(r['id'] == 'iso-05D0' for r in rows))
        self.assertTrue(any(r['id'] == 'ipa-tie-NFD' for r in rows))

    def test_structure_is_not_ink_equivalence(self):
        a = dict(advance=10, glyphs=[dict(advance=10, bounds=[0, -5, 8, 0]), dict(advance=0, bounds=[2, -8, 4, -6])])
        b = dict(advance=10, glyphs=[dict(advance=10, bounds=[0, -8, 8, 0])])
        self.assertFalse(same(a, b))
        self.assertTrue(same_union(a, b))

    def test_integer_rounding_tolerance(self):
        a = dict(advance=10, glyphs=[dict(advance=10, bounds=[0.49, -5, 8, 0])])
        b = dict(advance=10, glyphs=[dict(advance=10, bounds=[0, -5, 8, 0])])
        self.assertTrue(same(a, b))
        a['glyphs'][0]['bounds'][0] = 1
        self.assertFalse(same(a, b))


if __name__ == '__main__':
    unittest.main()
