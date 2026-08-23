use std::collections::HashMap;

use crate::geometry::{Rect, union_all};
use crate::params::Params;
use crate::segmentation::Region;
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
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let neighbors = if lines[i].upright && lines[j].upright {
                are_horizontal_neighbors(&lines[i].bbox, &lines[j].bbox, params)
            } else if !lines[i].upright && !lines[j].upright {
                are_vertical_neighbors(&lines[i].bbox, &lines[j].bbox, params)
            } else {
                false
            };
            if neighbors {
                union(&mut parent, i, j);
            }
        }
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
            let block_lines: Vec<Line> = idxs.iter().map(|&i| owned[i].take().unwrap()).collect();
            let bbox = union_all(block_lines.iter().map(|l| l.bbox));
            Block {
                bbox,
                lines: block_lines,
            }
        })
        .collect()
}

/// Walks a `Region` tree and merges each leaf's lines into blocks
/// independently (`group_blocks`, per region), flattening the results.
/// Since block-merging is always scoped to a single leaf, this is the only
/// entry point that guarantees merges never cross a region boundary.
pub fn group_blocks_in_region(region: Region, params: &Params) -> Vec<Block> {
    match region {
        Region::Leaf { lines, .. } => group_blocks(lines, params),
        Region::Split { children, .. } => children
            .into_iter()
            .flat_map(|child| group_blocks_in_region(child, params))
            .collect(),
    }
}
