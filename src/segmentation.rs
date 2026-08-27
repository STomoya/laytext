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

/// `line.bbox.width()` outlier factor for the X-axis obstacle mask: a line
/// wider than this times the region's median line width is treated as a
/// bridging obstacle (a caption/table-row spanning a column gutter) and
/// excluded from gap projection. Only "too wide" is flagged, not "too
/// narrow" - a narrow line can't bridge a gap - which means the narrowest
/// line in any set is provably never flagged (min <= median <= threshold
/// whenever this factor is > 1.0), so masking can never remove every
/// candidate interval.
const OBSTACLE_WIDTH_FACTOR: f64 = 1.25;

fn median_line_height(lines: &[Line]) -> f64 {
    let mut heights: Vec<f64> = lines.iter().map(|l| l.bbox.height()).collect();
    heights.sort_by(f64::total_cmp);
    let mid = heights.len() / 2;
    if heights.len().is_multiple_of(2) {
        (heights[mid - 1] + heights[mid]) / 2.0
    } else {
        heights[mid]
    }
}

fn median_line_width(lines: &[Line]) -> f64 {
    let mut widths: Vec<f64> = lines.iter().map(|l| l.bbox.width()).collect();
    widths.sort_by(f64::total_cmp);
    let mid = widths.len() / 2;
    if widths.len().is_multiple_of(2) {
        (widths[mid - 1] + widths[mid]) / 2.0
    } else {
        widths[mid]
    }
}

fn mask_bridging_obstacles(lines: &[Line]) -> Vec<(f64, f64)> {
    let threshold = OBSTACLE_WIDTH_FACTOR * median_line_width(lines);
    lines
        .iter()
        .filter(|l| l.bbox.width() <= threshold)
        .map(|l| (l.bbox.x0, l.bbox.x1))
        .collect()
}

fn widest_gap(gaps: Vec<(f64, f64)>) -> Option<(f64, f64)> {
    gaps.into_iter()
        .max_by(|a, b| (a.1 - a.0).total_cmp(&(b.1 - b.0)))
}

/// Like [`widest_gap`] over the interval projection, but tolerant of a
/// single intruding interval (e.g. a caption/table-row line bridging a
/// column gutter) that would otherwise merge every run together and hide
/// the gap entirely. Only engages when the plain projection finds nothing,
/// so it never changes behavior for a region where a gap is already found.
///
/// X-axis only ([`try_axis_cut_x`]): stacked lines on the Y axis are
/// naturally non-overlapping neighbors, so excluding any one of them
/// synthesizes a fake gap between its neighbors even in ordinary paragraph
/// text (see git history for the regression this caused when tried on Y).
/// Column text is safe because real paragraph lines share x-overlap (a
/// common left/center edge), so excluding one can't manufacture a gap
/// between the rest — only a genuine bridging intruder can.
///
/// ponytail: O(n^2 log n) single-interval-exclusion search, not a proper
/// maximal-whitespace-rectangle algorithm. Upgrade if the real corpus shows
/// intrusions from more than one interior line at once.
fn widest_gap_tolerant(intervals: &[(f64, f64)], min_gap: f64) -> Option<(f64, f64)> {
    let baseline = widest_gap(find_gaps(intervals, min_gap));
    if baseline.is_some() || intervals.len() < 3 {
        return baseline;
    }
    (0..intervals.len())
        .filter_map(|i| {
            let mut subset = intervals.to_vec();
            subset.remove(i);
            widest_gap(find_gaps(&subset, min_gap))
        })
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

/// An axis cut candidate: the two partitions plus the gap width that
/// separates them, so callers can compare candidates across axes and pick
/// whichever gap is actually widest.
struct AxisCut {
    left_or_top: Vec<Line>,
    right_or_bottom: Vec<Line>,
    gap_width: f64,
}

fn try_axis_cut_x(lines: &[Line], gap_min: f64) -> Option<AxisCut> {
    let intervals: Vec<(f64, f64)> = lines.iter().map(|l| (l.bbox.x0, l.bbox.x1)).collect();
    let gap = widest_gap_tolerant(&intervals, gap_min)?;
    let mid = (gap.0 + gap.1) / 2.0;
    let (left, right): (Vec<Line>, Vec<Line>) = lines
        .iter()
        .cloned()
        .partition(|l| (l.bbox.x0 + l.bbox.x1) / 2.0 < mid);
    Some(AxisCut {
        left_or_top: left,
        right_or_bottom: right,
        gap_width: gap.1 - gap.0,
    })
}

fn try_axis_cut_y(lines: &[Line], gap_min: f64) -> Option<AxisCut> {
    let intervals: Vec<(f64, f64)> = lines.iter().map(|l| (l.bbox.y0, l.bbox.y1)).collect();
    let gap = widest_gap(find_gaps(&intervals, gap_min))?;
    let mid = (gap.0 + gap.1) / 2.0;
    let (top, bottom): (Vec<Line>, Vec<Line>) = lines
        .iter()
        .cloned()
        .partition(|l| (l.bbox.y0 + l.bbox.y1) / 2.0 >= mid);
    Some(AxisCut {
        left_or_top: top,
        right_or_bottom: bottom,
        gap_width: gap.1 - gap.0,
    })
}

/// Recursively partitions a page's lines into a `Region` tree via an X-Y
/// cut: a forced horizontal split around any full-width line (title/header/
/// footer) mixed with narrower lines, then a whitespace-gap cut on whichever
/// axis has the wider candidate gap (column vs. row), so a page with e.g.
/// two stacked bands that each happen to also be two-column splits on the
/// band boundary first rather than always defaulting to columns. A region
/// that matches none of these becomes a `Leaf`, ready for per-region
/// line-to-block merging.
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

    let mut median_height_cache: Option<f64> = None;
    let mut median_height =
        || *median_height_cache.get_or_insert_with(|| median_line_height(&lines));

    let column_gap_min = params
        .column_gap_min
        .unwrap_or_else(|| median_height() * AUTO_COLUMN_GAP_FACTOR);
    let row_gap_min = params
        .row_gap_min
        .unwrap_or_else(|| median_height() * AUTO_ROW_GAP_FACTOR);

    let x_cut = try_axis_cut_x(&lines, column_gap_min);
    let y_cut = try_axis_cut_y(&lines, row_gap_min);

    let prefer_x = match (&x_cut, &y_cut) {
        (Some(x), Some(y)) => x.gap_width >= y.gap_width,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => return Region::Leaf { bbox, lines },
    };

    if prefer_x {
        let x = x_cut.unwrap();
        Region::Split {
            bbox,
            orientation: Orientation::Vertical,
            children: vec![
                segment(x.left_or_top, params),
                segment(x.right_or_bottom, params),
            ],
        }
    } else {
        let y = y_cut.unwrap();
        Region::Split {
            bbox,
            orientation: Orientation::Horizontal,
            children: vec![
                segment(y.left_or_top, params),
                segment(y.right_or_bottom, params),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mask_bridging_obstacles, median_line_height, median_line_width, widest_gap, widest_gap_tolerant};

    #[test]
    fn widest_gap_nan_width_does_not_panic() {
        let result = widest_gap(vec![(0.0, 5.0), (0.0, f64::NAN)]);
        assert!(result.is_some());
    }

    #[test]
    fn widest_gap_tolerant_returns_the_widest_gap_across_all_single_line_exclusions() {
        // Baseline (all 5 intervals) merges into a single run: bridge1
        // connects p1 to p2, bridge2 connects p2 to p3, so nothing is found
        // without excluding one of them. Excluding bridge1 alone reveals a
        // 20pt gap; excluding bridge2 alone reveals a wider 30pt gap - the
        // wider one must win, not just whichever is found first.
        let p1 = (0.0, 10.0);
        let bridge1 = (8.0, 32.0);
        let p2 = (30.0, 40.0);
        let bridge2 = (38.0, 75.0);
        let p3 = (70.0, 130.0);
        let intervals = vec![p1, bridge1, p2, bridge2, p3];
        assert_eq!(widest_gap_tolerant(&intervals, 5.0), Some((40.0, 70.0)));
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
    fn median_line_height_even_count_returns_average_of_middle_two() {
        use crate::types::{Char, Line};

        let heights = [10.0, 30.0];
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
        assert_eq!(median_line_height(&lines), 20.0);
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

    #[test]
    fn median_line_width_odd_count_returns_middle_value() {
        use crate::types::{Char, Line};

        let widths = [5.0, 20.0, 10.0];
        let lines: Vec<Line> = widths
            .iter()
            .map(|&w| {
                let bbox = crate::geometry::Rect {
                    x0: 0.0,
                    y0: 0.0,
                    x1: w,
                    y1: 10.0,
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
        assert_eq!(median_line_width(&lines), 10.0);
    }

    #[test]
    fn median_line_width_even_count_returns_average_of_middle_two() {
        use crate::types::{Char, Line};

        let widths = [10.0, 30.0];
        let lines: Vec<Line> = widths
            .iter()
            .map(|&w| {
                let bbox = crate::geometry::Rect {
                    x0: 0.0,
                    y0: 0.0,
                    x1: w,
                    y1: 10.0,
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
        assert_eq!(median_line_width(&lines), 20.0);
    }

    #[test]
    fn mask_bridging_obstacles_keeps_all_lines_when_widths_are_uniform() {
        use crate::types::{Char, Line};

        let rect = |x0: f64, x1: f64| crate::geometry::Rect {
            x0,
            y0: 0.0,
            x1,
            y1: 10.0,
        };
        let line = |bbox: crate::geometry::Rect| Line {
            bbox,
            upright: true,
            chars: vec![Char {
                bbox,
                text: 'x',
                font: None,
            }],
        };
        // all width 50.0: nothing is a width outlier
        let lines = vec![
            line(rect(0.0, 50.0)),
            line(rect(70.0, 120.0)),
            line(rect(200.0, 250.0)),
        ];
        assert_eq!(
            mask_bridging_obstacles(&lines),
            vec![(0.0, 50.0), (70.0, 120.0), (200.0, 250.0)]
        );
    }

    #[test]
    fn mask_bridging_obstacles_excludes_a_lone_wide_outlier() {
        use crate::types::{Char, Line};

        let rect = |x0: f64, x1: f64| crate::geometry::Rect {
            x0,
            y0: 0.0,
            x1,
            y1: 10.0,
        };
        let line = |bbox: crate::geometry::Rect| Line {
            bbox,
            upright: true,
            chars: vec![Char {
                bbox,
                text: 'x',
                font: None,
            }],
        };
        // widths [50, 50, 150]; median 50.0, threshold 62.5: only the 150-wide
        // line (a caption/table-row bridging a gutter) is excluded.
        let lines = vec![
            line(rect(0.0, 50.0)),
            line(rect(70.0, 120.0)),
            line(rect(150.0, 300.0)),
        ];
        assert_eq!(
            mask_bridging_obstacles(&lines),
            vec![(0.0, 50.0), (70.0, 120.0)]
        );
    }

    #[test]
    fn mask_bridging_obstacles_excludes_multiple_simultaneous_outliers() {
        use crate::types::{Char, Line};

        let rect = |x0: f64, x1: f64| crate::geometry::Rect {
            x0,
            y0: 0.0,
            x1,
            y1: 10.0,
        };
        let line = |bbox: crate::geometry::Rect| Line {
            bbox,
            upright: true,
            chars: vec![Char {
                bbox,
                text: 'x',
                font: None,
            }],
        };
        // widths [50, 50, 200, 210]; median 125.0, threshold 156.25: both wide
        // lines are excluded simultaneously - the case the old single-exclusion
        // widest_gap_tolerant could never handle.
        let lines = vec![
            line(rect(0.0, 50.0)),
            line(rect(60.0, 110.0)),
            line(rect(0.0, 200.0)),
            line(rect(10.0, 220.0)),
        ];
        assert_eq!(
            mask_bridging_obstacles(&lines),
            vec![(0.0, 50.0), (60.0, 110.0)]
        );
    }

    #[test]
    fn mask_bridging_obstacles_never_excludes_the_narrowest_line() {
        use crate::types::{Char, Line};

        let rect = |x0: f64, x1: f64| crate::geometry::Rect {
            x0,
            y0: 0.0,
            x1,
            y1: 10.0,
        };
        let line = |bbox: crate::geometry::Rect| Line {
            bbox,
            upright: true,
            chars: vec![Char {
                bbox,
                text: 'x',
                font: None,
            }],
        };
        // widths [1, 1, 1, 100, 100, 100]; median 50.5, threshold 63.125: the
        // majority (the three 100-wide lines) get excluded, but the narrowest
        // line can never be excluded for any OBSTACLE_WIDTH_FACTOR > 1.0 (the
        // minimum of any set is always <= its median, so it's always <= the
        // threshold too) - this is why mask_bridging_obstacles needs no
        // "masked everything" guard: that state is unreachable.
        let lines = vec![
            line(rect(0.0, 1.0)),
            line(rect(10.0, 11.0)),
            line(rect(20.0, 21.0)),
            line(rect(100.0, 200.0)),
            line(rect(300.0, 400.0)),
            line(rect(500.0, 600.0)),
        ];
        assert_eq!(
            mask_bridging_obstacles(&lines),
            vec![(0.0, 1.0), (10.0, 11.0), (20.0, 21.0)]
        );
    }
}
