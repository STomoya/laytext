use crate::geometry::{Rect, find_gaps, union_all};
use crate::params::Params;
use crate::types::Line;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Region {
    Leaf {
        bbox: Rect,
        lines: Vec<Line>,
    },
    Split {
        bbox: Rect,
        orientation: Orientation,
        children: Vec<Region>,
    },
}

/// `column_gap_min`/`row_gap_min` fallback multipliers, applied to the
/// median line height of the region being segmented when a threshold isn't
/// given explicitly. Geometry-only (no font metrics), per the project's
/// threshold-fallback rule; approximates the corpus-tuned fixed defaults
/// (20pt/15pt against ~10pt body text) used in `validation/`.
const AUTO_COLUMN_GAP_FACTOR: f64 = 2.0;
const AUTO_ROW_GAP_FACTOR: f64 = 1.5;

fn median_line_height(lines: &[Line]) -> f64 {
    let mut heights: Vec<f64> = lines.iter().map(|l| l.bbox.height()).collect();
    heights.sort_by(f64::total_cmp);
    heights[heights.len() / 2]
}

fn widest_gap(gaps: Vec<(f64, f64)>) -> Option<(f64, f64)> {
    gaps.into_iter()
        .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
}

/// Above this band count, banding no longer resembles a page-level title/
/// [column-body]/footer boundary (at most 3 bands in every real shape this
/// heuristic targets) and instead signals repeating in-flow structure, like
/// a series of short inline headings each followed by its own paragraph -
/// content noise, not a section boundary.
const MAX_FORCED_BANDS: usize = 3;

/// Splits `lines` into consecutive top-to-bottom bands whose lines share the
/// same "is full width relative to `bbox`" status, and recursively segments
/// each band. Returns `None` when every line has the same status (nothing to
/// force apart), when banding produces nothing but single-line bands (the
/// signature of justified-text noise rather than a real section boundary),
/// or when it produces more than `MAX_FORCED_BANDS` (see its doc comment).
fn try_full_width_split(lines: &[Line], bbox: Rect, params: &Params) -> Option<Vec<Region>> {
    let threshold = params.full_width_threshold * bbox.width();
    let is_full = |l: &Line| l.bbox.width() >= threshold;

    let full_count = lines.iter().filter(|l| is_full(l)).count();
    let narrow_count = lines.len() - full_count;
    // Full-width lines must stay a minority, or an ordinary paragraph's
    // shorter wrapped last line reads as a "title" and gets split off.
    if full_count == 0 || narrow_count == 0 || full_count > narrow_count {
        return None;
    }

    let mut sorted: Vec<Line> = lines.to_vec();
    sorted.sort_by(|a, b| {
        let ay = (a.bbox.y0 + a.bbox.y1) / 2.0;
        let by = (b.bbox.y0 + b.bbox.y1) / 2.0;
        by.total_cmp(&ay)
    });

    let mut bands: Vec<Vec<Line>> = Vec::new();
    let mut current_full: Option<bool> = None;
    for l in sorted {
        let f = is_full(&l);
        if current_full == Some(f) {
            bands.last_mut().unwrap().push(l);
        } else {
            bands.push(vec![l]);
            current_full = Some(f);
        }
    }

    // A genuine section boundary (a title/header/footer band around column
    // content, however many bands that produces) always groups real content
    // together: at least one band has more than one line. A justified
    // paragraph's wrapped lines can straddle the width threshold
    // line-by-line instead, producing a chain where every band is a single
    // line — that's noise, not a section boundary, so leave it to the
    // column/row whitespace-gap cuts (or a plain leaf) rather than forcing
    // it apart here. A long chain of repeating narrow/full bands is the same
    // signature at a coarser grain (see MAX_FORCED_BANDS).
    if bands.len() > MAX_FORCED_BANDS || bands.iter().all(|b| b.len() == 1) {
        return None;
    }

    Some(bands.into_iter().map(|b| segment(b, params)).collect())
}

fn try_axis_cut_x(lines: &[Line], gap_min: f64) -> Option<(Vec<Line>, Vec<Line>)> {
    let intervals: Vec<(f64, f64)> = lines.iter().map(|l| (l.bbox.x0, l.bbox.x1)).collect();
    let gap = widest_gap(find_gaps(&intervals, gap_min))?;
    let (left, right): (Vec<Line>, Vec<Line>) =
        lines.iter().cloned().partition(|l| l.bbox.x1 <= gap.0);
    Some((left, right))
}

fn try_axis_cut_y(lines: &[Line], gap_min: f64) -> Option<(Vec<Line>, Vec<Line>)> {
    let intervals: Vec<(f64, f64)> = lines.iter().map(|l| (l.bbox.y0, l.bbox.y1)).collect();
    let gap = widest_gap(find_gaps(&intervals, gap_min))?;
    let (top, bottom): (Vec<Line>, Vec<Line>) =
        lines.iter().cloned().partition(|l| l.bbox.y0 >= gap.1);
    Some((top, bottom))
}

/// Recursively partitions a page's lines into a `Region` tree via an X-Y
/// cut: a forced horizontal split around any full-width line (title/header/
/// footer) mixed with narrower lines, then a whitespace-gap vertical
/// (column) cut, then a whitespace-gap horizontal (row) cut, tried in that
/// order at every level. A region that matches none of these becomes a
/// `Leaf`, ready for per-region line-to-block merging.
pub fn segment(lines: Vec<Line>, params: &Params) -> Region {
    let bbox = union_all(lines.iter().map(|l| l.bbox));

    if lines.len() <= 1 {
        return Region::Leaf { bbox, lines };
    }

    if let Some(children) = try_full_width_split(&lines, bbox, params) {
        return Region::Split {
            bbox,
            orientation: Orientation::Horizontal,
            children,
        };
    }

    let column_gap_min = params
        .column_gap_min
        .unwrap_or_else(|| median_line_height(&lines) * AUTO_COLUMN_GAP_FACTOR);
    if let Some((left, right)) = try_axis_cut_x(&lines, column_gap_min) {
        return Region::Split {
            bbox,
            orientation: Orientation::Vertical,
            children: vec![segment(left, params), segment(right, params)],
        };
    }

    let row_gap_min = params
        .row_gap_min
        .unwrap_or_else(|| median_line_height(&lines) * AUTO_ROW_GAP_FACTOR);
    if let Some((top, bottom)) = try_axis_cut_y(&lines, row_gap_min) {
        return Region::Split {
            bbox,
            orientation: Orientation::Horizontal,
            children: vec![segment(top, params), segment(bottom, params)],
        };
    }

    Region::Leaf { bbox, lines }
}

#[cfg(test)]
mod tests {
    use super::{median_line_height, widest_gap};

    #[test]
    fn widest_gap_nan_width_does_not_panic() {
        let result = widest_gap(vec![(0.0, 5.0), (0.0, f64::NAN)]);
        assert!(result.is_some());
    }

    #[test]
    fn median_line_height_odd_count_returns_middle_value() {
        use crate::types::{Char, Line};

        let heights = [5.0, 20.0, 10.0];
        let lines: Vec<Line> = heights
            .iter()
            .map(|&h| {
                let bbox = crate::geometry::Rect {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 10.0,
                    y1: h,
                };
                Line {
                    bbox,
                    upright: true,
                    chars: vec![Char {
                        bbox,
                        text: 'x',
                        font: None,
                    }],
                }
            })
            .collect();
        assert_eq!(median_line_height(&lines), 10.0);
    }

    #[test]
    fn try_full_width_split_nan_y_does_not_panic() {
        use crate::params::Params;
        use crate::types::{Char, Line};

        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| crate::geometry::Rect { x0, y0, x1, y1 };
        let line = |bbox: crate::geometry::Rect| Line {
            bbox,
            upright: true,
            chars: vec![Char {
                bbox,
                text: 'x',
                font: None,
            }],
        };

        let narrow = line(rect(0.0, f64::NAN, 10.0, 10.0));
        let full = line(rect(0.0, 0.0, 100.0, 10.0));
        let bbox = rect(0.0, 0.0, 100.0, 10.0);
        let params = Params::default();

        let _ = super::try_full_width_split(&[narrow, full], bbox, &params);
    }
}
