use crate::blocks::group_blocks;
use crate::geometry::union_all;
use crate::params::Params;
use crate::segmentation::{Region, segment};
use crate::types::{Block, Line, Page};

fn assemble_region(region: Region, params: &Params) -> Vec<Block> {
    match region {
        Region::Leaf { lines, .. } => {
            let mut blocks = group_blocks(lines, params);
            blocks.sort_by(|a, b| {
                b.bbox
                    .y1
                    .partial_cmp(&a.bbox.y1)
                    .unwrap()
                    .then(a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap())
            });
            blocks
        }
        Region::Split { children, .. } => children
            .into_iter()
            .flat_map(|child| assemble_region(child, params))
            .collect(),
    }
}

/// Segments `lines` into regions, merges lines into blocks per region, and
/// flattens the result into reading order: regions in the order `segment`
/// produced them (left-then-right for a column cut, top-then-bottom for a
/// row/full-width cut), blocks within a region sorted top-to-bottom then
/// left-to-right.
pub fn assemble(lines: Vec<Line>, params: &Params) -> Page {
    let bbox = union_all(lines.iter().map(|l| l.bbox));
    let region = segment(lines, params);
    let blocks = assemble_region(region, params);
    Page { bbox, blocks }
}
