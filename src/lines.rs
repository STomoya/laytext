use crate::geometry::Rect;
use crate::params::Params;
use crate::types::{Char, Line};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Orientation {
    Horizontal,
    Vertical,
}

struct LineBuilder {
    orientation: Orientation,
    word_margin: f64,
    chars: Vec<Char>,
    bbox: Option<Rect>,
    // Horizontal: previous char's x1. Vertical: previous char's y0.
    prev_edge: f64,
}

impl LineBuilder {
    fn new(orientation: Orientation, word_margin: f64) -> Self {
        let prev_edge = match orientation {
            Orientation::Horizontal => f64::INFINITY,
            Orientation::Vertical => f64::NEG_INFINITY,
        };
        LineBuilder {
            orientation,
            word_margin,
            chars: Vec::new(),
            bbox: None,
            prev_edge,
        }
    }

    fn push(&mut self, c: Char) {
        if self.word_margin > 0.0 {
            let margin = self.word_margin * c.bbox.width().max(c.bbox.height());
            let gap_open = match self.orientation {
                Orientation::Horizontal => self.prev_edge < c.bbox.x0 - margin,
                Orientation::Vertical => c.bbox.y1 + margin < self.prev_edge,
            };
            if gap_open {
                let space_bbox = match self.orientation {
                    Orientation::Horizontal => Rect {
                        x0: self.prev_edge,
                        y0: c.bbox.y0,
                        x1: c.bbox.x0,
                        y1: c.bbox.y1,
                    },
                    Orientation::Vertical => Rect {
                        x0: c.bbox.x0,
                        y0: c.bbox.y1,
                        x1: c.bbox.x1,
                        y1: self.prev_edge,
                    },
                };
                self.chars.push(Char {
                    bbox: space_bbox,
                    text: ' ',
                    font: None,
                });
            }
        }
        self.prev_edge = match self.orientation {
            Orientation::Horizontal => c.bbox.x1,
            Orientation::Vertical => c.bbox.y0,
        };
        self.bbox = Some(match self.bbox {
            None => c.bbox,
            Some(b) => Rect {
                x0: b.x0.min(c.bbox.x0),
                y0: b.y0.min(c.bbox.y0),
                x1: b.x1.max(c.bbox.x1),
                y1: b.y1.max(c.bbox.y1),
            },
        });
        self.chars.push(c);
    }

    fn finish(self) -> Line {
        Line {
            bbox: self.bbox.unwrap_or(Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
            }),
            upright: matches!(self.orientation, Orientation::Horizontal),
            chars: self.chars,
        }
    }
}

fn halign(a: &Char, b: &Char, params: &Params) -> bool {
    a.bbox.is_voverlap(&b.bbox)
        && a.bbox.height().min(b.bbox.height()) * params.line_overlap < a.bbox.voverlap(&b.bbox)
        && a.bbox.hdistance(&b.bbox) < a.bbox.width().max(b.bbox.width()) * params.char_margin
}

fn valign(a: &Char, b: &Char, params: &Params) -> bool {
    params.detect_vertical
        && a.bbox.is_hoverlap(&b.bbox)
        && a.bbox.width().min(b.bbox.width()) * params.line_overlap < a.bbox.hoverlap(&b.bbox)
        && a.bbox.vdistance(&b.bbox) < a.bbox.height().max(b.bbox.height()) * params.char_margin
}

/// Groups a page's chars into lines. Direct port of pdfminer's
/// `LTLayoutContainer.group_objects`. Input must already be in roughly
/// reading order (pdfminer relies on PDF content-stream order); this
/// function does not sort.
pub fn group_lines(chars: &[Char], params: &Params) -> Vec<Line> {
    let mut result = Vec::new();
    let mut open: Option<LineBuilder> = None;
    let mut prev: Option<&Char> = None;

    for c in chars {
        if let Some(obj0) = prev {
            let ha = halign(obj0, c, params);
            let va = valign(obj0, c, params);

            let extended = match &mut open {
                Some(builder) => match builder.orientation {
                    Orientation::Horizontal if ha => {
                        builder.push(c.clone());
                        true
                    }
                    Orientation::Vertical if va => {
                        builder.push(c.clone());
                        true
                    }
                    _ => false,
                },
                None => false,
            };

            if !extended {
                if let Some(builder) = open.take() {
                    result.push(builder.finish());
                } else if va && !ha {
                    let mut builder = LineBuilder::new(Orientation::Vertical, params.word_margin);
                    builder.push(obj0.clone());
                    builder.push(c.clone());
                    open = Some(builder);
                } else if ha && !va {
                    let mut builder = LineBuilder::new(Orientation::Horizontal, params.word_margin);
                    builder.push(obj0.clone());
                    builder.push(c.clone());
                    open = Some(builder);
                } else {
                    let mut builder = LineBuilder::new(Orientation::Horizontal, params.word_margin);
                    builder.push(obj0.clone());
                    result.push(builder.finish());
                }
            }
        }
        prev = Some(c);
    }

    match open {
        Some(builder) => result.push(builder.finish()),
        None => {
            if let Some(obj0) = prev {
                let mut builder = LineBuilder::new(Orientation::Horizontal, params.word_margin);
                builder.push(obj0.clone());
                result.push(builder.finish());
            }
        }
    }

    result
}
