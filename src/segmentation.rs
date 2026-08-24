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

fn widest_gap(gaps: Vec<(f64, f64)>) -> Option<(f64, f64)> {
    gaps.into_iter()
        .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
}

/// Splits `lines` into consecutive top-to-bottom bands whose lines share the
/// same "is full width relative to `bbox`" status, and recursively segments
/// each band. Returns `None` when every line has the same status (nothing to
/// force apart), or when banding produces nothing but single-line bands
/// (see the check below) - the signature of justified-text noise rather
/// than a real section boundary.
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
        by.partial_cmp(&ay).unwrap()
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
    // it apart here.
    if bands.iter().all(|b| b.len() == 1) {
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

    let column_gap_min = params.column_gap_min.expect(
        "Params.column_gap_min is required (non-None) for v1; None is reserved for future auto-tuning",
    );
    if let Some((left, right)) = try_axis_cut_x(&lines, column_gap_min) {
        return Region::Split {
            bbox,
            orientation: Orientation::Vertical,
            children: vec![segment(left, params), segment(right, params)],
        };
    }

    let row_gap_min = params.row_gap_min.expect(
        "Params.row_gap_min is required (non-None) for v1; None is reserved for future auto-tuning",
    );
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
    use super::widest_gap;

    #[test]
    fn widest_gap_nan_width_does_not_panic() {
        let result = widest_gap(vec![(0.0, 5.0), (0.0, f64::NAN)]);
        assert!(result.is_some());
    }
}
