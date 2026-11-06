"""Tests du script de carte vitrine : python3 -I scripts/showcase_test.py"""

import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image, ImageDraw

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


    def test_the_caption_fits_the_right_column(self):
        root = Path(__file__).resolve().parent.parent
        width = ImageDraw.Draw(Image.new("RGB", (1, 1))).textlength(
            showcase.CAPTION, font=showcase.font(root, showcase.CAPTION_PX)
        )
        self.assertLessEqual(width, showcase.SIZE[0] - showcase.MARGIN - showcase.COLUMN_X)

    def test_a_capture_without_separator_has_no_row(self):
        self.assertIsNone(showcase.separator_row(Image.new("RGB", (8, 8), (255, 255, 255))))

    def test_a_blank_card_does_not_contain_the_capture(self):
        root = Path(__file__).resolve().parent.parent
        blank = Image.new("RGB", showcase.SIZE, showcase.BACKGROUND)
        self.assertFalse(showcase.contains_capture(root, blank))


if __name__ == "__main__":
    unittest.main()
