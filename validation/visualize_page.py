"""Overlays pdfminer.six and laytext line/block bboxes on a rendered PDF page.

Dev-only visual debugging tool (not part of the validation/test suite): draws
both engines' bboxes directly on the page so a person can eyeball exactly
where the grouping agrees or diverges on a specific page. Complements the
corpus-wide numeric comparison in validation/compare_pdfminer.py, whose
helper functions (char extraction, pdfminer box-finding) this script reuses.

Usage:
  uv run python validation/visualize_page.py data/pdfs/FILE.pdf --page 0 --out page.png
  uv run python validation/visualize_page.py data/pdfs/FILE.pdf --page 0 --blocks --out page.png
"""

import argparse
import pathlib

import compare_pdfminer as cp
import laytext
import pdfminer.high_level
import pdfminer.layout
import pypdfium2 as pdfium
from PIL import ImageDraw, ImageFont

PDFMINER_COLOR = (235, 104, 52)  # orange
LAYTEXT_COLOR = (42, 120, 214)  # blue
LINE_WIDTH = 2
DASH_LEN = 6


def pdf_bbox_to_px(bbox: cp.BBox, page_height: float, scale: float) -> tuple[float, float, float, float]:
    """Convert a PDF-point bbox (origin bottom-left) to pixel coords (origin top-left)."""
    x0, y0, x1, y1 = bbox
    return (x0 * scale, (page_height - y1) * scale, x1 * scale, (page_height - y0) * scale)


def draw_box(draw: ImageDraw.ImageDraw, box: tuple[float, float, float, float], color, dashed: bool = False) -> None:
    """Draw a solid or dashed rectangle outline."""
    if not dashed:
        draw.rectangle(box, outline=color, width=LINE_WIDTH)
        return
    x0, y0, x1, y1 = (round(v) for v in box)
    for x in range(x0, x1, DASH_LEN * 2):
        seg = min(x + DASH_LEN, x1)
        draw.line([(x, y0), (seg, y0)], fill=color, width=LINE_WIDTH)
        draw.line([(x, y1), (seg, y1)], fill=color, width=LINE_WIDTH)
    for y in range(y0, y1, DASH_LEN * 2):
        seg = min(y + DASH_LEN, y1)
        draw.line([(x0, y), (x0, seg)], fill=color, width=LINE_WIDTH)
        draw.line([(x1, y), (x1, seg)], fill=color, width=LINE_WIDTH)


def draw_legend(draw: ImageDraw.ImageDraw, pdf_name: str, page_idx: int, pm_n: int, lt_n: int) -> None:
    """Draw a small caption block in the top-left corner."""
    font = ImageFont.load_default()
    lines = [
        f'{pdf_name}  page {page_idx}',
        f'pdfminer lines: {pm_n} (orange, dashed = block)',
        f'laytext  lines: {lt_n} (blue, dashed = block)',
    ]
    pad = 6
    line_h = 14
    w = max(draw.textlength(t, font=font) for t in lines) + pad * 2
    h = len(lines) * line_h + pad * 2
    draw.rectangle((4, 4, 4 + w, 4 + h), fill=(255, 255, 255, 230), outline=(0, 0, 0))
    for i, text in enumerate(lines):
        draw.text((4 + pad, 4 + pad + i * line_h), text, fill=(0, 0, 0), font=font)


def main() -> int:
    """Render one PDF page with both engines' line/block bboxes overlaid; write a PNG."""
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('pdf', type=pathlib.Path)
    parser.add_argument('--page', type=int, default=0, help='zero-indexed page number')
    parser.add_argument('--out', type=pathlib.Path, default=pathlib.Path('page_boxes.png'))
    parser.add_argument('--dpi', type=float, default=150.0)
    parser.add_argument('--blocks', action='store_true', help='also overlay block-level bboxes (dashed)')
    parser.add_argument('--column-gap-min', type=float, default=20.0)
    parser.add_argument('--row-gap-min', type=float, default=15.0)
    args = parser.parse_args()

    la_params = pdfminer.layout.LAParams(all_texts=True)
    params = laytext.Params(
        char_margin=la_params.char_margin,
        line_overlap=la_params.line_overlap,
        line_margin=la_params.line_margin,
        word_margin=la_params.word_margin,
        column_gap_min=args.column_gap_min,
        row_gap_min=args.row_gap_min,
    )

    pdf = pdfium.PdfDocument(str(args.pdf))
    page = pdf[args.page]
    page_height = page.get_size()[1]
    scale = args.dpi / 72

    image = page.render(scale=scale).to_pil().convert('RGB')
    draw = ImageDraw.Draw(image)

    def to_px(bbox: cp.BBox) -> tuple[float, float, float, float]:
        return pdf_bbox_to_px(bbox, page_height, scale)

    pm_pages = list(pdfminer.high_level.extract_pages(str(args.pdf), laparams=la_params, page_numbers=[args.page]))
    pm_boxes = cp.find_text_boxes(pm_pages[0])
    pm_line_count = 0
    for box in pm_boxes:
        if args.blocks:
            draw_box(draw, to_px(box.bbox), PDFMINER_COLOR, dashed=True)
        for line in box:
            draw_box(draw, to_px(line.bbox), PDFMINER_COLOR)
            pm_line_count += 1

    chars = cp.extract_page_chars(page)
    lt_page = laytext.analyze_page(chars, params)
    lt_line_count = 0
    for block in lt_page.blocks:
        if args.blocks:
            draw_box(
                draw, to_px((block.bbox.x0, block.bbox.y0, block.bbox.x1, block.bbox.y1)), LAYTEXT_COLOR, dashed=True
            )
        for line in block.lines:
            draw_box(draw, to_px((line.bbox.x0, line.bbox.y0, line.bbox.x1, line.bbox.y1)), LAYTEXT_COLOR)
            lt_line_count += 1

    draw_legend(draw, args.pdf.name, args.page, pm_line_count, lt_line_count)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    image.save(args.out)
    print(f'wrote {args.out} ({image.width}x{image.height}px, pdfminer={pm_line_count}, laytext={lt_line_count} lines)')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
