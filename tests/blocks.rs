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
fn side_by_side_centrally_aligned_vertical_lines_merge_into_one_block() {
    let params = Params::default();
    // Same width (10), vertical centers match, but neither edge does.
    let a = line(rect(0.0, 0.0, 10.0, 100.0), false);
    let b = line(rect(12.0, 20.0, 22.0, 80.0), false);
    let blocks = group_blocks(vec![a.clone(), b.clone()], &params);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].lines, vec![a, b]);
}
