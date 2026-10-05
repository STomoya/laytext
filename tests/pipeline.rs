use _core::assemble::{assemble, assemble_region};
use _core::blocks::group_blocks;
use _core::geometry::Rect;
use _core::lines::group_lines;
use _core::params::{Params, Strategy};
use _core::segmentation::segment;
use _core::types::{Char, Line};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn line(bbox: Rect) -> Line {
    Line {
        bbox,
        upright: true,
        confidence: 1.0,
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
    let blocks = group_blocks(vec![title, col_a, col_b], &params, 0.0);
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
    let blocks = assemble_region(region, &params, 0.0);

    assert_eq!(blocks.len(), 3, "no cross-gutter or cross-title merge");
    for expected in [vec![title], vec![col_a], vec![col_b]] {
        assert!(
            blocks.iter().any(|b| b.lines == expected),
            "expected a standalone block for {expected:?}, got {blocks:?}"
        );
    }
}

#[test]
fn assemble_orders_blocks_top_to_bottom_within_a_single_region() {
    let params = Params {
        column_gap_min: Some(1.0),
        row_gap_min: Some(50.0),
        segmentation: Strategy::XyCut,
        ..Default::default()
    };
    let top = line(rect(0.0, 50.0, 6.0, 60.0));
    let bottom = line(rect(0.0, 0.0, 6.0, 10.0));

    // Passed in bottom-to-top order to prove assemble sorts by position,
    // not by input order.
    let page = assemble(vec![bottom.clone(), top.clone()], &params, 6.0, 60.0);

    assert_eq!(page.blocks.len(), 2);
    assert_eq!(page.blocks[0].lines, vec![top]);
    assert_eq!(page.blocks[1].lines, vec![bottom]);
}

#[test]
fn assemble_orders_full_width_title_before_left_column_before_right_column() {
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        segmentation: Strategy::XyCut,
        ..Default::default()
    };
    let (title, col_a, col_b) = multi_column_page();

    // Shuffled input order to prove assemble sorts by reading order, not
    // by input order.
    let page = assemble(
        vec![col_b.clone(), title.clone(), col_a.clone()],
        &params,
        220.0,
        110.0,
    );

    assert_eq!(page.blocks.len(), 3);
    assert_eq!(page.blocks[0].lines, vec![title]);
    assert_eq!(page.blocks[1].lines, vec![col_a]);
    assert_eq!(page.blocks[2].lines, vec![col_b]);
}

#[test]
fn assemble_defaults_to_pdfminer_strategy_flat_clustering_no_region_tree() {
    let col_a = line(rect(0.0, 0.0, 90.0, 100.0));
    let col_b = line(rect(110.0, 0.0, 200.0, 100.0));

    let page = assemble(
        vec![col_b.clone(), col_a.clone()],
        &Params::default(),
        200.0,
        100.0,
    );

    assert_eq!(page.blocks.len(), 2);
    assert_eq!(page.blocks[0].lines, vec![col_a]);
    assert_eq!(page.blocks[1].lines, vec![col_b]);
}

#[test]
fn assemble_default_pdfminer_strategy_bridges_the_gutter_like_flat_block_merge_alone() {
    let (title, col_a, col_b) = multi_column_page();

    let page = assemble(
        vec![title.clone(), col_a.clone(), col_b.clone()],
        &Params::default(),
        220.0,
        110.0,
    );

    assert_eq!(
        page.blocks.len(),
        1,
        "default Pdfminer strategy runs flat block-merge with no region \
         scoping; Strategy::XyCut is required to keep the title and both \
         columns separate (see \
         assemble_orders_full_width_title_before_left_column_before_right_column)"
    );
}

/// Three 2deg-tilted dense lines (40 6x10 chars at a 7pt pitch, ~9.7pt
/// rise across each) with a 20pt baseline step. Deskewed, each line is
/// 10pt tall with a 10pt gap, over line_margin * height (5pt): three
/// separate paragraphs. The raw axis-aligned line bboxes are ~19.7pt tall,
/// shrinking the gap to ~0.3pt and inflating the threshold to ~9.9pt, so
/// merging on raw geometry collapses them into one block.
fn skewed_paragraphs() -> Vec<Char> {
    let slope = 2.0_f64.to_radians().tan();
    [540.0, 520.0, 500.0]
        .into_iter()
        .flat_map(|baseline| {
            (0..40).map(move |i| {
                let x0 = (i as f64) * 7.0;
                let y0 = baseline + (x0 + 3.0) * slope;
                Char {
                    bbox: rect(x0, y0, x0 + 6.0, y0 + 10.0),
                    text: 'x',
                    font: None,
                }
            })
        })
        .collect()
}

#[test]
fn skewed_paragraph_gap_keeps_lines_in_separate_blocks() {
    let params = Params {
        deskew: true,
        word_margin: 0.0,
        ..Params::default()
    };
    let lines = group_lines(skewed_paragraphs(), &params);
    assert_eq!(lines.len(), 3);
    let raw_bboxes: Vec<Rect> = lines.iter().map(|l| l.bbox).collect();

    let page = assemble(lines, &params, 300.0, 560.0);

    assert_eq!(page.blocks.len(), 3);
    let mut out_bboxes: Vec<Rect> = page.blocks.iter().map(|b| b.bbox).collect();
    out_bboxes.sort_by(|a, b| b.y1.total_cmp(&a.y1));
    assert_eq!(
        out_bboxes, raw_bboxes,
        "output geometry must stay unsheared"
    );
}

#[test]
fn skewed_block_merge_matches_pdfminer_when_deskew_is_off() {
    let params = Params {
        word_margin: 0.0,
        ..Params::default()
    };
    let lines = group_lines(skewed_paragraphs(), &params);
    assert_eq!(lines.len(), 3);

    let page = assemble(lines, &params, 300.0, 560.0);

    assert_eq!(page.blocks.len(), 1);
}

/// Two stacked bands of side-by-side blocks whose column gutters don't
/// line up (so no page-wide column cut exists), rotated 4°: the right side
/// rises enough that the raw y-projections leave no row gap clearing the
/// derived `row_gap_min`, so without shear correction XyCut never splits
/// the bands and orders blocks by raw top edge (right before left).
fn skewed_staggered_bands() -> Vec<Char> {
    let slope = 4.0_f64.to_radians().tan();
    let run = move |baseline: f64, x_start: f64, count: usize| {
        (0..count).map(move |i| {
            let x0 = x_start + (i as f64) * 7.0;
            let y0 = baseline + (x0 + 3.0) * slope;
            Char {
                bbox: rect(x0, y0, x0 + 6.0, y0 + 10.0),
                text: 'x',
                font: None,
            }
        })
    };
    let mut chars = Vec::new();
    for (baselines, blocks) in [
        ([560.0, 545.0], [(0.0, 18), (154.0, 18)]),
        ([515.0, 500.0], [(0.0, 15), (136.0, 20)]),
    ] {
        for (x_start, count) in blocks {
            for b in baselines {
                chars.extend(run(b, x_start, count));
            }
        }
    }
    chars
}

#[test]
fn xycut_deskew_segments_on_shear_corrected_bboxes() {
    let params = Params {
        deskew: true,
        word_margin: 0.0,
        segmentation: Strategy::XyCut,
        ..Params::default()
    };
    let lines = group_lines(skewed_staggered_bands(), &params);
    assert_eq!(lines.len(), 8);
    let mut raw_bboxes: Vec<Rect> = lines.iter().map(|l| l.bbox).collect();

    let page = assemble(lines, &params, 300.0, 600.0);

    let x0s: Vec<f64> = page.blocks.iter().map(|b| b.bbox.x0).collect();
    assert_eq!(x0s, vec![0.0, 154.0, 0.0, 136.0]);
    let mut out_bboxes: Vec<Rect> = page
        .blocks
        .iter()
        .flat_map(|b| b.lines.iter().map(|l| l.bbox))
        .collect();
    let key = |a: &Rect, b: &Rect| a.x0.total_cmp(&b.x0).then(b.y1.total_cmp(&a.y1));
    out_bboxes.sort_by(key);
    raw_bboxes.sort_by(key);
    assert_eq!(
        out_bboxes, raw_bboxes,
        "output geometry must stay unsheared"
    );
}
