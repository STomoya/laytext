use pyo3::prelude::*;

#[pyclass(get_all, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub char_margin: f64,
    pub line_overlap: f64,
    pub line_margin: f64,
    pub word_margin: f64,
    pub column_gap_min: Option<f64>,
    pub row_gap_min: Option<f64>,
    pub full_width_threshold: f64,
    pub detect_vertical: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            char_margin: 2.0,
            line_overlap: 0.5,
            line_margin: 0.5,
            word_margin: 0.1,
            column_gap_min: None,
            row_gap_min: None,
            full_width_threshold: 0.9,
            detect_vertical: false,
        }
    }
}

#[pymethods]
impl Params {
    #[new]
    #[pyo3(signature = (
        char_margin=2.0, line_overlap=0.5, line_margin=0.5, word_margin=0.1,
        column_gap_min=None, row_gap_min=None, full_width_threshold=0.9,
        detect_vertical=false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn py_new(
        char_margin: f64,
        line_overlap: f64,
        line_margin: f64,
        word_margin: f64,
        column_gap_min: Option<f64>,
        row_gap_min: Option<f64>,
        full_width_threshold: f64,
        detect_vertical: bool,
    ) -> Self {
        Params {
            char_margin,
            line_overlap,
            line_margin,
            word_margin,
            column_gap_min,
            row_gap_min,
            full_width_threshold,
            detect_vertical,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Params;

    #[test]
    fn defaults_match_pdfminer_laparams_defaults() {
        let p = Params::default();
        assert_eq!(p.char_margin, 2.0);
        assert_eq!(p.line_overlap, 0.5);
        assert_eq!(p.line_margin, 0.5);
        assert_eq!(p.word_margin, 0.1);
        assert_eq!(p.column_gap_min, None);
        assert_eq!(p.row_gap_min, None);
        assert_eq!(p.full_width_threshold, 0.9);
        assert!(!p.detect_vertical);
    }

    #[test]
    fn py_new_assigns_all_fields_from_arguments() {
        let p = Params::py_new(1.0, 2.0, 3.0, 4.0, Some(5.0), Some(6.0), 7.0, true);
        assert_eq!(p.char_margin, 1.0);
        assert_eq!(p.line_overlap, 2.0);
        assert_eq!(p.line_margin, 3.0);
        assert_eq!(p.word_margin, 4.0);
        assert_eq!(p.column_gap_min, Some(5.0));
        assert_eq!(p.row_gap_min, Some(6.0));
        assert_eq!(p.full_width_threshold, 7.0);
        assert!(p.detect_vertical);
    }

    #[test]
    fn params_extracts_from_python_object() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let p = Params::default();
            let obj = pyo3::Py::new(py, p.clone()).unwrap();
            let extracted: Params = obj.extract(py).unwrap();
            assert_eq!(extracted, p);
        });
    }

    #[test]
    fn params_extraction_fails_for_wrong_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = PyString::new(py, "not params");
            let extracted: Result<Params, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }
}
