use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use crate::geometry::Rect;
use crate::types::Block;

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
        id != a
            && id != b
            && bboxes[id].x0 < union.x1
            && bboxes[id].x1 > union.x0
            && bboxes[id].y0 < union.y1
            && bboxes[id].y1 > union.y0
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct HeapEntry {
    skip_isany: bool,
    dist: f64,
    a: usize,
    b: usize,
}

impl Eq for HeapEntry {}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// `BinaryHeap` is a max-heap; this `Ord` is deliberately the reverse of
// pdfminer's own priority (not-yet-deferred pairs before deferred ones,
// regardless of distance; then smallest distance; then smallest ids for
// determinism) so that `.pop()` returns pdfminer's next pair to try.
impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .skip_isany
            .cmp(&self.skip_isany)
            .then_with(|| other.dist.total_cmp(&self.dist))
            .then_with(|| other.a.cmp(&self.a))
            .then_with(|| other.b.cmp(&self.b))
    }
}

struct Tree {
    bboxes: Vec<Rect>,
    vertical: Vec<bool>,
    // Node id `n + k` (`n` = leaf count) is a merged group; its two
    // children are `group_children[k]`.
    group_children: Vec<[usize; 2]>,
}

fn collect_ordered(id: usize, n: usize, tree: &Tree, boxes_flow: f64, out: &mut Vec<usize>) {
    if id < n {
        out.push(id);
        return;
    }
    let mut children = tree.group_children[id - n];
    let parent_vertical = tree.vertical[id];
    children.sort_by(|&x, &y| {
        sort_key(parent_vertical, &tree.bboxes[x], boxes_flow).total_cmp(&sort_key(
            parent_vertical,
            &tree.bboxes[y],
            boxes_flow,
        ))
    });
    for c in children {
        collect_ordered(c, n, tree, boxes_flow, out);
    }
}

/// Direct port of pdfminer's `LTLayoutContainer.group_textboxes`: merges
/// `blocks` via nearest-first hierarchical clustering (by [`dist`]),
/// deferring an obstructed pair via [`isany`] until closer, unobstructed
/// pairs have had their chance, then flattens the resulting tree into
/// reading order via each group's own [`sort_key`] (driven by
/// `boxes_flow`, mirroring `LAParams.boxes_flow`).
pub fn order_blocks(blocks: Vec<Block>, boxes_flow: f64) -> Vec<Block> {
    let n = blocks.len();
    if n <= 1 {
        return blocks;
    }

    let mut bboxes: Vec<Rect> = blocks.iter().map(|b| b.bbox).collect();
    let mut vertical: Vec<bool> = blocks.iter().map(|b| !b.lines[0].upright).collect();
    let mut group_children: Vec<[usize; 2]> = Vec::new();

    let mut active: HashSet<usize> = (0..n).collect();
    let mut done: HashSet<usize> = HashSet::new();

    let mut heap: BinaryHeap<HeapEntry> = BinaryHeap::new();
    for i in 0..n {
        for j in (i + 1)..n {
            heap.push(HeapEntry {
                skip_isany: false,
                dist: dist(&bboxes[i], &bboxes[j]),
                a: i,
                b: j,
            });
        }
    }

    while let Some(entry) = heap.pop() {
        if done.contains(&entry.a) || done.contains(&entry.b) {
            continue;
        }
        if !entry.skip_isany && isany(entry.a, entry.b, &active, &bboxes) {
            heap.push(HeapEntry {
                skip_isany: true,
                ..entry
            });
            continue;
        }

        let new_bbox = bboxes[entry.a].union(&bboxes[entry.b]);
        let new_vertical = vertical[entry.a] || vertical[entry.b];
        let new_id = bboxes.len();
        bboxes.push(new_bbox);
        vertical.push(new_vertical);
        group_children.push([entry.a, entry.b]);

        active.remove(&entry.a);
        active.remove(&entry.b);
        done.insert(entry.a);
        done.insert(entry.b);

        for &other in &active {
            heap.push(HeapEntry {
                skip_isany: false,
                dist: dist(&new_bbox, &bboxes[other]),
                a: new_id.min(other),
                b: new_id.max(other),
            });
        }
        active.insert(new_id);
        if active.len() <= 1 {
            break;
        }
    }

    let tree = Tree {
        bboxes,
        vertical,
        group_children,
    };
    let root = *active.iter().next().unwrap();
    let mut order = Vec::with_capacity(n);
    collect_ordered(root, n, &tree, boxes_flow, &mut order);

    let mut owned: Vec<Option<Block>> = blocks.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|i| owned[i].take().unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{HeapEntry, dist, isany, sort_key};
    use crate::geometry::Rect;
    use std::collections::HashSet;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    #[test]
    fn heap_entry_cmp_breaks_ties_on_smaller_b_when_skip_isany_and_dist_and_a_match() {
        let lower_b = HeapEntry {
            skip_isany: false,
            dist: 1.0,
            a: 0,
            b: 1,
        };
        let higher_b = HeapEntry {
            skip_isany: false,
            dist: 1.0,
            a: 0,
            b: 2,
        };
        assert!(lower_b > higher_b);
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
    fn isany_false_when_a_third_box_only_touches_the_union_boundary() {
        let bboxes = vec![
            rect(0.0, 0.0, 10.0, 10.0),
            rect(20.0, 0.0, 30.0, 10.0),
            rect(10.0, 10.0, 40.0, 20.0),
        ];
        let active: HashSet<usize> = [0, 1, 2].into_iter().collect();
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
