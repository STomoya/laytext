pub mod assemble;
pub mod blocks;
pub mod geometry;
pub mod lines;
pub mod params;
pub mod segmentation;
pub mod types;

use pyo3::prelude::*;

use assemble::assemble as assemble_impl;
use geometry::Rect;
use lines::group_lines as group_lines_impl;
use params::Params;
use types::{Block, Char, FontInfo, Line, Page};

#[pyfunction]
#[pyo3(name = "group_lines")]
fn group_lines_py(chars: Vec<Char>, params: Params) -> Vec<Line> {
    group_lines_impl(chars, &params)
}

#[pyfunction]
#[pyo3(name = "analyze_page")]
fn analyze_page_py(chars: Vec<Char>, params: Params) -> Page {
    let lines = group_lines_impl(chars, &params);
    assemble_impl(lines, &params)
}

#[cfg(test)]
mod tests {
    use super::{analyze_page_py, group_lines_py};
    use crate::assemble::assemble;
    use crate::geometry::Rect;
    use crate::lines::group_lines;
    use crate::params::Params;
    use crate::types::Char;

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
        let chars = vec![a_char()];
        let params = Params {
            column_gap_min: Some(10.0),
            row_gap_min: Some(10.0),
            ..Default::default()
        };
        let lines = group_lines(chars.clone(), &params);
        let expected = assemble(lines, &params);
        assert_eq!(analyze_page_py(chars, params), expected);
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
    use crate::Params;
    #[pymodule_export]
    use crate::Rect;
    #[pymodule_export]
    use crate::analyze_page_py;
    #[pymodule_export]
    use crate::group_lines_py;
}
