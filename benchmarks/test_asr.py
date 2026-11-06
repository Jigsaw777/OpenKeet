import unittest

from asr import score, stable_partial


class Metrics(unittest.TestCase):
    def test_cash_and_cache_are_distinct(self):
        result = score("LRU cache in Java!", "LRU cash in JAVA", ["LRU cache", "Java"])
        self.assertEqual((result["word_errors"], result["term_hits"], result["words"]), (1, 1, 4))

    def test_empty_reference_and_hypothesis(self):
        self.assertTrue(score("", "spurious speech", [])["false_transcript"])
        self.assertTrue(score("why?", "", [])["missed"])

    def test_partial_stability_requires_the_final_prefix(self):
        def event(kind, at, text):
            return kind, at, {"utterance_id": 1, "text": text}

        events = [event("partial_transcript", 1, "Do not"), event("partial_transcript", 2, "Do you"),
                  event("transcript", 3, "Do you agree?")]
        self.assertEqual(stable_partial(events), 2)
        self.assertIsNone(stable_partial(events[:2]))


if __name__ == "__main__":
    unittest.main()
