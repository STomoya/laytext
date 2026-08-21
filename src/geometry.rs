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
}

#[cfg(test)]
mod tests {
    use super::Rect;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
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
}
