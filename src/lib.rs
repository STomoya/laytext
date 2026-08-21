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
    group_lines_impl(&chars, &params)
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
