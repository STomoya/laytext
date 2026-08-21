use pyo3::prelude::*;

use crate::geometry::Rect;

#[pyclass(get_all)]
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
    fn py_new(name: Option<String>, size: Option<f64>, bold: Option<bool>, italic: Option<bool>) -> Self {
        FontInfo { name, size, bold, italic }
    }
}

#[pyclass(get_all)]
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

#[pyclass(get_all)]
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
        Line { bbox, upright, chars }
    }
}
