use _core::blocks::group_blocks;
use _core::geometry::Rect;
use _core::params::Params;
use _core::types::{Char, Line};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn line(bbox: Rect, upright: bool) -> Line {
    Line {
        bbox,
        upright,
        chars: vec![Char {
            bbox,
            text: 'x',
            font: None,
        }],
        confidence: 1.0,
    }
}

#[test]
fn empty_input_produces_no_blocks() {
    let params = Params::default();
    assert_eq!(group_blocks(vec![], &params), vec![]);
}

#[test]
fn single_line_produces_single_block() {
    let params = Params::default();
    let a = line(rect(0.0, 0.0, 100.0, 10.0), true);
    let blocks = group_blocks(vec![a.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].bbox, a.bbox);
    assert_eq!(blocks[0].lines, vec![a]);
}

#[test]
fn stacked_left_aligned_lines_merge_into_one_block() {
    let params = Params::default();
    // Same height (10), left-aligned (x0=0), 2pt vertical gap: well within
    // line_margin(0.5)*height(10)=5.
    let a = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let b = line(rect(0.0, 8.0, 100.0, 18.0), true);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].bbox, rect(0.0, 8.0, 100.0, 30.0));
    assert_eq!(blocks[0].lines, vec![a, b]);
}

#[test]
fn far_apart_lines_stay_separate_blocks() {
    let params = Params::default();
    // 200pt vertical gap: exceeds line_margin(0.5)*height(10)=5.
    let a = line(rect(0.0, 220.0, 100.0, 230.0), true);
    let b = line(rect(0.0, 8.0, 100.0, 18.0), true);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].lines, vec![a]);
    assert_eq!(blocks[1].lines, vec![b]);
}

#[test]
fn misaligned_close_lines_stay_separate_blocks() {
    let params = Params::default();
    // Close vertically but neither left-, right-, nor center-aligned.
    let a = line(rect(0.0, 20.0, 20.0, 30.0), true);
    let b = line(rect(80.0, 8.0, 100.0, 18.0), true);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].lines, vec![a]);
    assert_eq!(blocks[1].lines, vec![b]);
}

#[test]
fn right_aligned_close_lines_merge_into_one_block() {
    let params = Params::default();
    let a = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let b = line(rect(40.0, 8.0, 100.0, 18.0), true);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].lines, vec![a, b]);
}

#[test]
fn centrally_aligned_close_lines_merge_into_one_block() {
    let params = Params::default();
    let a = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let b = line(rect(20.0, 8.0, 80.0, 18.0), true);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].lines, vec![a, b]);
}

#[test]
fn different_height_lines_stay_separate_blocks() {
    let params = Params::default();
    // Left-aligned and close, but height differs by 30 which exceeds
    // line_margin(0.5)*height(10)=5.
    let a = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let b = line(rect(0.0, -20.0, 100.0, 20.0), true);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 2);
}

#[test]
fn transitively_chained_lines_merge_into_one_block() {
    let params = Params::default();
    let a = line(rect(0.0, 40.0, 100.0, 50.0), true);
    let b = line(rect(0.0, 28.0, 100.0, 38.0), true);
    let c = line(rect(0.0, 16.0, 100.0, 26.0), true);
    let blocks = group_blocks(vec![a.clone(), b.clone(), c.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].lines, vec![a, b, c]);
}

#[test]
fn upright_and_vertical_lines_never_merge() {
    let params = Params::default();
    let a = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let b = line(rect(0.0, 8.0, 100.0, 18.0), false);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].lines, vec![a]);
    assert_eq!(blocks[1].lines, vec![b]);
}

#[test]
fn side_by_side_lower_aligned_vertical_lines_merge_into_one_block() {
    let params = Params::default();
    // Same width (10), lower-aligned (y0=0), 2pt horizontal gap: well
    // within line_margin(0.5)*width(10)=5.
    let a = line(rect(0.0, 0.0, 10.0, 100.0), false);
    let b = line(rect(12.0, 0.0, 22.0, 100.0), false);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].bbox, rect(0.0, 0.0, 22.0, 100.0));
    assert_eq!(blocks[0].lines, vec![a, b]);
}

#[test]
fn side_by_side_upper_aligned_vertical_lines_merge_into_one_block() {
    let params = Params::default();
    // Same width (10), upper-aligned (y1=100), different y0: only the
    // upper-edge check (not lower or center) is within tolerance
    // (d = line_margin(0.5) * width(10) = 5).
    let a = line(rect(0.0, 0.0, 10.0, 100.0), false);
    let b = line(rect(12.0, 20.0, 22.0, 100.0), false);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].lines, vec![a, b]);
}

#[test]
fn lines_within_a_block_are_sorted_top_to_bottom_regardless_of_input_order() {
    let params = Params::default();
    let top = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let bottom = line(rect(0.0, 8.0, 100.0, 18.0), true);
    // Passed in bottom-to-top order to prove group_blocks sorts by
    // position, not by input order.
    let blocks = group_blocks(vec![bottom.clone(), top.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].lines, vec![top, bottom]);
}

#[test]
fn side_by_side_centrally_aligned_vertical_lines_merge_into_one_block() {
    let params = Params::default();
    // Same width (10), vertical centers match, but neither edge does.
    let a = line(rect(0.0, 0.0, 10.0, 100.0), false);
    let b = line(rect(12.0, 20.0, 22.0, 80.0), false);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].lines, vec![a, b]);
}

#[test]
fn single_line_block_has_confidence_one() {
    let params = Params::default();
    let a = line(rect(0.0, 0.0, 100.0, 10.0), true);
    let blocks = group_blocks(vec![a], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].confidence, 1.0);
}

#[test]
fn block_confidence_is_high_for_closely_spaced_lines() {
    let params = Params::default();
    // threshold d = line_margin(0.5) * height(10) = 5; vdistance = 0.5:
    // ratio = 1.0 - 0.5/5.0 = 0.9.
    let a = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let b = line(rect(0.0, 9.5, 100.0, 19.5), true);
    let blocks = group_blocks(vec![a, b], &params);
    assert_eq!(blocks.len(), 1);
    assert!((blocks[0].confidence - 0.9).abs() < 1e-9);
}

#[test]
fn block_confidence_is_low_when_lines_barely_clear_line_margin() {
    let params = Params::default();
    // threshold d = line_margin(0.5) * height(10) = 5; vdistance = 4.9:
    // ratio = 1.0 - 4.9/5.0 = 0.02.
    let a = line(rect(0.0, 20.0, 100.0, 30.0), true);
    let b = line(rect(0.0, 5.1, 100.0, 15.1), true);
    let blocks = group_blocks(vec![a, b], &params);
    assert_eq!(blocks.len(), 1);
    assert!((blocks[0].confidence - 0.02).abs() < 1e-9);
}

#[test]
fn block_confidence_reflects_the_weakest_merge_in_a_multi_line_block() {
    let params = Params::default();
    // a-b vdistance 0.5 (ratio 0.9); b-c vdistance 4.9 (ratio 0.02). The
    // block's overall confidence must be the minimum across all its
    // merges, not an average.
    let a = line(rect(0.0, 40.0, 100.0, 50.0), true);
    let b = line(rect(0.0, 29.5, 100.0, 39.5), true);
    let c = line(rect(0.0, 14.6, 100.0, 24.6), true);
    let blocks = group_blocks(vec![a, b, c], &params);
    assert_eq!(blocks.len(), 1);
    assert!((blocks[0].confidence - 0.02).abs() < 1e-9);
}

#[test]
fn tabular_block_with_a_repeated_internal_edge_is_flagged_tabular() {
    let params = Params::default();
    // A full-width top row plus two narrower rows that both start/end at
    // the same interior x (20, 80) — not the block's own outer x0/x1 (0,
    // 100), which only the top row touches. This repeated interior
    // boundary (a shared cell-column edge) is the tabular signal; the top
    // row alone shares nothing internal.
    let top = line(rect(0.0, 40.0, 100.0, 50.0), true);
    let mid = line(rect(20.0, 28.0, 80.0, 38.0), true);
    let bottom = line(rect(20.0, 16.0, 80.0, 26.0), true);
    let blocks = group_blocks(vec![top, mid, bottom], &params);
    assert_eq!(blocks.len(), 1);
    assert!(blocks[0].tabular);
}

#[test]
fn ordinary_ragged_right_paragraph_is_not_tabular() {
    let params = Params::default();
    // All three lines share only the block's own left margin (x0 = 0);
    // their right edges (100, 70, 90) are all different, so no interior
    // edge is ever repeated.
    let a = line(rect(0.0, 40.0, 100.0, 50.0), true);
    let b = line(rect(0.0, 28.0, 70.0, 38.0), true);
    let c = line(rect(0.0, 16.0, 90.0, 26.0), true);
    let blocks = group_blocks(vec![a, b, c], &params);
    assert_eq!(blocks.len(), 1);
    assert!(!blocks[0].tabular);
}

#[test]
fn a_two_line_block_is_never_tabular_regardless_of_alignment() {
    let params = Params::default();
    // Two non-upright lines touching at x=20 (a.x0 = b.x1 = 20), merged via
    // are_vertical_neighbors (same y-range, touching in x, equal width).
    // bbox = (0,0,40,10): x=20 is internal (not bbox's own 0 or 40) and
    // shared by both lines — without the lines.len()<3 guard, this WOULD
    // evaluate tabular=true (aligned_count=2, 2*2=4>2). The guard is what
    // forces false here.
    let a = line(rect(20.0, 0.0, 40.0, 10.0), false);
    let b = line(rect(0.0, 0.0, 20.0, 10.0), false);
    let blocks = group_blocks(vec![a, b], &params);
    assert_eq!(blocks.len(), 1);
    assert!(!blocks[0].tabular);
}
