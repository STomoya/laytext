use crate::geometry::Rect;
use crate::types::Char;

/// Below this many chars, a band's angle isn't fit at all — mirrors the
/// spike script's own `MIN_CHARS_FOR_FIT` threshold.
const MIN_CHARS_FOR_BAND_FIT: usize = 4;

/// Below this many usable band angles, the page has too little text to
/// estimate confidently — return `0.0` (no correction) rather than guess
/// from noise.
const MIN_BANDS_FOR_PAGE_ESTIMATE: usize = 3;

fn median_char_height(chars: &[Char]) -> f64 {
    if chars.is_empty() {
        return 0.0;
    }
    let mut heights: Vec<f64> = chars.iter().map(|c| c.bbox.height()).collect();
    heights.sort_by(f64::total_cmp);
    let mid = heights.len() / 2;
    if heights.len().is_multiple_of(2) {
        (heights[mid - 1] + heights[mid]) / 2.0
    } else {
        heights[mid]
    }
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

/// Greedily buckets `chars` (sorted top-to-bottom by `y0`) into bands using
/// a permissive y-window derived from the page's median char height, then
/// fits each large-enough band's angle by least squares on bottom-center
/// `y` vs `x`. Deliberately wider than `halign`'s own overlap tolerance —
/// the goal here is "roughly the same visual line" for estimation, not a
/// precise grouping decision.
fn band_angles(chars: &[Char]) -> Vec<f64> {
    if chars.is_empty() {
        return Vec::new();
    }
    let window = 1.5 * median_char_height(chars);
    let mut sorted: Vec<&Char> = chars.iter().collect();
    sorted.sort_by(|a, b| b.bbox.y0.total_cmp(&a.bbox.y0));

    let mut bands: Vec<Vec<&Char>> = Vec::new();
    for c in sorted {
        match bands.last_mut() {
            Some(band) if (band.last().unwrap().bbox.y0 - c.bbox.y0).abs() <= window => {
                band.push(c);
            }
            _ => bands.push(vec![c]),
        }
    }

    bands
        .into_iter()
        .filter(|band| band.len() >= MIN_CHARS_FOR_BAND_FIT)
        .filter_map(|band| {
            let points: Vec<(f64, f64)> = band
                .iter()
                .map(|c| ((c.bbox.x0 + c.bbox.x1) / 2.0, c.bbox.y0))
                .collect();
            fit_slope(&points).map(|slope| slope.atan().to_degrees())
        })
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
#[allow(dead_code)]
pub(crate) const SKEW_NOISE_FLOOR_DEGREES: f64 = 0.15;

/// Shear-corrects a working copy of each char's bbox for use in grouping
/// *decisions only* — `y' = y - x * tan(angle)`, using each char's
/// horizontal bbox center as `x`. Never mutates or returns the original
/// `Char`s; callers must keep using the original, uncorrected bboxes for
/// anything that ends up in output geometry.
#[allow(dead_code)]
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
/// too little text to estimate confidently), via a permissive band
/// bucketing + per-band least-squares fit, aggregated by median (robust
/// to the small number of extreme-outlier bands real scans exhibit —
/// formulas, watermarks, short fragments). Geometry-only: no font metrics.
pub fn estimate_page_skew(chars: &[Char]) -> f64 {
    let angles = band_angles(chars);
    if angles.len() < MIN_BANDS_FOR_PAGE_ESTIMATE {
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
