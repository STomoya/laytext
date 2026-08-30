use std::collections::HashMap;

use crate::geometry::{Rect, margin_ratio, union_all};
use crate::params::Params;
use crate::types::{Block, Line};

fn find(parent: &mut [usize], x: usize) -> usize {
    if parent[x] != x {
        parent[x] = find(parent, parent[x]);
    }
    parent[x]
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

fn are_horizontal_neighbors(a: &Rect, b: &Rect, params: &Params) -> bool {
    let d = params.line_margin * a.height();
    a.is_hoverlap(b)
        && a.vdistance(b) <= d
        && (a.height() - b.height()).abs() <= d
        && ((a.x0 - b.x0).abs() <= d
            || (a.x1 - b.x1).abs() <= d
            || (((a.x0 + a.x1) - (b.x0 + b.x1)) / 2.0).abs() <= d)
}

fn are_vertical_neighbors(a: &Rect, b: &Rect, params: &Params) -> bool {
    let d = params.line_margin * a.width();
    a.is_voverlap(b)
        && a.hdistance(b) <= d
        && (a.width() - b.width()).abs() <= d
        && ((a.y0 - b.y0).abs() <= d
            || (a.y1 - b.y1).abs() <= d
            || (((a.y0 + a.y1) - (b.y0 + b.y1)) / 2.0).abs() <= d)
}

/// Distance tolerance (points) for treating two lines' x0/x1 edges as "the
/// same" internal boundary when counting tabular alignment.
const TABULAR_ALIGN_TOLERANCE: f64 = 1.0;

/// A block is tabular when a majority of its lines share a common x0 or x1
/// edge that is not simply the block's own outer left/right margin: a
/// repeated *internal* boundary (e.g. a shared table-column edge) is the
/// geometric signature of stacked table rows, as opposed to one ragged-right
/// paragraph, whose lines share only the block's own outer left margin.
/// Fewer than 3 lines is never enough for "repeated" to mean anything.
fn block_is_tabular(lines: &[Line], bbox: &Rect) -> bool {
    if lines.len() < 3 {
        return false;
    }
    let is_internal = |x: f64| {
        (x - bbox.x0).abs() > TABULAR_ALIGN_TOLERANCE
            && (x - bbox.x1).abs() > TABULAR_ALIGN_TOLERANCE
    };
    let has_repeated_internal_edge = |line: &Line| {
        [line.bbox.x0, line.bbox.x1].into_iter().any(|x| {
            is_internal(x)
                && lines
                    .iter()
                    .filter(|other| {
                        (other.bbox.x0 - x).abs() <= TABULAR_ALIGN_TOLERANCE
                            || (other.bbox.x1 - x).abs() <= TABULAR_ALIGN_TOLERANCE
                    })
                    .count()
                    >= 2
        })
    };
    let aligned_count = lines.iter().filter(|l| has_repeated_internal_edge(l)).count();
    aligned_count * 2 > lines.len()
}

/// Merges neighboring lines into blocks, scoped to a single region (a
/// region's lines never merge with another region's). Direct port of
/// pdfminer's `LTLayoutContainer.group_textlines`: for each line, find
/// same-height/same-width and left-/right-/center-aligned neighbors within
/// `line_margin`, then union-find them into connected components. Upright
/// and non-upright lines never merge with each other, mirroring pdfminer's
/// separate `LTTextLineHorizontal`/`LTTextLineVertical` neighbor search.
pub fn group_blocks(lines: Vec<Line>, params: &Params) -> Vec<Block> {
    let n = lines.len();
    if n == 0 {
        return Vec::new();
    }

    // ponytail: O(n^2) neighbor search per region; add a spatial index
    // (like pdfminer's Plane) if profiling shows this dominates for
    // regions with many lines.
    let mut parent: Vec<usize> = (0..n).collect();
    // Every qualifying neighbor pair, with the margin ratio of the distance
    // that made it qualify — collected before roots settle, since union-find
    // path compression during the loop can still move any node's root.
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let a = &lines[i].bbox;
            let b = &lines[j].bbox;
            let neighbors = if lines[i].upright && lines[j].upright {
                are_horizontal_neighbors(a, b, params)
            } else if !lines[i].upright && !lines[j].upright {
                are_vertical_neighbors(a, b, params)
            } else {
                false
            };
            if neighbors {
                let ratio = if lines[i].upright {
                    margin_ratio(a.vdistance(b), params.line_margin * a.height())
                } else {
                    margin_ratio(a.hdistance(b), params.line_margin * a.width())
                };
                edges.push((i, j, ratio));
                union(&mut parent, i, j);
            }
        }
    }

    // Fully compress every path so `parent[i]` is each node's final root.
    for i in 0..n {
        find(&mut parent, i);
    }
    let mut min_confidence: HashMap<usize, f64> = HashMap::new();
    for (i, j, ratio) in edges {
        let root = parent[i];
        debug_assert_eq!(root, parent[j]);
        min_confidence
            .entry(root)
            .and_modify(|c: &mut f64| *c = c.min(ratio))
            .or_insert(ratio);
    }

    let mut order: Vec<usize> = Vec::new();
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups
            .entry(root)
            .or_insert_with(|| {
                order.push(root);
                Vec::new()
            })
            .push(i);
    }

    let mut owned: Vec<Option<Line>> = lines.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|root| {
            let idxs = &groups[&root];
            let mut block_lines: Vec<Line> =
                idxs.iter().map(|&i| owned[i].take().unwrap()).collect();
            block_lines.sort_by(|a, b| {
                b.bbox
                    .y1
                    .total_cmp(&a.bbox.y1)
                    .then(a.bbox.x0.total_cmp(&b.bbox.x0))
            });
            let bbox = union_all(block_lines.iter().map(|l| l.bbox));
            let tabular = block_is_tabular(&block_lines, &bbox);
            Block {
                bbox,
                // Placeholder: `assemble` overwrites this with the final
                // flattened reading-order index once all regions are merged.
                reading_order: 0,
                lines: block_lines,
                tabular,
                confidence: *min_confidence.get(&root).unwrap_or(&1.0),
            }
        })
        .collect()
}
