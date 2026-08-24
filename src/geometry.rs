use pyo3::prelude::*;

#[pyclass(get_all, from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

#[pymethods]
impl Rect {
    #[new]
    fn py_new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Rect { x0, y0, x1, y1 }
    }
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    pub fn is_hoverlap(&self, other: &Rect) -> bool {
        other.x0 <= self.x1 && self.x0 <= other.x1
    }

    pub fn hoverlap(&self, other: &Rect) -> f64 {
        if self.is_hoverlap(other) {
            (self.x0 - other.x1).abs().min((self.x1 - other.x0).abs())
        } else {
            0.0
        }
    }

    pub fn hdistance(&self, other: &Rect) -> f64 {
        if self.is_hoverlap(other) {
            0.0
        } else {
            (self.x0 - other.x1).abs().min((self.x1 - other.x0).abs())
        }
    }

    pub fn is_voverlap(&self, other: &Rect) -> bool {
        other.y0 <= self.y1 && self.y0 <= other.y1
    }

    pub fn voverlap(&self, other: &Rect) -> f64 {
        if self.is_voverlap(other) {
            (self.y0 - other.y1).abs().min((self.y1 - other.y0).abs())
        } else {
            0.0
        }
    }

    pub fn vdistance(&self, other: &Rect) -> f64 {
        if self.is_voverlap(other) {
            0.0
        } else {
            (self.y0 - other.y1).abs().min((self.y1 - other.y0).abs())
        }
    }

    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }
}

/// Empty input returns a zero rect.
pub fn union_all(rects: impl IntoIterator<Item = Rect>) -> Rect {
    let mut iter = rects.into_iter();
    let first = match iter.next() {
        Some(r) => r,
        None => {
            return Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 0.0,
                y1: 0.0,
            };
        }
    };
    iter.fold(first, |acc, r| acc.union(&r))
}

/// Projects `intervals` onto an axis, merges overlapping/touching ones into
/// runs, and returns the gaps between consecutive runs that are at least
/// `min_gap` wide, in ascending order.
pub fn find_gaps(intervals: &[(f64, f64)], min_gap: f64) -> Vec<(f64, f64)> {
    let mut sorted: Vec<(f64, f64)> = intervals.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut runs: Vec<(f64, f64)> = Vec::new();
    for iv in sorted {
        match runs.last_mut() {
            Some(run) if iv.0 <= run.1 => {
                if iv.1 > run.1 {
                    run.1 = iv.1;
                }
            }
            _ => runs.push(iv),
        }
    }

    runs.windows(2)
        .filter_map(|w| {
            let gap = (w[0].1, w[1].0);
            (gap.1 - gap.0 >= min_gap).then_some(gap)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Rect;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    #[test]
    fn py_new_assigns_all_fields_from_arguments() {
        let r = Rect::py_new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(r, rect(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn rect_extracts_from_python_object() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let obj = pyo3::Py::new(py, rect(0.0, 0.0, 1.0, 1.0)).unwrap();
            let extracted: Rect = obj.extract(py).unwrap();
            assert_eq!(extracted, rect(0.0, 0.0, 1.0, 1.0));
        });
    }

    #[test]
    fn rect_extraction_fails_for_wrong_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = PyString::new(py, "not a rect");
            let extracted: Result<Rect, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }

    #[test]
    fn width_and_height() {
        let r = rect(1.0, 2.0, 5.0, 9.0);
        assert_eq!(r.width(), 4.0);
        assert_eq!(r.height(), 7.0);
    }

    #[test]
    fn hoverlap_true_when_x_ranges_overlap() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(3.0, 0.0, 9.0, 10.0);
        assert!(a.is_hoverlap(&b));
    }

    #[test]
    fn hoverlap_false_when_x_ranges_disjoint() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(20.0, 0.0, 26.0, 10.0);
        assert!(!a.is_hoverlap(&b));
    }

    #[test]
    fn hoverlap_amount_matches_pdfminer_formula() {
        // ported formula: min(|a.x0-b.x1|, |a.x1-b.x0|), not true overlap width
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(3.0, 0.0, 9.0, 10.0);
        assert_eq!(a.hoverlap(&b), 3.0);
    }

    #[test]
    fn hoverlap_zero_when_disjoint() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(20.0, 0.0, 26.0, 10.0);
        assert_eq!(a.hoverlap(&b), 0.0);
    }

    #[test]
    fn hdistance_zero_when_overlapping() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(3.0, 0.0, 9.0, 10.0);
        assert_eq!(a.hdistance(&b), 0.0);
    }

    #[test]
    fn hdistance_positive_gap_when_disjoint() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(9.0, 0.0, 15.0, 10.0);
        assert_eq!(a.hdistance(&b), 3.0);
    }

    #[test]
    fn voverlap_true_when_y_ranges_overlap() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(0.0, 5.0, 6.0, 15.0);
        assert!(a.is_voverlap(&b));
    }

    #[test]
    fn voverlap_false_when_y_ranges_disjoint() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(0.0, 20.0, 6.0, 30.0);
        assert!(!a.is_voverlap(&b));
    }

    #[test]
    fn voverlap_amount_matches_pdfminer_formula() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(0.0, 5.0, 6.0, 15.0);
        assert_eq!(a.voverlap(&b), 5.0);
    }

    #[test]
    fn voverlap_zero_when_disjoint() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(0.0, 20.0, 6.0, 30.0);
        assert_eq!(a.voverlap(&b), 0.0);
    }

    #[test]
    fn vdistance_zero_when_overlapping() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(0.0, 5.0, 6.0, 15.0);
        assert_eq!(a.vdistance(&b), 0.0);
    }

    #[test]
    fn vdistance_positive_gap_when_disjoint() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(0.0, 13.0, 6.0, 20.0);
        assert_eq!(a.vdistance(&b), 3.0);
    }

    #[test]
    fn union_covers_the_bounding_box_of_both_rects() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(3.0, 5.0, 20.0, 30.0);
        assert_eq!(a.union(&b), rect(0.0, 0.0, 20.0, 30.0));
    }

    #[test]
    fn union_all_empty_input_returns_a_zero_rect() {
        assert_eq!(super::union_all([]), rect(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn union_all_single_rect_returns_itself() {
        let a = rect(1.0, 2.0, 3.0, 4.0);
        assert_eq!(super::union_all([a]), a);
    }

    #[test]
    fn union_all_folds_multiple_rects() {
        let a = rect(0.0, 0.0, 6.0, 10.0);
        let b = rect(3.0, 5.0, 20.0, 30.0);
        let c = rect(-5.0, 2.0, 4.0, 6.0);
        assert_eq!(super::union_all([a, b, c]), rect(-5.0, 0.0, 20.0, 30.0));
    }

    #[test]
    fn find_gaps_empty_input_returns_no_gaps() {
        assert_eq!(super::find_gaps(&[], 1.0), vec![]);
    }

    #[test]
    fn find_gaps_single_interval_returns_no_gaps() {
        assert_eq!(super::find_gaps(&[(0.0, 5.0)], 1.0), vec![]);
    }

    #[test]
    fn find_gaps_overlapping_intervals_merge_without_a_gap() {
        assert_eq!(super::find_gaps(&[(0.0, 5.0), (3.0, 8.0)], 1.0), vec![]);
    }

    #[test]
    fn find_gaps_touching_intervals_merge_without_a_gap() {
        assert_eq!(super::find_gaps(&[(0.0, 5.0), (5.0, 8.0)], 1.0), vec![]);
    }

    #[test]
    fn find_gaps_gap_smaller_than_min_gap_is_not_reported() {
        assert_eq!(super::find_gaps(&[(0.0, 5.0), (5.5, 8.0)], 1.0), vec![]);
    }

    #[test]
    fn find_gaps_gap_at_least_min_gap_is_reported() {
        assert_eq!(
            super::find_gaps(&[(0.0, 5.0), (7.0, 8.0)], 2.0),
            vec![(5.0, 7.0)]
        );
    }

    #[test]
    fn find_gaps_unsorted_input_is_handled() {
        assert_eq!(
            super::find_gaps(&[(7.0, 8.0), (0.0, 5.0)], 2.0),
            vec![(5.0, 7.0)]
        );
    }

    #[test]
    fn find_gaps_returns_multiple_gaps_in_order() {
        assert_eq!(
            super::find_gaps(&[(0.0, 5.0), (10.0, 15.0), (20.0, 25.0)], 2.0),
            vec![(5.0, 10.0), (15.0, 20.0)]
        );
    }
}
