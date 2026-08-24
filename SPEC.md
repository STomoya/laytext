# laytext — Layout Detection Specification

## 1. Overview

A standalone Python package, implemented in Rust and exposed via PyO3, that
replicates and improves on pdfminer.six's line/block layout analysis stage.
It consumes character bounding boxes (as produced externally by pypdfium2 or
any other extractor) and produces a hierarchical `Page → Block → Line → Char`
structure, with multi-column-aware grouping.

This package does **not** parse PDFs, decode content streams, or extract
characters — that stays upstream (pypdfium2 in the primary use case). It is
purely a geometric clustering engine.

Callers must supply char bboxes that are the font's advance-width/
ink-inclusive box, not the glyph's tight ink-only bbox — e.g. pypdfium2's
`get_charbox(..., loose=True)`, not the default tight box. Tight boxes leave
small inter-glyph gaps that can be comparable in size to `word_margin`'s
default threshold, causing spurious word-margin space insertion; this was
empirically observed to fragment a 34-line CJK page into 105 lines when fed
tight boxes, resolved completely by switching to `loose=True`.

## 2. Goals / Non-goals

**Goals**
- Match or exceed pdfminer.six's line/block grouping quality.
- Correctly handle multi-column pages (no cross-gutter merging).
- Produce a clean 3-level hierarchy: block → line → char.
- Run fast enough for bulk/batch processing (Rust core, no per-page ML model).
- Ship as a pip-installable package usable from any Python application.

**Non-goals**
- No OCR, no PDF parsing, no rendering.
- No ML-based layout detection (e.g. DocLayNet-style models) — out of scope,
  see prior discussion; can be a separate optional stage later, not part of
  this package.
- No table structure detection.
- Font metadata is best-effort and optional — not load-bearing for any
  grouping decision.

## 3. Pipeline

The public entry point operates on a whole document (all pages) in a single
call — see §8 and §10 for why. Internally, the four stages below still run
independently per page; pages share no state, so document-level batching is
purely a call-boundary and scheduling optimization, not a change to the
per-page algorithm.

Per page:

1. **Char → Line** — group input char boxes into text lines.
2. **Region segmentation** — recursively partition the page into rectangular
   regions via an X-Y cut, using line boxes as the segmentation input. A
   forced horizontal split around a full-width band (title/header/footer)
   takes priority when present; otherwise whichever axis — column or row —
   has the wider candidate whitespace gap at that recursion level wins the
   cut, so a page whose stacked bands are each also multi-column doesn't
   default to splitting into columns first. This is the multi-column-
   handling stage.
3. **Line → Block** — within each leaf region independently, agglomeratively
   merge lines into blocks. Merging never crosses a region boundary.
4. **Reading-order assembly** — walk the region tree in the order `segment`
   produced it (forced full-width bands top-to-bottom; a column split
   left-to-right; a row split top-to-bottom) to assign a `reading_order`
   index to every block.

Stage 1 is essentially the existing pdfminer `group_objects` logic ported
directly. Stages 2 and 4 are new. Stage 3 is pdfminer's `group_textboxes`
logic, unchanged except for being scoped per-region instead of per-page.

## 4. Data model

```rust
pub struct PageInput {
    pub width: f64,
    pub height: f64,
    pub chars: Vec<Char>,     // as supplied by the caller (e.g. pypdfium2)
}

pub struct Page {
    pub width: f64,
    pub height: f64,
    pub blocks: Vec<Block>,   // in reading order
}

pub struct Block {
    pub bbox: Rect,
    pub reading_order: usize,
    pub lines: Vec<Line>,
}

pub struct Line {
    pub bbox: Rect,
    pub upright: bool,        // horizontal vs vertical text
    pub chars: Vec<Char>,
}

pub struct Char {
    pub bbox: Rect,
    pub text: char,
    pub font: Option<FontInfo>,
}

pub struct FontInfo {
    pub name: Option<String>,
    pub size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

pub struct Rect { pub x0: f64, pub y0: f64, pub x1: f64, pub y1: f64 }
```

`FontInfo` is optional at the `Char` level and every downstream field within
it is independently optional, since embedded OCR fonts (the primary target
use case) may supply none, some, or unreliable values. No stage of the
pipeline may require `FontInfo` to be present — see §6.

## 5. Module boundaries

- **`geometry`** — `Rect` type, overlap/distance/containment helpers,
  whitespace-gap projection along an axis. No knowledge of PDF/text
  concepts; pure geometry, reusable across stages.
- **`lines`** — char → line grouping (port of pdfminer `group_objects` /
  line-overlap logic).
- **`segmentation`** — recursive X-Y cut producing a region tree from line
  boxes. Owns the full-width-line special case (title/header/footer
  detection that forces a horizontal cut) and the column-vs-row tie-break
  (whichever axis has the wider whitespace gap wins, see §3).
- **`blocks`** — line → block agglomerative merge (port of pdfminer
  `group_textboxes`), operating on a single region's lines.
- **`assemble`** — walks the region tree + per-region blocks to produce the
  final `Page` with reading order assigned.
- **`params`** — the tunable-parameter struct (§7), analogous to pdfminer's
  `LAParams`.
- **Rust `tests/`** (crate-root integration tests, `cargo test`) — owns all
  algorithm/logic correctness testing for every module above. Python-side
  tests never assert on grouping correctness (§9); that lives here.
- **`validation/compare_pdfminer.py`** (dev-only, not part of the shipped
  package) — the pdfminer-comparison and DPI-rendering-tolerance harness
  described in §9. Necessarily Python, since it invokes pdfminer.six
  directly to produce reference output; it validates laytext's output
  against that reference and against real-data rendering tolerances, and is
  distinct from the Python bindings tests below.
- **`pybind`** — PyO3 bindings using the declarative-module pattern
  (`#[pymodule] mod ... { #[pymodule_export] ... }`) rather than the older
  imperative `m.add_function(...)` style: `Page`/`Block`/`Line`/`Char`/
  `FontInfo` exposed as Python classes (or converted to `dict`/dataclass,
  TBD at implementation time), plus the top-level `analyze_document(pages,
  params)` entry point (§8, §10), all re-exported into the module via
  `#[pymodule_export]`. A single-page `analyze_page` convenience wrapper
  can remain for ad-hoc/interactive use, but it's just `analyze_document`
  with one page — not a separate code path.

## 6. Font-metric independence

Every threshold that pdfminer derives from font size or metrics must have a
geometry-only fallback, since the primary target input has no reliable font
data:

- **Effective char/line size** is computed from char bbox height, not from
  `FontInfo.size`. `FontInfo.size`, when present, may be used as a secondary
  signal but is never required.
- **Baseline / vertical alignment** is computed from char bbox `y0`/`y1`,
  not from font ascent/descent.
- **`char_margin` / `line_margin` / `column_gap_min` / `row_gap_min`** are
  all expressed as absolute distances or as multiples of the geometry-derived
  effective size — never as multiples of a font-reported size.

This guarantees the same code path runs whether or not `FontInfo` is
populated, so the package behaves identically for the scanned-book/OCR case
(`font: None` throughout) and for a normal digitally-authored PDF.

## 7. Parameters

A single params struct, passed into the top-level entry point, mirrors and
extends pdfminer's `LAParams`:

| Parameter | Role |
|---|---|
| `char_margin` | max gap to merge chars into the same line |
| `line_overlap` | min vertical overlap fraction to consider two chars same line |
| `line_margin` | max gap to merge lines into the same block, within a region |
| `word_margin` | inserts inferred word-space chars during line assembly |
| `column_gap_min` | `Option<f64>` — min horizontal whitespace gap width to trigger a column (vertical) cut; `None` auto-derives the threshold from the region's median line height (see §12) |
| `row_gap_min` | `Option<f64>` — min vertical whitespace gap to trigger a horizontal cut; same auto-derivation as above on `None` |
| `full_width_threshold` | fraction of region width a line must span to be treated as a full-width element (forces horizontal cut) |
| `detect_vertical` | enable vertical (top-to-bottom) text handling |

All parameters get sane defaults but are overridable per call, since gutter
width and column layout vary a lot across real-world documents.

## 8. Python interface (sketch)

```python
from laytext import analyze_document, Params

params = Params(column_gap_min=8.0)

# one call for the whole document
pages = analyze_document(
    [PageInput(chars=page_chars, width=w, height=h) for ...],
    params=params,
)

for page in pages:
    for block in page.blocks:
        for line in block.lines:
            for ch in line.chars:
                ...
```

Each page's `chars` is a list of simple records: bbox + text + optional font
fields — matching whatever pypdfium2 (or another extractor) provides. The
caller assembles all pages up front (streaming/chunked batches are also
fine — see §10 — but the API shape is document-level, not per-page).

A lower-level `group_lines(chars, params) -> list[Line]` and its
document-batched counterpart `group_lines_document(pages, params) ->
list[list[Line]]` are also exposed, for callers that only need stage 1
(char → line grouping) without region segmentation, block-merge, or
reading-order assembly. `group_lines_document` parallelizes across pages
the same way `analyze_document` does.

## 9. Validation strategy

- **Test ownership split**: Rust (`tests/`, `cargo test`) owns all
  algorithm/logic correctness testing across every stage in §3/§5, with a
  **100% line-coverage target** (`cargo llvm-cov`) and **test-driven
  development** (failing test precedes implementation for every unit of
  work on the Rust side). Python tests
  (`python/tests/`) cover only the PyO3 bindings surface — type conversion,
  exception mapping, call shape — never grouping correctness, and TDD is
  not enforced there or in `validation/`, since neither is algorithm code.
  The one deliberate exception is the pdfminer-comparison/DPI-tolerance
  harness below, which must run in Python because it depends on
  pdfminer.six itself; it lives in its own `validation/` location, separate
  from both the Rust tests and the Python bindings tests, since it isn't
  testing laytext's Python bindings — it's validating laytext's output.
- **Regression corpus**: pdfminer.six's own layout test fixtures, to confirm
  single-column output parity (block/line boundaries) before adding
  multi-column logic.
- **Multi-column corpus**: a self-assembled set of 2- and 3-column PDFs
  (academic papers, newspapers) to validate no cross-gutter merges and
  correct reading order, including full-width headers/titles interrupting
  a column layout.
- **OCR/scanned-book corpus**: representative pages from the target
  application (uniform font, unreliable size, no bold/italic) to confirm
  geometry-only thresholds hold up without font signal.
- **Real-data corpus**: the user's own actual production input data (real
  documents fed to the system today) is the primary validation set, not
  just synthetic/public fixtures — this is what final acceptance is judged
  against, in addition to the corpora above.
- **Output comparison against pdfminer**: for each document in the real-data
  corpus, run both pdfminer.six (single-column-equivalent settings) and this
  package, and compare resulting block/line bboxes. Exact bit-for-bit
  parity is a stretch goal, not a hard requirement — the multi-column
  region-segmentation pass (§3) is a deliberate behavioral change from
  pdfminer's flat grouping and may legitimately produce different
  boundaries on multi-column pages. Where pdfminer and this package should
  agree (effectively single-column layout, no column cut triggered), aim
  for exact match; where they diverge because of the column-segmentation
  pass, the check should measure and log the divergence for review rather
  than fail outright.
- **Rendering-tolerance comparison**: since bit-exact geometric parity may
  not always be achievable or even correct (given the multi-column
  behavior change), compare pdfminer vs. this package's output *as
  rendered*, at two DPIs matching real downstream usage:
  - **200 DPI** (used for on-screen PDF visualization in the UI): maximum
    allowed deviation between the two outputs, converted to pixels at this
    resolution, is **< 1 pixel** on any bbox edge — i.e.
    `max(|Δx0|, |Δy0|, |Δx1|, |Δy1|)` per compared bbox, not an aggregate
    metric like IoU. This is a hard pass/fail threshold.
  - **600 DPI** (used internally for OCR processing on the scanned images):
    a **soft** threshold — deviations are logged and reviewed rather than
    failing the run outright, since this resolution is for internal
    processing accuracy rather than a hard visual-fidelity requirement.
  - Both checks convert PDF-point bbox coordinates to pixels at the target
    DPI (`points * dpi / 72`) before comparing, so the tolerance is
    expressed in the same units as what a person or the OCR stage actually
    sees, rather than as a fixed point tolerance that means different
    things at different DPIs.
- **Performance benchmark**: the port must be measurably faster than
  pdfminer.six on the real-data corpus (wall-clock, single-page and
  full-document/batched — see §10), not just functionally equivalent. This
  is a release-blocking check, not just informational.

## 10. Multi-page / performance

**Why a single per-page call doesn't scale**: each `analyze_page` call
crosses the Python/Rust FFI boundary and (by default) holds the GIL for the
duration. For a large PDF (hundreds of pages), that's hundreds of round
trips, and — more importantly — no opportunity to parallelize, since each
call blocks on the GIL individually even though pages have zero data
dependency on each other.

**Design**:

- `analyze_document(pages, params)` takes the whole document's char data in
  one call and returns `Vec<Page>` (§8). Internally it releases the GIL for
  the duration of processing (`py.allow_threads`) and uses data parallelism
  across pages (`rayon`'s `par_iter`), since the four-stage pipeline (§3) has
  no cross-page state. This turns page count into a wall-clock win rather
  than a per-call overhead multiplier.
- Marshalling the input itself becomes the next bottleneck at scale: a
  Python list of per-char dict/object records means one Python object
  allocation + PyO3 conversion per char, times every char in the document.
  For large PDFs this can dominate over the actual clustering work. Two
  input shapes to support:
  - **Simple (default)**: list of per-page lists of char records (bbox +
    text + optional font fields), as in §8 — fine for moderate documents
    and for ergonomics.
  - **Batched/SoA (opt-in, for large PDFs)**: flat arrays per page —
    parallel `x0`/`y0`/`x1`/`y1` numpy arrays plus a joined text buffer and
    per-char offsets — so the FFI crossing is a handful of contiguous buffer
    copies instead of N Python object conversions. `pypdfium2` extraction
    can feasibly emit this shape directly if it becomes the bottleneck.
  - Both shapes feed the same internal per-page pipeline; this is purely an
    input-marshalling optimization, not an algorithm change.
- Output reconstruction (`Vec<Page>` → Python objects) has the same
  per-object cost in reverse. If profiling shows this dominates, the same
  SoA idea applies to output (flat bbox arrays + a `dict`/dataclass "view"
  layer), but start with plain nested Python objects (§4) and only add this
  if it proves necessary — premature flattening makes the API harder to use
  for no benefit if it's not actually the bottleneck.

## 11. Build & packaging

- **Target ABI**: build against `abi3-py311` — the `pyo3` dependency enables
  the `abi3` feature plus the `abi3-py311` companion feature, which fixes
  the minimum supported Python version at 3.11 and produces a single wheel
  per OS/arch importable on 3.11 and all later CPython versions, instead of
  one wheel per Python minor version.
- **Do not use the `extension-module` Cargo feature.** It's deprecated —
  historically it disabled linking to `libpython` for manylinux compliance,
  but it also disabled linking for `cargo test`/benchmarks, which broke
  in-repo testing. It's superseded by the `PYO3_BUILD_EXTENSION_MODULE`
  environment variable, which `maturin` (>= 1.9.4) sets automatically during
  wheel builds while leaving `cargo test` unaffected. Practically: don't add
  `extension-module` to `Cargo.toml`'s `pyo3` features list; just build with
  a current `maturin` and this is handled for you.
- **Module definition**: use the declarative `#[pymodule] mod` form with
  `#[pymodule_export]` (§5) rather than the older imperative
  `fn my_extension(m: &Bound<PyModule>) -> PyResult<()> { m.add_function(...) }`
  style — it keeps the list of exported classes/functions declarative and
  colocated with their definitions.
- **Build tool**: `maturin`, consistent with the abi3/extension-module notes
  above.

## 12. Decisions

- **Block-merge criterion**: port pdfminer's greedy nearest-neighbor
  agglomerative merge (priority-queue over pairwise box distance) as-is,
  scoped per-region (§3). The column-safety concern that motivates an
  overlap-based alternative (e.g. PyMuPDF4LLM's style) is already handled
  by region segmentation happening *before* block-merging runs, so a
  distance-based merge cannot bridge a column gutter regardless. Revisit
  only if real-data validation (§9) surfaces merge-quality issues (e.g.
  paragraph/indentation handling) that region-scoping alone doesn't fix.
- **`column_gap_min` / `row_gap_min` tuning**: `Option<f64>` in `Params`
  (§7). An explicit value always overrides auto-detection. On `None`,
  `segment()` (`src/segmentation.rs`) derives the threshold from the
  current region's median line height — geometry-only, no font metrics —
  multiplied by a fixed factor per axis (2.0× for columns, 1.5× for rows),
  recomputed at each recursion level so nested regions with different text
  sizes get locally appropriate thresholds. These factors approximate the
  corpus-tuned fixed defaults (20pt/15pt against ~10pt body text) used in
  `validation/`; revisit them if real-data validation (§9) shows them
  failing across documents with materially different gutter widths.

## 13. Open questions

(none remaining — see §12 for the resolutions of the two questions
previously listed here)