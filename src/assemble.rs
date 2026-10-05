use crate::blocks::group_blocks;
use crate::params::{Params, Strategy};
use crate::reading_order::order_blocks;
use crate::segmentation::{Region, segment};
use crate::skew::estimate_page_skew;
use crate::types::{Block, Char, Line, Page};

pub fn assemble_region(region: Region, params: &Params, skew_degrees: f64) -> Vec<Block> {
    match region {
        Region::Leaf { lines, .. } => {
            let mut blocks = group_blocks(lines, params, skew_degrees);
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
            .flat_map(|child| assemble_region(child, params, skew_degrees))
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
    // ponytail: clones every char and re-estimates the skew group_lines
    // already computed; thread it through from group_lines if profiling
    // shows this pass matters.
    let skew = if params.deskew {
        let chars: Vec<Char> = lines.iter().flat_map(|l| l.chars.clone()).collect();
        estimate_page_skew(&chars)
    } else {
        0.0
    };
    let ordered = match params.segmentation {
        Strategy::XyCut => {
            let region = segment(lines, params);
            assemble_region(region, params, skew)
        }
        Strategy::Pdfminer => {
            let blocks = group_blocks(lines, params, skew);
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
