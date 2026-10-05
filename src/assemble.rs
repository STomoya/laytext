use crate::blocks::group_blocks;
use crate::params::{Params, Strategy};
use crate::reading_order::order_blocks;
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

/// `Strategy::XyCut`: segments `lines` into regions, merges lines into
/// blocks per region, and flattens region-tree order into reading order
/// (see [`assemble_region`]). `Strategy::Pdfminer` (default): merges
/// `lines` into blocks flat, with no region tree, then orders them via
/// [`order_blocks`], a port of pdfminer's own `group_textboxes`
/// clustering. Each block's `reading_order` is assigned sequentially over
/// whichever flattened order results.
pub fn assemble(lines: Vec<Line>, params: &Params, width: f64, height: f64) -> Page {
    let ordered = match params.segmentation {
        Strategy::XyCut => {
            let region = segment(lines, params);
            assemble_region(region, params)
        }
        Strategy::Pdfminer => {
            let blocks = group_blocks(lines, params);
            order_blocks(blocks, params.boxes_flow)
        }
    };
    let blocks = ordered
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
