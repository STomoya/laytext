# laytext

A Rust-implemented, PyO3-exposed Python package that performs
pdfminer.six-style line/block layout detection on PDF character data, with
multi-column-aware grouping and a hierarchical `Page → Block → Line → Char`
output.

It consumes char bounding boxes produced externally (e.g. by pypdfium2) — it
does not parse PDFs itself.

## Install

```sh
uv add laytext
# or
pip install laytext
```

## Usage

```python
from laytext import analyze_document, Char, PageInput, Params, Rect

# chars come from your own PDF extraction step (e.g. pypdfium2), using
# loose (advance-width) char boxes, not tight ink-only boxes
chars = [Char(bbox=Rect(x0=0, y0=0, x1=6, y1=10), text='a')]

pages = analyze_document(
    [PageInput(chars=chars, width=612, height=792)],
    params=Params(),
)

for page in pages:
    for block in page.blocks:
        for line in block.lines:
            print(''.join(ch.text for ch in line.chars))
```

`Params()` with no arguments defaults to `Strategy.Pdfminer`: reading
order comes from a port of pdfminer's own `group_textboxes` clustering,
tuned via `boxes_flow`. `column_gap_min` / `row_gap_min` /
`full_width_threshold` only apply under the opt-in `Strategy.XyCut`. See
`SPEC.md` for the full design and `AGENTS.md` for repo/dev conventions.

For pages with a PDF `/Rotate`, pass chars, `width` and `height` in
unrotated user space (as pypdfium2's `get_charbox`/`get_mediabox` return
them) plus `rotation=page.get_rotation()` (any multiple of 90). Layout runs
on the upright page; every output bbox stays in that unrotated space.

## License

Apache 2.0 — see `LICENSE`.
