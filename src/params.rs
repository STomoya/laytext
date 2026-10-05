use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Plain Rust enum, not a `#[pyclass]`: pyo3-macros-backend 0.28's protocol-
/// slot wrapper codegen (`generate_type_slot` in `pymethod.rs`) emits the
/// FFI trampoline for any `#[pymethods]` `__int__`/`__repr__` with an
/// unspanned `quote!`, so its coverage region always maps to the enclosing
/// attribute rather than the body — permanently unreachable-looking to
/// `cargo llvm-cov` regardless of who writes the method. The Python-facing
/// `Strategy` enum (with real `str()`/`repr()` support) is instead a plain
/// `enum.StrEnum` in `python/laytext/__init__.py`; this type only needs to
/// convert to/from a plain string at the FFI boundary, which uses pyo3's own
/// already-correct built-in string conversion and no macro codegen of ours.
///
/// Deliberately string-keyed rather than ordinal/int-keyed: an ordinal
/// mapping (`Pdfminer = 0`, `XyCut = 1`) silently breaks if a future
/// variant's Rust declaration order and Python value ever drift apart.
/// A name mismatch here fails loudly (`ValueError`) instead of silently
/// picking the wrong strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strategy {
    #[default]
    Pdfminer,
    XyCut,
}

impl Strategy {
    fn as_str(self) -> &'static str {
        match self {
            Strategy::Pdfminer => "pdfminer",
            Strategy::XyCut => "xycut",
        }
    }
}

impl<'py> IntoPyObject<'py> for Strategy {
    type Target = <&'static str as IntoPyObject<'py>>::Target;
    type Output = <&'static str as IntoPyObject<'py>>::Output;
    type Error = <&'static str as IntoPyObject<'py>>::Error;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        self.as_str().into_pyobject(py)
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for Strategy {
    type Error = PyErr;

    fn extract(obj: pyo3::Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let s: &str = obj.extract()?;
        match s {
            "pdfminer" => Ok(Strategy::Pdfminer),
            "xycut" => Ok(Strategy::XyCut),
            other => Err(PyValueError::new_err(format!(
                "{other:?} is not a valid Strategy"
            ))),
        }
    }
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
    /// Opt-in: make line-grouping and block-merge decisions on bboxes
    /// shear-corrected by the estimated page skew (output geometry is never
    /// corrected). Off by default: on skewed pages it deliberately diverges
    /// from pdfminer, the default path's reference.
    pub deskew: bool,
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
            deskew: false,
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
        deskew=false,
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
        deskew: bool,
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
            deskew,
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
        assert!(!p.deskew);
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
            true,
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
        assert!(p.deskew);
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
    fn strategy_into_pyobject_returns_the_expected_str_value() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::IntoPyObject;
            use pyo3::types::PyAnyMethods;
            let pdfminer = Strategy::Pdfminer.into_pyobject(py).unwrap();
            let xycut = Strategy::XyCut.into_pyobject(py).unwrap();
            assert_eq!(pdfminer.extract::<String>().unwrap(), "pdfminer");
            assert_eq!(xycut.extract::<String>().unwrap(), "xycut");
        });
    }

    #[test]
    fn strategy_extracts_from_a_python_str() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let pdfminer = PyString::new(py, "pdfminer");
            let xycut = PyString::new(py, "xycut");
            assert_eq!(pdfminer.extract::<Strategy>().unwrap(), Strategy::Pdfminer);
            assert_eq!(xycut.extract::<Strategy>().unwrap(), Strategy::XyCut);
        });
    }

    #[test]
    fn strategy_extraction_fails_for_a_str_with_no_matching_variant() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = PyString::new(py, "not a strategy");
            let extracted: Result<Strategy, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }

    #[test]
    fn strategy_extraction_fails_for_wrong_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::IntoPyObject;
            use pyo3::types::PyAnyMethods;
            let obj = 1i64.into_pyobject(py).unwrap();
            let extracted: Result<Strategy, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }
}
