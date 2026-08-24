use crate::blocks::group_blocks;
use crate::params::Params;
use crate::segmentation::{Region, segment};
use crate::types::{Block, Line, Page};

pub fn assemble_region(region: Region, params: &Params) -> Vec<Block> {
    match region {
        Region::Leaf { lines, .. } => {
            let mut blocks = group_blocks(lines, params);
            blocks.sort_by(|a, b| {
                b.bbox
                    .y1
                    .total_cmp(&a.bbox.y1)
                    .then(a.bbox.x0.total_cmp(&b.bbox.x0))
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
/// left-to-right. Each block's `reading_order` is assigned sequentially
/// over that flattened order.
pub fn assemble(lines: Vec<Line>, params: &Params, width: f64, height: f64) -> Page {
    let region = segment(lines, params);
    let blocks = assemble_region(region, params)
        .into_iter()
        .enumerate()
        .map(|(reading_order, block)| Block {
            reading_order,
            ..block
        })
        .collect();
    Page {
        width,
        height,
        blocks,
    }
}
