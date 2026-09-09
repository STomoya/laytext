use _core::geometry::Rect;
use _core::reading_order::order_blocks;
use _core::types::{Block, Char, Line};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn block(bbox: Rect, upright: bool) -> Block {
    Block {
        bbox,
        reading_order: 0,
        lines: vec![Line {
            bbox,
            upright,
            chars: vec![Char {
                bbox,
                text: 'x',
                font: None,
            }],
        }],
    }
}

#[test]
fn empty_input_returns_empty() {
    assert_eq!(order_blocks(vec![], 0.5), vec![]);
}

#[test]
fn single_block_returns_unchanged() {
    let b = block(rect(0.0, 0.0, 10.0, 10.0), true);
    assert_eq!(order_blocks(vec![b.clone()], 0.5), vec![b]);
}

#[test]
fn stacked_pair_orders_top_before_bottom() {
    let top = block(rect(0.0, 50.0, 10.0, 60.0), true);
    let bottom = block(rect(0.0, 0.0, 10.0, 10.0), true);
    let ordered = order_blocks(vec![bottom.clone(), top.clone()], 0.5);
    assert_eq!(ordered, vec![top, bottom]);
}

#[test]
fn two_columns_stay_in_left_then_right_order_without_any_region_tree() {
    let col_a = block(rect(0.0, 0.0, 90.0, 100.0), true);
    let col_b = block(rect(110.0, 0.0, 200.0, 100.0), true);
    let ordered = order_blocks(vec![col_b.clone(), col_a.clone()], 0.5);
    assert_eq!(ordered, vec![col_a, col_b]);
}

#[test]
fn a_vertical_block_forces_the_merge_to_top_right_to_bottom_left_flow() {
    let horizontal = block(rect(0.0, 0.0, 10.0, 10.0), true);
    let vertical = block(rect(20.0, 0.0, 30.0, 10.0), false);
    let ordered = order_blocks(vec![horizontal.clone(), vertical.clone()], 0.5);
    // TbRl sort key favors larger x, smaller y1: `vertical` (x0=20) sorts
    // before `horizontal` (x0=0).
    assert_eq!(ordered, vec![vertical, horizontal]);
}

#[test]
fn an_intervening_block_defers_the_closer_pair_but_still_merges_all_three() {
    let a = block(rect(0.0, 0.0, 10.0, 10.0), true);
    // Sits squarely inside union(a, c) with no relation of its own to
    // either: pairing it with `a` or `c` costs far more (area 5400) than
    // the raw `a`-`c` gap (area 100), so `a`-`c` is the pair the merge
    // loop tries first — and must defer via `isany`, since `o` intersects
    // their union bbox — before the algorithm can make progress.
    let o = block(rect(11.0, 0.0, 19.0, 500.0), true);
    let c = block(rect(20.0, 0.0, 30.0, 10.0), true);

    let ordered = order_blocks(vec![a.clone(), o.clone(), c.clone()], 0.5);

    assert_eq!(ordered, vec![o, a, c]);
}
