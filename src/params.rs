use pyo3::prelude::*;

#[pyclass(eq, eq_int, from_py_object)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Strategy {
    #[default]
    Pdfminer,
    XyCut,
}

#[pyclass(get_all, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    pub char_margin: f64,
    pub line_overlap: f64,
    pub line_margin: f64,
    pub word_margin: f64,
    /// XyCut-strategy only; unused by the default Pdfminer strategy.
    pub column_gap_min: Option<f64>,
    /// XyCut-strategy only; unused by the default Pdfminer strategy.
    pub row_gap_min: Option<f64>,
    /// XyCut-strategy only; unused by the default Pdfminer strategy.
    pub full_width_threshold: f64,
    pub detect_vertical: bool,
    pub segmentation: Strategy,
    pub boxes_flow: f64,
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
            segmentation: Strategy::Pdfminer,
            boxes_flow: 0.5,
        }
    }
}

#[pymethods]
impl Params {
    #[new]
    #[pyo3(signature = (
        char_margin=2.0, line_overlap=0.5, line_margin=0.5, word_margin=0.1,
        column_gap_min=None, row_gap_min=None, full_width_threshold=0.9,
        detect_vertical=false, segmentation=Strategy::Pdfminer, boxes_flow=0.5,
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
        segmentation: Strategy,
        boxes_flow: f64,
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
            segmentation,
            boxes_flow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Params, Strategy};

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
        assert_eq!(p.segmentation, Strategy::Pdfminer);
        assert_eq!(p.boxes_flow, 0.5);
    }

    #[test]
    fn py_new_assigns_all_fields_from_arguments() {
        let p = Params::py_new(
            1.0,
            2.0,
            3.0,
            4.0,
            Some(5.0),
            Some(6.0),
            7.0,
            true,
            Strategy::XyCut,
            0.25,
        );
        assert_eq!(p.char_margin, 1.0);
        assert_eq!(p.line_overlap, 2.0);
        assert_eq!(p.line_margin, 3.0);
        assert_eq!(p.word_margin, 4.0);
        assert_eq!(p.column_gap_min, Some(5.0));
        assert_eq!(p.row_gap_min, Some(6.0));
        assert_eq!(p.full_width_threshold, 7.0);
        assert!(p.detect_vertical);
        assert_eq!(p.segmentation, Strategy::XyCut);
        assert_eq!(p.boxes_flow, 0.25);
    }

    #[test]
    fn strategy_default_is_pdfminer() {
        assert_eq!(Strategy::default(), Strategy::Pdfminer);
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

    #[test]
    fn strategy_variants_compare_via_pyo3_eq() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let a = pyo3::Py::new(py, Strategy::Pdfminer).unwrap();
            let b = pyo3::Py::new(py, Strategy::XyCut).unwrap();
            let c = pyo3::Py::new(py, Strategy::Pdfminer).unwrap();
            use pyo3::types::PyAnyMethods;
            assert!(a.bind(py).as_any().eq(c.bind(py)).unwrap());
            assert!(!a.bind(py).as_any().eq(b.bind(py)).unwrap());
        });
    }

    #[test]
    fn strategy_extracts_from_python_object() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let obj = pyo3::Py::new(py, Strategy::XyCut).unwrap();
            let extracted: Strategy = obj.extract(py).unwrap();
            assert_eq!(extracted, Strategy::XyCut);
        });
    }

    #[test]
    fn strategy_extraction_fails_for_wrong_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = PyString::new(py, "not a strategy");
            let extracted: Result<Strategy, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }

    #[test]
    fn strategy_compares_equal_to_its_raw_int_value() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::PyAnyMethods;
            use pyo3::IntoPyObject;
            let obj = pyo3::Py::new(py, Strategy::XyCut).unwrap();
            let one = 1i64.into_pyobject(py).unwrap();
            assert!(obj.bind(py).as_any().eq(one).unwrap());
        });
    }

    #[test]
    fn strategy_repr_and_int_reported_via_python_builtins() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyDict};
            let obj = pyo3::Py::new(py, Strategy::XyCut).unwrap();
            let globals = PyDict::new(py);
            globals.set_item("s", obj).unwrap();
            let result = py
                .eval(
                    std::ffi::CStr::from_bytes_with_nul(b"(repr(s), int(s))\0").unwrap(),
                    Some(&globals),
                    None,
                )
                .unwrap();
            let (repr, as_int): (String, i64) = result.extract().unwrap();
            assert!(repr.contains("XyCut"));
            assert_eq!(as_int, 1);
        });
    }

    #[test]
    fn strategy_compares_unequal_to_an_incompatible_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = pyo3::Py::new(py, Strategy::XyCut).unwrap();
            let s = PyString::new(py, "not a strategy");
            assert!(!obj.bind(py).as_any().eq(s).unwrap());
        });
    }
}
