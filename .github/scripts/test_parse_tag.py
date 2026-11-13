import unittest
from parse_tag import classify

class TagTests(unittest.TestCase):
    def test_production(self):
        self.assertEqual(classify("v1.2.1.0", "1.2.1"), "production")
        with self.assertRaises(ValueError):
            classify("v1.2.1.0", "1.2.2")
    def test_testing(self):
        for tag in ("v1.2.1.4a", "v1.2.1.5b"):
            self.assertEqual(classify(tag, "0.1.0"), "testing")
    def test_invalid(self):
        for tag in ("v1.2.1.4", "v1.2.1.0a", "v1.2.1.4c", "v1.2.1.00", "v1.2.1.00b", "v1.2.1.0-extra"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                classify(tag, "1.2.1")

if __name__ == "__main__":
    unittest.main()
