pub mod geometry;
pub mod lines;
pub mod params;
pub mod types;

use pyo3::prelude::*;

use geometry::Rect;
use lines::group_lines as group_lines_impl;
use params::Params;
use types::{Char, FontInfo, Line};

#[pyfunction]
#[pyo3(name = "group_lines")]
fn group_lines_py(chars: Vec<Char>, params: Params) -> Vec<Line> {
    group_lines_impl(chars, &params)
}

#[cfg(test)]
mod tests {
    use super::group_lines_py;
    use crate::geometry::Rect;
    use crate::lines::group_lines;
    use crate::params::Params;
    use crate::types::Char;

    #[test]
    fn group_lines_py_delegates_to_lines_group_lines() {
        let chars = vec![Char {
            bbox: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 6.0,
                y1: 10.0,
            },
            text: 'a',
            font: None,
        }];
        let params = Params::default();
        let expected = group_lines(chars.clone(), &params);
        assert_eq!(group_lines_py(chars, params), expected);
    }
}

#[pymodule]
mod _core {
    #[pymodule_export]
    use crate::Char;
    #[pymodule_export]
    use crate::FontInfo;
    #[pymodule_export]
    use crate::Line;
    #[pymodule_export]
    use crate::Params;
    #[pymodule_export]
    use crate::Rect;
    #[pymodule_export]
    use crate::group_lines_py;
}
