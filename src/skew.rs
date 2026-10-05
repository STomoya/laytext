use crate::geometry::Rect;
use crate::types::Char;

/// A run needs at least this many chars to be fit at all.
const MIN_CHARS_PER_RUN: usize = 6;

/// A run must span at least this many median char heights horizontally.
/// Shorter runs (table cells, numbers) are dominated by per-glyph box
/// differences rather than page tilt: on the real corpus, rows of 8-char
/// cells drove a flat page's estimate to ~2deg until this was raised.
const MIN_RUN_SPAN_HEIGHTS: f64 = 10.0;

/// Below this many usable runs, the page has too little text to estimate
/// confidently — return `0.0` (no correction) rather than guess from noise.
const MIN_RUNS_FOR_PAGE_ESTIMATE: usize = 3;

/// Runs steeper than this are rotated text (axis labels, stamps), not scan
/// skew, and are left out of the page estimate.
const MAX_RUN_ANGLE_DEGREES: f64 = 15.0;

fn median_char_height(chars: &[Char]) -> f64 {
    if chars.is_empty() {
        return 0.0;
    }
    median(chars.iter().map(|c| c.bbox.height()).collect())
}

/// Least-squares slope of `y` vs `x` for `points`, or `None` if `x` has no
/// spread (a vertical/degenerate fit) or the fit is otherwise non-finite.
fn fit_slope(points: &[(f64, f64)]) -> Option<f64> {
    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|p| p.0).sum();
    let sum_y: f64 = points.iter().map(|p| p.1).sum();
    let sum_xy: f64 = points.iter().map(|p| p.0 * p.1).sum();
    let sum_xx: f64 = points.iter().map(|p| p.0 * p.0).sum();
    let denom = n * sum_xx - sum_x * sum_x;
    if !denom.is_finite() || denom == 0.0 {
        return None;
    }
    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    slope.is_finite().then_some(slope)
}

/// Does `next` continue `prev`'s visual line? Relies on input reading
/// order (as `group_lines` already does): consecutive chars that advance
/// left-to-right with a small horizontal gap and a small vertical step are
/// on the same line. Line breaks, column jumps and vertical text all fail
/// the test, so a run never mixes lines — unlike bucketing by y, which
/// chains adjacent lines whenever line pitch is under the bucket window.
fn continues_run(prev: &Rect, next: &Rect, h: f64) -> bool {
    let gap = next.x0 - prev.x1;
    next.x0 + next.x1 > prev.x0 + prev.x1
        && gap >= -0.5 * h
        && gap <= 1.5 * h
        && (next.y0 - prev.y0).abs() <= 0.5 * h
}

/// Splits `chars` into same-line runs (see [`continues_run`]) and fits each
/// long-enough run's angle by least squares on bottom-center `y` vs `x`.
fn run_angles(chars: &[Char]) -> Vec<f64> {
    let h = median_char_height(chars);
    let mut runs: Vec<&[Char]> = Vec::new();
    let mut start = 0;
    for i in 1..=chars.len() {
        if i == chars.len() || !continues_run(&chars[i - 1].bbox, &chars[i].bbox, h) {
            runs.push(&chars[start..i]);
            start = i;
        }
    }
    runs.into_iter()
        .filter(|run| {
            run.len() >= MIN_CHARS_PER_RUN
                && run[run.len() - 1].bbox.x1 - run[0].bbox.x0 >= MIN_RUN_SPAN_HEIGHTS * h
        })
        .filter_map(|run| {
            let points: Vec<(f64, f64)> = run
                .iter()
                .map(|c| ((c.bbox.x0 + c.bbox.x1) / 2.0, c.bbox.y0))
                .collect();
            fit_slope(&points).map(|slope| slope.atan().to_degrees())
        })
        .filter(|angle| angle.abs() <= MAX_RUN_ANGLE_DEGREES)
        .collect()
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

/// Below this estimated angle, correction is skipped entirely and
/// `group_lines` runs exactly as before this feature existed — keeps the
/// common case (the vast majority of real-corpus pages, per the
/// originating spike's median of 0.02°) a no-op, both for correctness (no
/// risk of the estimate's own noise perturbing an already-fine page) and
/// for performance (skip the extra pass).
pub(crate) const SKEW_NOISE_FLOOR_DEGREES: f64 = 0.15;

/// Shear-corrects a working copy of each char's bbox for use in grouping
/// *decisions only* — `y' = y - x * tan(angle)`, using each char's
/// horizontal bbox center as `x`. Never mutates or returns the original
/// `Char`s; callers must keep using the original, uncorrected bboxes for
/// anything that ends up in output geometry.
pub(crate) fn shear_correct_bboxes(chars: &[Char], angle_degrees: f64) -> Vec<Rect> {
    let shift_per_x = angle_degrees.to_radians().tan();
    chars
        .iter()
        .map(|c| {
            let x_center = (c.bbox.x0 + c.bbox.x1) / 2.0;
            let shift = x_center * shift_per_x;
            Rect {
                x0: c.bbox.x0,
                x1: c.bbox.x1,
                y0: c.bbox.y0 - shift,
                y1: c.bbox.y1 - shift,
            }
        })
        .collect()
}

/// Estimates a page's dominant skew angle in degrees (`0.0` if there's
/// too little text to estimate confidently), via a per-run least-squares
/// fit over same-line runs of consecutive chars, aggregated by median
/// (robust to outlier runs — formulas, watermarks, rotated labels).
/// Geometry-only: no font metrics. `chars` must be in reading order.
pub fn estimate_page_skew(chars: &[Char]) -> f64 {
    let angles = run_angles(chars);
    if angles.len() < MIN_RUNS_FOR_PAGE_ESTIMATE {
        return 0.0;
    }
    median(angles)
}

#[cfg(test)]
mod tests {
    use super::{fit_slope, median, median_char_height, shear_correct_bboxes};
    use crate::geometry::Rect;
    use crate::types::Char;

    fn ch(x0: f64, y0: f64, x1: f64, y1: f64) -> Char {
        Char {
            bbox: Rect { x0, y0, x1, y1 },
            text: 'x',
            font: None,
        }
    }

    #[test]
    fn median_char_height_empty_returns_zero() {
        assert_eq!(median_char_height(&[]), 0.0);
    }

    #[test]
    fn median_char_height_odd_count_returns_middle_value() {
        let chars = vec![
            ch(0.0, 0.0, 6.0, 5.0),
            ch(0.0, 0.0, 6.0, 20.0),
            ch(0.0, 0.0, 6.0, 10.0),
        ];
        assert_eq!(median_char_height(&chars), 10.0);
    }

    #[test]
    fn median_char_height_even_count_returns_average_of_middle_two() {
        let chars = vec![ch(0.0, 0.0, 6.0, 10.0), ch(0.0, 0.0, 6.0, 30.0)];
        assert_eq!(median_char_height(&chars), 20.0);
    }

    #[test]
    fn fit_slope_returns_the_least_squares_slope() {
        // Perfect line y = 2x: slope must be exactly 2.0.
        let points = [(0.0, 0.0), (1.0, 2.0), (2.0, 4.0), (3.0, 6.0)];
        assert_eq!(fit_slope(&points), Some(2.0));
    }

    #[test]
    fn fit_slope_returns_none_when_x_has_no_spread() {
        // Every point at x=5.0: a vertical stack, no slope is defined.
        let points = [(5.0, 0.0), (5.0, 10.0), (5.0, 20.0)];
        assert_eq!(fit_slope(&points), None);
    }

    #[test]
    fn median_odd_count_returns_middle_value() {
        assert_eq!(median(vec![3.0, 1.0, 2.0]), 2.0);
    }

    #[test]
    fn median_even_count_returns_average_of_middle_two() {
        assert_eq!(median(vec![1.0, 2.0, 3.0, 4.0]), 2.5);
    }

    #[test]
    fn shear_correct_bboxes_shifts_y_by_x_center_times_tan_angle() {
        let chars = vec![ch(0.0, 100.0, 10.0, 110.0), ch(100.0, 100.0, 110.0, 110.0)];
        let angle = 10.0;
        let corrected = shear_correct_bboxes(&chars, angle);

        let shift0 = 5.0 * angle.to_radians().tan();
        let shift1 = 105.0 * angle.to_radians().tan();
        assert_eq!(corrected[0].y0, 100.0 - shift0);
        assert_eq!(corrected[0].y1, 110.0 - shift0);
        assert_eq!(corrected[1].y0, 100.0 - shift1);
        assert_eq!(corrected[1].y1, 110.0 - shift1);
    }

    #[test]
    fn shear_correct_bboxes_leaves_x_unchanged() {
        let chars = vec![ch(0.0, 100.0, 10.0, 110.0)];
        let corrected = shear_correct_bboxes(&chars, 10.0);
        assert_eq!(corrected[0].x0, 0.0);
        assert_eq!(corrected[0].x1, 10.0);
    }

    #[test]
    fn shear_correct_bboxes_zero_angle_is_identity() {
        let chars = vec![ch(0.0, 100.0, 10.0, 110.0), ch(50.0, 5.0, 60.0, 20.0)];
        let corrected = shear_correct_bboxes(&chars, 0.0);
        assert_eq!(corrected[0], chars[0].bbox);
        assert_eq!(corrected[1], chars[1].bbox);
    }

    #[test]
    fn shear_correct_bboxes_empty_input_returns_empty() {
        assert_eq!(shear_correct_bboxes(&[], 5.0), Vec::<Rect>::new());
    }
}
