use pyo3::prelude::*;

#[pyclass(get_all)]
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
        char_margin: f64, line_overlap: f64, line_margin: f64, word_margin: f64,
        column_gap_min: Option<f64>, row_gap_min: Option<f64>, full_width_threshold: f64,
        detect_vertical: bool,
    ) -> Self {
        Params {
            char_margin, line_overlap, line_margin, word_margin,
            column_gap_min, row_gap_min, full_width_threshold, detect_vertical,
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
        assert!(!p.detect_vertical);
    }
}
