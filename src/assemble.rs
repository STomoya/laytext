use crate::blocks::group_blocks;
use crate::geometry::{Rect, union_all};
use crate::lines::group_lines;
use crate::params::{Params, Strategy};
use crate::reading_order::order_blocks;
use crate::segmentation::{Region, segment};
use crate::skew::{SKEW_NOISE_FLOOR_DEGREES, estimate_page_skew, shear_correct_bboxes};
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

/// Undoes the shear correction `assemble` applies before segmenting: a
/// line's bbox is exactly the union of its chars' bboxes (spaces lie
/// within it), so recomputing from the untouched chars restores it.
fn restore_line_bboxes(region: Region) -> Region {
    match region {
        Region::Leaf { bbox, lines } => Region::Leaf {
            bbox,
            lines: lines
                .into_iter()
                .map(|l| Line {
                    bbox: union_all(l.chars.iter().map(|c| c.bbox)),
                    ..l
                })
                .collect(),
        },
        Region::Split {
            bbox,
            orientation,
            children,
        } => Region::Split {
            bbox,
            orientation,
            children: children.into_iter().map(restore_line_bboxes).collect(),
        },
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
            let region = if skew.abs() > SKEW_NOISE_FLOOR_DEGREES {
                let corrected = lines
                    .into_iter()
                    .map(|l| Line {
                        bbox: union_all(shear_correct_bboxes(&l.chars, skew)),
                        ..l
                    })
                    .collect();
                restore_line_bboxes(segment(corrected, params))
            } else {
                segment(lines, params)
            };
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

fn map_page_bboxes(page: Page, f: impl Fn(Rect) -> Rect, width: f64, height: f64) -> Page {
    let blocks = page
        .blocks
        .into_iter()
        .map(|b| Block {
            bbox: f(b.bbox),
            lines: b
                .lines
                .into_iter()
                .map(|l| Line {
                    bbox: f(l.bbox),
                    chars: l
                        .chars
                        .into_iter()
                        .map(|c| Char {
                            bbox: f(c.bbox),
                            ..c
                        })
                        .collect(),
                    ..l
                })
                .collect(),
            ..b
        })
        .collect();
    Page {
        width,
        height,
        blocks,
    }
}

/// Full per-page pipeline. `chars`, `width` and `height` are in unrotated
/// PDF user space; `rotation` is the page's `/Rotate` (0/90/180/270). The
/// pipeline runs on the upright page, and every output bbox is mapped back
/// into the caller's unrotated space.
pub fn analyze(chars: Vec<Char>, width: f64, height: f64, rotation: i32, params: &Params) -> Page {
    let (up_w, up_h) = if rotation % 180 == 90 {
        (height, width)
    } else {
        (width, height)
    };
    let chars = chars
        .into_iter()
        .map(|c| Char {
            bbox: c.bbox.rotated(rotation, width, height),
            ..c
        })
        .collect();
    let page = assemble(group_lines(chars, params), params, up_w, up_h);
    map_page_bboxes(
        page,
        |r| r.rotated(360 - rotation, up_w, up_h),
        width,
        height,
    )
}
