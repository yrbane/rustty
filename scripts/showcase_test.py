"""Tests du script de carte vitrine : python3 -I -m unittest scripts/showcase_test.py"""

import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import showcase  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent


class ShowcaseTest(unittest.TestCase):
    def test_card_has_the_expected_size_and_includes_the_window_capture(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "card.webp"
            showcase.compose(ROOT, out)
            im = Image.open(out)
            self.assertEqual(im.size, (1200, 630))
            self.assertTrue(showcase.contains_capture(ROOT, im))

    def test_the_capture_box_fits_the_right_column(self):
        x, y, w, h = showcase.capture_box(ROOT)
        self.assertGreaterEqual(x, showcase.COLUMN_X)
        self.assertLessEqual(x + w, 1200 - showcase.MARGIN)
        self.assertLessEqual(y + h, 630 - showcase.MARGIN)


if __name__ == "__main__":
    unittest.main()
