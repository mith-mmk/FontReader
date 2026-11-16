import unittest
from european_corpus import corpus


class EuropeanCorpusTests(unittest.TestCase):
    def test_unique_ids_and_horizontal_scope(self):
        rows = corpus()
        self.assertEqual(len(rows), 55)
        self.assertEqual(len({r['id'] for r in rows}), len(rows))
        self.assertTrue(all(r['direction'] == 'ltr' for r in rows))

    def test_distinct_letters_not_replaced(self):
        texts = {r['text'] for r in corpus()}
        self.assertTrue({'ß', 'ẞ', 'ss', 'SS', 'æ', 'œ'} <= texts)

    def test_all_six_umlauts_have_both_forms(self):
        import unicodedata
        texts = {r['text'] for r in corpus()}
        for char in 'äöüÄÖÜ':
            self.assertIn(char, texts)
            self.assertIn(unicodedata.normalize('NFD', char), texts)


if __name__ == '__main__':
    unittest.main()
