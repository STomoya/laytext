"""Validates `laytext.estimate_page_skew` on the real-data corpus.

The corpus has no scanned-at-an-angle pages, so skewed pages are
synthesized: each char's center is rotated about the page center by a
known angle and its box replaced by the axis-aligned bound of the rotated
glyph rect, which is what an OCR engine emits for a page scanned at that
angle. The expected estimate is the applied angle plus the page's own
measured skew (the median least-squares slope of pdfium's text lines,
independent of laytext).

Fails if, on horizontal-text pages:
  - any unmodified page's estimate is off by more than 0.15 deg (the
    correction noise floor) on more than 1% of pages, or
  - fewer than 90% of rotated pages are within 0.25 deg, or
  - any rotated page's estimate has the wrong sign.

Usage:
    uv run python validation/skew_estimator.py
"""

import math
import statistics
import sys

import pypdfium2 as pdfium
from compare_pdfminer import PDF_DIR, extract_page_char_boxes

import laytext

ANGLES = [-1.0, 0.3, 0.5, 1.0, 2.0, 3.0]
MIN_CHARS_FOR_ROTATION = 300
MIN_CHARS_PER_MEASURED_LINE = 10
VERTICAL_TEXT_DEGREES = 45.0
MAX_LINE_DEGREES = 15.0
NOISE_FLOOR_DEGREES = 0.15
ROTATED_TOLERANCE_DEGREES = 0.25
MAX_FLAT_OFF_FRACTION = 0.01
MIN_ROTATED_WITHIN_FRACTION = 0.9


def fit_angle(points: list[tuple[float, float]]) -> float | None:
    """Least-squares angle (degrees) of y vs x, or None for no x spread."""
    n = len(points)
    sx = sum(x for x, _ in points)
    sy = sum(y for _, y in points)
    sxx = sum(x * x for x, _ in points)
    sxy = sum(x * y for x, y in points)
    d = n * sxx - sx * sx
    return math.degrees(math.atan((n * sxy - sx * sy) / d)) if d > 0 else None


def measured_skew(page: pdfium.PdfPage) -> float | None:
    """Median line angle using pdfium's own line breaks, not laytext's grouping.

    pdfium emits no break where table-cell text wraps, so a line is also
    split wherever x jumps back left. Returns None for a vertical-text page
    (median line angle beyond 45deg); otherwise the median over lines
    within 15deg, since steeper fits are stacked chars in narrow cells.
    """
    textpage = page.get_textpage()
    lines, cur = [], []
    for i in range(textpage.count_chars()):
        text = textpage.get_text_range(i, 1)
        if text in ('\n', '\r', ''):
            lines.append(cur)
            cur = []
            continue
        x0, y0, x1, y1 = textpage.get_charbox(i, loose=True)
        if x1 > x0 and y1 > y0:
            if cur and (x0 + x1) / 2 < cur[-1][0]:
                lines.append(cur)
                cur = []
            cur.append(((x0 + x1) / 2, y0))
    lines.append(cur)
    angles = [a for line in lines if len(line) >= MIN_CHARS_PER_MEASURED_LINE and (a := fit_angle(line)) is not None]
    if angles and abs(statistics.median(angles)) > VERTICAL_TEXT_DEGREES:
        return None
    flat = [a for a in angles if abs(a) <= MAX_LINE_DEGREES]
    return statistics.median(flat) if flat else 0.0


def rotate(boxes: list, angle: float, cx: float, cy: float) -> list[laytext.Char]:
    """Rotate char centers about (cx, cy); re-bound each rotated glyph rect."""
    c, s = math.cos(math.radians(angle)), math.sin(math.radians(angle))
    chars = []
    for (x0, y0, x1, y1), text in boxes:
        mx, my = (x0 + x1) / 2 - cx, (y0 + y1) / 2 - cy
        rx, ry = cx + mx * c - my * s, cy + mx * s + my * c
        hw = ((x1 - x0) * abs(c) + (y1 - y0) * abs(s)) / 2
        hh = ((x1 - x0) * abs(s) + (y1 - y0) * abs(c)) / 2
        chars.append(laytext.Char(laytext.Rect(rx - hw, ry - hh, rx + hw, ry + hh), text))
    return chars


def main() -> int:
    """Run the flat and rotated checks over the corpus and print the report."""
    pdfs = sorted(PDF_DIR.glob('*.pdf'))
    if not pdfs:
        print(f'no PDFs found in {PDF_DIR}', file=sys.stderr)
        return 1

    flat_total = flat_off = 0
    flat_worst: list[tuple[float, str]] = []
    rotated: dict[float, list[float]] = {a: [] for a in ANGLES}
    wrong_sign: list[str] = []
    for pdf_path in pdfs:
        pdf = pdfium.PdfDocument(pdf_path)
        for page_idx in range(len(pdf)):
            page = pdf[page_idx]
            boxes = extract_page_char_boxes(page)
            base = measured_skew(page)
            if not boxes or base is None:
                continue  # vertical text: no horizontal skew to estimate
            label = f'{pdf_path.name} page {page_idx}'
            chars = [laytext.Char(laytext.Rect(*b), t) for b, t in boxes]
            err = abs(laytext.estimate_page_skew(chars) - base)
            flat_total += 1
            if err > NOISE_FLOOR_DEGREES:
                flat_off += 1
                flat_worst.append((err, label))
            if len(boxes) < MIN_CHARS_FOR_ROTATION:
                continue
            w, h = page.get_size()
            for angle in ANGLES:
                expected = base + angle
                est = laytext.estimate_page_skew(rotate(boxes, angle, w / 2, h / 2))
                rotated[angle].append(est - expected)
                if est * expected < 0 and abs(est) > NOISE_FLOOR_DEGREES:
                    wrong_sign.append(f'{label} @ {angle:+.1f}deg: estimate {est:+.2f}')

    ok = True
    print(f'unmodified pages: {flat_off}/{flat_total} off by > 0.15deg')
    for err, label in sorted(flat_worst, reverse=True)[:10]:
        print(f'  {label}: off by {err:.2f}deg')
    if flat_off > MAX_FLAT_OFF_FRACTION * flat_total:
        ok = False

    for angle, errs in rotated.items():
        within = sum(abs(e) <= ROTATED_TOLERANCE_DEGREES for e in errs)
        print(f'rotated {angle:+.1f}deg: {within}/{len(errs)} within 0.25deg, max |err| {max(map(abs, errs)):.2f}')
        if within < MIN_ROTATED_WITHIN_FRACTION * len(errs):
            ok = False
    if wrong_sign:
        ok = False
        print(f'wrong-sign estimates: {len(wrong_sign)}')
        for line in wrong_sign[:10]:
            print(f'  {line}')

    print('PASS' if ok else 'FAIL')
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
