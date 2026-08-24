# laytext

A Rust-implemented, PyO3-exposed Python package that performs pdfminer.six-
style line/block layout detection on PDF character data, with multi-column-
aware grouping and a hierarchical `Page → Block → Line → Char` output. It
consumes char bounding boxes produced externally (e.g. by pypdfium2) — it
does not parse PDFs itself. v1 is done when it matches or beats pdfminer.six
in grouping quality on the user's real production corpus, is measurably
faster, and correctly handles multi-column layouts pdfminer's original
algorithm cannot.

## Tech stack

- Language: Rust (core), Python (bindings surface + test harness)
- Python bridge: PyO3, `abi3-py311` (single wheel, Python 3.11+)
- Parallelism: `rayon` (data-parallel across pages)
- Build tool: `maturin` (>= 1.9.4)
- Python env/tooling: `uv` (run, `uvx` for ad-hoc tools), `ruff` (lint/format), `ty` (type checking)
- Platform / deployment target: pip-installable wheel, Linux only (x86_64 + aarch64 glibc, cross-compiled via zig — see `scripts/build.sh`)
- License: Apache 2.0

## Architecture

Per-page pipeline: char boxes → line grouping → region segmentation (X-Y
cut: a forced full-width band split takes priority; otherwise whichever
axis — column or row — has the wider whitespace gap wins, handles
multi-column) → line→block merge (scoped per region, never crosses a
region boundary) → reading-order assembly. The document-level entry point
batches all pages into one call, releases the GIL, and processes pages in
parallel via `rayon` (pages share no state). A lower-level `group_lines`/
`group_lines_document` entry point also exists for callers that only need
line grouping (no segmentation/block-merge/assembly), with the same
per-document parallel-across-pages shape as `analyze_document`.

```
laytext/
  Cargo.toml
  pyproject.toml
  src/
    lib.rs            # #[pymodule] mod with #[pymodule_export] re-exports
    types.rs           # Page, PageInput, Block, Line, Char, FontInfo, Rect
    geometry.rs         # Rect ops, overlap/distance, whitespace-gap projection
    lines.rs            # char -> line grouping (port of pdfminer group_objects)
    segmentation.rs      # recursive X-Y cut -> region tree
    blocks.rs            # line -> block merge (port of pdfminer group_textboxes), per-region
    assemble.rs          # region tree + blocks -> final Page, reading order
    params.rs            # Params struct
  tests/                 # Rust integration tests (cargo test) — owns all
    lines.rs              # algorithm/logic correctness testing
    segmentation.rs
    blocks.rs
    pipeline.rs           # end-to-end multi-stage pipeline tests
  python/
    laytext/
      __init__.py
      _core.pyi            # type stub for the compiled _core extension module
    tests/               # Python tests — bindings surface ONLY: type
      test_bindings.py    # conversions, exception mapping, call shape.
                           # No algorithm/logic assertions here.
  validation/            # cross-implementation validation, not a test
    compare_pdfminer.py   # suite — runs pdfminer.six itself (Python-only)
                           # to compare output on real-data corpus, plus
                           # DPI-tolerance checks and perf benchmark
    visualize_page.py     # renders a page's detected boxes over the PDF
                           # for manual visual inspection
```

## Commands

- Install (dev): `uv run maturin develop --release`
- Build wheel: `./scripts/build.sh`
- Test (Rust — owns all algorithm/logic correctness): `cargo test` (needs
  `LD_LIBRARY_PATH` pointed at the Python lib dir used by `pyo3`, e.g.
  `LD_LIBRARY_PATH=$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')`,
  or the test binary fails to find `libpython3.11.so.1.0` when run directly)
- Test (Python bindings only): `uv run pytest python/tests/`
- Validation (pdfminer comparison + DPI tolerance + perf, real-data corpus,
  release-blocking, not part of the regular test suite):
  `uv run python validation/compare_pdfminer.py`
- Lint / typecheck: `cargo clippy -- -D warnings` and `uvx ty check python/`
- Format: `cargo fmt` and `uvx ruff check --fix python/ && uvx ruff format python/`
- Coverage (Rust, target 100%): `cargo llvm-cov --workspace --fail-under-lines 100`

## Development workflow

This project follows **test-driven development**: for every unit of work
(a stage in §-numbered milestones below, or any bugfix), write the failing
Rust test in `tests/` first, confirm it fails, then implement the minimum
code in `src/` to make it pass. Do not write implementation code ahead of
the test that specifies its behavior. This applies to the Rust side only
(where all algorithm/logic testing lives, per the split below) — the Python
bindings layer and the `validation/` harness are not algorithm code and
aren't subject to this workflow.

## Coding conventions

- Always work on a branch; never commit directly to `main`. Merge to `main`
  only once changes are validated.
- Only use permissively licensed dependencies (MIT, Apache-2.0) unless
  explicitly told otherwise.
- Do not add the `extension-module` Cargo feature — it's deprecated. Rely on
  `PYO3_BUILD_EXTENSION_MODULE`, which `maturin` >= 1.9.4 sets automatically.
- Define the PyO3 module with the declarative `#[pymodule] mod { ... }` +
  `#[pymodule_export]` pattern — not the older imperative
  `fn my_extension(m: &Bound<PyModule>) { m.add_function(...) }` style.
- No grouping threshold may require `FontInfo`/font metrics to be present.
  Every threshold (char/line size, baseline, margins) must have a
  geometry-only fallback derived from char bbox dimensions — the primary
  target input (OCR'd scanned books) has no reliable font data.
- Block-merging (line → block) always runs scoped to a single region;
  region segmentation (the X-Y cut) always runs first, so merges can never
  cross a column boundary.
- `column_gap_min` / `row_gap_min` are `Option<f64>` in `Params`. When
  `None`, `segment()` (`src/segmentation.rs`) derives the threshold per
  region from that region's median line height (geometry-only, no font
  metrics), recomputed at each recursion level so nested regions with
  different text sizes get locally appropriate thresholds. An explicit
  value always overrides the derived one.
- Prefer porting pdfminer's existing algorithm (line grouping,
  distance-based block merge) over designing new heuristics — deviate only
  where multi-column support or the hierarchical output genuinely require
  new logic (region segmentation, reading-order assembly).

## Testing & workflow rules

- **Rust owns all algorithm/logic testing.** `cargo test` (in `tests/` plus
  any inline `#[cfg(test)]` unit tests) must pass for every stage —
  `geometry`, `lines`, `segmentation`, `blocks`, `assemble` — before merging
  to `main`. This is where correctness of the actual grouping logic is
  verified. **Target 100% line coverage on the Rust side**, checked via
  `cargo llvm-cov`; a change that drops coverage should add the missing
  test alongside it rather than being merged with a gap.
- **Test-driven development is required** on the Rust side (see Development
  workflow above): the failing test precedes the implementation, always.
- **Python tests cover only the bindings surface** (`python/tests/`): that
  types convert correctly across the FFI boundary, exceptions map sanely,
  `analyze_document`/`analyze_page` are callable with the expected shapes.
  No algorithm correctness assertions belong here — if a Python test is
  checking whether blocks were grouped correctly, that check belongs in
  `tests/` on the Rust side instead.
- **The pdfminer-comparison and DPI-tolerance harness (`validation/`) is a
  deliberate exception to the above split**: it necessarily runs in Python
  because it invokes pdfminer.six directly (a pure-Python library) to
  generate reference output. It's a validation/benchmarking script, not a
  "Python test" in the sense above — it doesn't test laytext's Python
  bindings, it validates laytext's Rust-side output against a reference
  implementation and against real-data rendering tolerances.
- Validate against the user's real production data corpus, not just
  synthetic fixtures — this is the primary acceptance basis.
- Compare output against pdfminer.six: exact bbox match is not required
  where the multi-column region-segmentation pass causes a deliberate
  behavioral difference; log and review those divergences rather than
  failing on them. Where no column cut is triggered (effectively
  single-column), aim for exact match.
- Rendering-tolerance check (convert bbox points to pixels via
  `points * dpi / 72`, compare `max(|Δx0|,|Δy0|,|Δx1|,|Δy1|)`):
  - 200 DPI (UI visualization): **< 1 pixel** — hard failure if exceeded.
  - 600 DPI (internal OCR processing): soft threshold — log and review,
    don't fail the run.
- Performance benchmark against pdfminer.six on the real-data corpus is
  **release-blocking**: the port must be measurably faster, not just
  functionally equivalent.

## Implementation milestones

1. **M1 – Single-column line grouping**: char → line grouping ported from
   pdfminer; PyO3 binding for a single page; matches pdfminer output on
   single-column real-data samples.
2. **M2 – Multi-column block grouping**: region segmentation (X-Y cut) and
   region-scoped line → block merge; verified no cross-gutter merges on
   multi-column real-data samples.
3. **M3 – Full hierarchical single-page output**: reading-order assembly;
   `analyze_page` returns the complete `Page → Block → Line → Char` tree.
4. **M4 – Multi-page batching**: `analyze_document` entry point, GIL
   released, pages processed in parallel via `rayon`.
5. **M5 – Validation & packaging**: `validation/compare_pdfminer.py`
   (pdfminer comparison, DPI-tolerance checks, performance benchmark against
   real-data corpus, release-blocking), `abi3-py311` wheel built via
   `maturin`.

## Out of scope for v1

- OCR, PDF parsing/content-stream decoding, rendering.
- ML-based layout detection (e.g. DocLayNet-style models).
- Table structure detection.
- Structure-of-arrays (numpy-backed) batched input/output — only add if
  profiling shows FFI marshalling is the actual bottleneck, not upfront.
