pub mod assemble;
pub mod blocks;
pub mod geometry;
pub mod lines;
pub mod params;
pub mod segmentation;
pub mod types;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use assemble::assemble as assemble_impl;
use geometry::Rect;
use lines::group_lines as group_lines_impl;
use params::Params;
use rayon::prelude::*;
use types::{Block, Char, FontInfo, Line, Page, PageInput};

#[pyfunction]
#[pyo3(name = "group_lines")]
fn group_lines_py(chars: Vec<Char>, params: Params) -> Vec<Line> {
    group_lines_impl(chars, &params)
}

#[pyfunction]
#[pyo3(name = "analyze_page")]
fn analyze_page_py(
    py: Python<'_>,
    chars: Vec<Char>,
    width: f64,
    height: f64,
    params: Params,
) -> PyResult<Page> {
    params.validate().map_err(PyValueError::new_err)?;
    Ok(py.detach(|| {
        let lines = group_lines_impl(chars, &params);
        assemble_impl(lines, &params, width, height)
    }))
}

#[pyfunction]
#[pyo3(name = "analyze_document")]
fn analyze_document_py(
    py: Python<'_>,
    pages: Vec<PageInput>,
    params: Params,
) -> PyResult<Vec<Page>> {
    params.validate().map_err(PyValueError::new_err)?;
    Ok(py.detach(|| {
        pages
            .into_par_iter()
            .map(|page| {
                let (width, height) = (page.width, page.height);
                let lines = group_lines_impl(page.chars, &params);
                assemble_impl(lines, &params, width, height)
            })
            .collect()
    }))
}

#[cfg(test)]
mod tests {
    use super::{analyze_document_py, analyze_page_py, group_lines_py};
    use crate::assemble::assemble;
    use crate::geometry::Rect;
    use crate::lines::group_lines;
    use crate::params::Params;
    use crate::types::{Char, PageInput};

    fn a_char() -> Char {
        Char {
            bbox: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 6.0,
                y1: 10.0,
            },
            text: 'a',
            font: None,
        }
    }

    #[test]
    fn group_lines_py_delegates_to_lines_group_lines() {
        let chars = vec![a_char()];
        let params = Params::default();
        let expected = group_lines(chars.clone(), &params);
        assert_eq!(group_lines_py(chars, params), expected);
    }

    #[test]
    fn analyze_page_py_delegates_to_group_lines_then_assemble() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let chars = vec![a_char()];
            let params = Params {
                column_gap_min: Some(10.0),
                row_gap_min: Some(10.0),
                ..Default::default()
            };
            let lines = group_lines(chars.clone(), &params);
            let expected = assemble(lines, &params, 100.0, 200.0);
            assert_eq!(
                analyze_page_py(py, chars, 100.0, 200.0, params).unwrap(),
                expected
            );
        });
    }

    #[test]
    fn analyze_page_py_rejects_missing_gap_params() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let result = analyze_page_py(py, vec![a_char()], 100.0, 200.0, Params::default());
            assert!(result.is_err());
        });
    }

    #[test]
    fn analyze_document_py_returns_one_page_per_input_page_in_order() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let params = Params {
                column_gap_min: Some(10.0),
                row_gap_min: Some(10.0),
                ..Default::default()
            };
            let pages = vec![
                PageInput {
                    width: 100.0,
                    height: 200.0,
                    chars: vec![a_char()],
                },
                PageInput {
                    width: 100.0,
                    height: 200.0,
                    chars: vec![],
                },
            ];
            let expected: Vec<_> = pages
                .iter()
                .cloned()
                .map(|p| analyze_page_py(py, p.chars, p.width, p.height, params.clone()).unwrap())
                .collect();

            assert_eq!(analyze_document_py(py, pages, params).unwrap(), expected);
        });
    }

    #[test]
    fn analyze_document_py_empty_batch_returns_empty_vec() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let params = Params {
                column_gap_min: Some(10.0),
                row_gap_min: Some(10.0),
                ..Default::default()
            };
            assert_eq!(analyze_document_py(py, vec![], params).unwrap(), vec![]);
        });
    }

    #[test]
    fn analyze_document_py_rejects_missing_gap_params() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let result = analyze_document_py(py, vec![], Params::default());
            assert!(result.is_err());
        });
    }
}

#[pymodule]
mod _core {
    #[pymodule_export]
    use crate::Block;
    #[pymodule_export]
    use crate::Char;
    #[pymodule_export]
    use crate::FontInfo;
    #[pymodule_export]
    use crate::Line;
    #[pymodule_export]
    use crate::Page;
    #[pymodule_export]
    use crate::PageInput;
    #[pymodule_export]
    use crate::Params;
    #[pymodule_export]
    use crate::Rect;
    #[pymodule_export]
    use crate::analyze_document_py;
    #[pymodule_export]
    use crate::analyze_page_py;
    #[pymodule_export]
    use crate::group_lines_py;
}
