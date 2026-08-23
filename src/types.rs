use pyo3::prelude::*;

use crate::geometry::Rect;

#[pyclass(get_all, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct FontInfo {
    pub name: Option<String>,
    pub size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

#[pymethods]
impl FontInfo {
    #[new]
    #[pyo3(signature = (name=None, size=None, bold=None, italic=None))]
    fn py_new(
        name: Option<String>,
        size: Option<f64>,
        bold: Option<bool>,
        italic: Option<bool>,
    ) -> Self {
        FontInfo {
            name,
            size,
            bold,
            italic,
        }
    }
}

#[pyclass(get_all, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct Char {
    pub bbox: Rect,
    pub text: char,
    pub font: Option<FontInfo>,
}

#[pymethods]
impl Char {
    #[new]
    #[pyo3(signature = (bbox, text, font=None))]
    fn py_new(bbox: Rect, text: char, font: Option<FontInfo>) -> Self {
        Char { bbox, text, font }
    }
}

#[pyclass(get_all, from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub bbox: Rect,
    pub upright: bool,
    pub chars: Vec<Char>,
}

#[pymethods]
impl Line {
    #[new]
    fn py_new(bbox: Rect, upright: bool, chars: Vec<Char>) -> Self {
        Line {
            bbox,
            upright,
            chars,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Char, FontInfo, Line};
    use crate::geometry::Rect;

    fn rect() -> Rect {
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
        }
    }

    #[test]
    fn font_info_py_new_assigns_all_fields_from_arguments() {
        let f = FontInfo::py_new(
            Some("Helvetica".to_string()),
            Some(10.0),
            Some(true),
            Some(false),
        );
        assert_eq!(f.name, Some("Helvetica".to_string()));
        assert_eq!(f.size, Some(10.0));
        assert_eq!(f.bold, Some(true));
        assert_eq!(f.italic, Some(false));
    }

    #[test]
    fn font_info_py_new_accepts_none_defaults() {
        let f = FontInfo::py_new(None, None, None, None);
        assert_eq!(
            f,
            FontInfo {
                name: None,
                size: None,
                bold: None,
                italic: None,
            }
        );
    }

    #[test]
    fn char_py_new_assigns_all_fields_from_arguments() {
        let font = FontInfo::py_new(Some("Helvetica".to_string()), None, None, None);
        let c = Char::py_new(rect(), 'a', Some(font.clone()));
        assert_eq!(c.bbox, rect());
        assert_eq!(c.text, 'a');
        assert_eq!(c.font, Some(font));
    }

    #[test]
    fn line_py_new_assigns_all_fields_from_arguments() {
        let c = Char::py_new(rect(), 'a', None);
        let l = Line::py_new(rect(), true, vec![c.clone()]);
        assert_eq!(l.bbox, rect());
        assert!(l.upright);
        assert_eq!(l.chars, vec![c]);
    }

    #[test]
    fn font_info_extracts_from_python_object() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let f = FontInfo::py_new(
                Some("Helvetica".to_string()),
                Some(10.0),
                Some(true),
                Some(false),
            );
            let obj = pyo3::Py::new(py, f.clone()).unwrap();
            let extracted: FontInfo = obj.extract(py).unwrap();
            assert_eq!(extracted, f);
        });
    }

    #[test]
    fn char_extracts_from_python_object() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let c = Char::py_new(rect(), 'a', None);
            let obj = pyo3::Py::new(py, c.clone()).unwrap();
            let extracted: Char = obj.extract(py).unwrap();
            assert_eq!(extracted, c);
        });
    }

    #[test]
    fn line_extracts_from_python_object() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let l = Line::py_new(rect(), true, vec![Char::py_new(rect(), 'a', None)]);
            let obj = pyo3::Py::new(py, l.clone()).unwrap();
            let extracted: Line = obj.extract(py).unwrap();
            assert_eq!(extracted, l);
        });
    }

    #[test]
    fn font_info_extraction_fails_for_wrong_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = PyString::new(py, "not a font info");
            let extracted: Result<FontInfo, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }

    #[test]
    fn char_extraction_fails_for_wrong_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = PyString::new(py, "not a char");
            let extracted: Result<Char, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }

    #[test]
    fn line_extraction_fails_for_wrong_python_type() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            use pyo3::types::{PyAnyMethods, PyString};
            let obj = PyString::new(py, "not a line");
            let extracted: Result<Line, _> = obj.extract();
            assert!(extracted.is_err());
        });
    }
}
