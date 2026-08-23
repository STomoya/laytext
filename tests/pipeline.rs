use _core::blocks::{group_blocks, group_blocks_in_region};
use _core::geometry::Rect;
use _core::params::Params;
use _core::segmentation::segment;
use _core::types::{Char, Line};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn line(bbox: Rect) -> Line {
    Line {
        bbox,
        upright: true,
        chars: vec![Char {
            bbox,
            text: 'x',
            font: None,
        }],
    }
}

// A full-width title sits close above (and left-aligned with) a two-column
// body. Without region scoping, the title's wide bbox horizontally overlaps
// both columns, so a naive line->block merge would bridge the title into
// whichever column it happens to align with.
fn multi_column_page() -> (Line, Line, Line) {
    let title = line(rect(0.0, 100.0, 220.0, 110.0));
    let col_a = line(rect(0.0, 87.0, 100.0, 97.0)); // 3pt below title, left-aligned
    let col_b = line(rect(120.0, 87.0, 220.0, 97.0)); // 3pt below title, right-aligned
    (title, col_a, col_b)
}

#[test]
fn naive_line_to_block_merge_bridges_the_gutter_without_region_scoping() {
    let params = Params::default();
    let (title, col_a, col_b) = multi_column_page();
    let blocks = group_blocks(vec![title, col_a, col_b], &params);
    assert_eq!(
        blocks.len(),
        1,
        "demonstrates why block-merging must be scoped per region: the \
         title's full-width bbox aligns with both columns and bridges them"
    );
}

#[test]
fn region_scoped_merge_keeps_the_title_and_both_columns_separate() {
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        ..Default::default()
    };
    let (title, col_a, col_b) = multi_column_page();
    let region = segment(vec![title.clone(), col_a.clone(), col_b.clone()], &params);
    let blocks = group_blocks_in_region(region, &params);

    assert_eq!(blocks.len(), 3, "no cross-gutter or cross-title merge");
    for expected in [vec![title], vec![col_a], vec![col_b]] {
        assert!(
            blocks.iter().any(|b| b.lines == expected),
            "expected a standalone block for {expected:?}, got {blocks:?}"
        );
    }
}
