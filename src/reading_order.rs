use std::collections::HashSet;

use crate::geometry::Rect;

/// pdfminer's `group_textboxes` distance metric: the area of the bounding
/// rect of `a` and `b`, less the area each already covers. Negative when
/// `a` and `b` overlap.
fn dist(a: &Rect, b: &Rect) -> f64 {
    let u = a.union(b);
    u.width() * u.height() - a.width() * a.height() - b.width() * b.height()
}

/// Does any other still-`active` node's bbox intersect the union of `a`
/// and `b`? A plain linear scan (ponytail: O(n) per check, not pdfminer's
/// spatial `Plane` grid index — upgrade if profiling shows this dominates
/// for pages with many blocks, same tradeoff `blocks.rs`'s own neighbor
/// search already accepts).
fn isany(a: usize, b: usize, active: &HashSet<usize>, bboxes: &[Rect]) -> bool {
    let union = bboxes[a].union(&bboxes[b]);
    active.iter().any(|&id| {
        id != a && id != b && bboxes[id].is_hoverlap(&union) && bboxes[id].is_voverlap(&union)
    })
}

/// pdfminer's reading-order sort key for one child of a merged group.
/// `vertical` is the *parent* group's flow (`TBRL` when `true`, `LRTB`
/// when `false`); ascending sort by this key gives reading order.
fn sort_key(vertical: bool, bbox: &Rect, boxes_flow: f64) -> f64 {
    if vertical {
        -(1.0 + boxes_flow) * (bbox.x0 + bbox.x1) - (1.0 - boxes_flow) * bbox.y1
    } else {
        (1.0 - boxes_flow) * bbox.x0 - (1.0 + boxes_flow) * (bbox.y0 + bbox.y1)
    }
}

#[cfg(test)]
mod tests {
    use super::{dist, isany, sort_key};
    use crate::geometry::Rect;
    use std::collections::HashSet;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    #[test]
    fn dist_of_disjoint_boxes_is_the_bounding_gap_area() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(20.0, 0.0, 30.0, 10.0);
        assert_eq!(dist(&a, &b), 100.0);
    }

    #[test]
    fn dist_of_overlapping_boxes_can_be_negative() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(5.0, 0.0, 15.0, 10.0);
        assert_eq!(dist(&a, &b), -50.0);
    }

    #[test]
    fn isany_true_when_a_third_box_sits_inside_the_union() {
        let bboxes = vec![
            rect(0.0, 0.0, 10.0, 10.0),
            rect(40.0, 0.0, 50.0, 10.0),
            rect(20.0, 0.0, 30.0, 10.0),
        ];
        let active: HashSet<usize> = [0, 1, 2].into_iter().collect();
        assert!(isany(0, 1, &active, &bboxes));
    }

    #[test]
    fn isany_false_when_nothing_sits_between_the_pair() {
        let bboxes = vec![rect(0.0, 0.0, 10.0, 10.0), rect(40.0, 0.0, 50.0, 10.0)];
        let active: HashSet<usize> = [0, 1].into_iter().collect();
        assert!(!isany(0, 1, &active, &bboxes));
    }

    #[test]
    fn isany_ignores_boxes_outside_the_active_set() {
        let bboxes = vec![
            rect(0.0, 0.0, 10.0, 10.0),
            rect(40.0, 0.0, 50.0, 10.0),
            rect(20.0, 0.0, 30.0, 10.0),
        ];
        let active: HashSet<usize> = [0, 1].into_iter().collect();
        assert!(!isany(0, 1, &active, &bboxes));
    }

    #[test]
    fn sort_key_horizontal_flow_favors_smaller_x0_and_larger_y() {
        let left_top = rect(0.0, 90.0, 10.0, 100.0);
        let right_bottom = rect(50.0, 0.0, 60.0, 10.0);
        assert!(sort_key(false, &left_top, 0.5) < sort_key(false, &right_bottom, 0.5));
    }

    #[test]
    fn sort_key_vertical_flow_favors_larger_x_and_smaller_y1() {
        let right_top = rect(50.0, 90.0, 60.0, 100.0);
        let left_bottom = rect(0.0, 0.0, 10.0, 10.0);
        assert!(sort_key(true, &right_top, 0.5) < sort_key(true, &left_bottom, 0.5));
    }
}
