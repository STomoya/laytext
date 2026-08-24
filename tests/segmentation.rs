use _core::geometry::Rect;
use _core::params::Params;
use _core::segmentation::{Orientation, Region, segment};
use _core::types::{Char, Line};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn line(bbox: Rect) -> Line {
    Line {
        bbox,
        upright: true,
        chars: vec![Char {
            bbox,
            text: 'x',
            font: None,
        }],
    }
}

fn params_with_gaps(column_gap_min: Option<f64>, row_gap_min: Option<f64>) -> Params {
    Params {
        column_gap_min,
        row_gap_min,
        ..Default::default()
    }
}

#[test]
fn empty_input_produces_an_empty_leaf() {
    let params = params_with_gaps(Some(5.0), Some(5.0));
    let region = segment(vec![], &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: rect(0.0, 0.0, 0.0, 0.0),
            lines: vec![],
        }
    );
}

#[test]
fn single_line_produces_a_leaf() {
    let params = params_with_gaps(Some(5.0), Some(5.0));
    let a = line(rect(0.0, 0.0, 100.0, 10.0));
    let region = segment(vec![a.clone()], &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: a.bbox,
            lines: vec![a],
        }
    );
}

#[test]
fn two_columns_with_a_clear_gap_split_vertically_left_to_right() {
    let params = params_with_gaps(Some(10.0), Some(10.0));
    let left = line(rect(0.0, 0.0, 100.0, 10.0));
    let right = line(rect(120.0, 0.0, 220.0, 10.0)); // 20pt gap: exceeds column_gap_min
    let region = segment(vec![left.clone(), right.clone()], &params);
    assert_eq!(
        region,
        Region::Split {
            bbox: rect(0.0, 0.0, 220.0, 10.0),
            orientation: Orientation::Vertical,
            children: vec![
                Region::Leaf {
                    bbox: left.bbox,
                    lines: vec![left],
                },
                Region::Leaf {
                    bbox: right.bbox,
                    lines: vec![right],
                },
            ],
        }
    );
}

#[test]
fn columns_with_gap_smaller_than_column_gap_min_stay_a_single_leaf() {
    let params = params_with_gaps(Some(30.0), Some(10.0));
    let left = line(rect(0.0, 0.0, 100.0, 10.0));
    let right = line(rect(105.0, 0.0, 205.0, 10.0)); // 5pt gap: below column_gap_min
    let region = segment(vec![left.clone(), right.clone()], &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: rect(0.0, 0.0, 205.0, 10.0),
            lines: vec![left, right],
        }
    );
}

#[test]
fn stacked_rows_with_a_clear_gap_split_horizontally_top_to_bottom() {
    let params = params_with_gaps(Some(10.0), Some(10.0));
    let top = line(rect(0.0, 120.0, 100.0, 130.0));
    let bottom = line(rect(0.0, 0.0, 100.0, 10.0)); // 110pt gap: exceeds row_gap_min
    let region = segment(vec![top.clone(), bottom.clone()], &params);
    assert_eq!(
        region,
        Region::Split {
            bbox: rect(0.0, 0.0, 100.0, 130.0),
            orientation: Orientation::Horizontal,
            children: vec![
                Region::Leaf {
                    bbox: top.bbox,
                    lines: vec![top],
                },
                Region::Leaf {
                    bbox: bottom.bbox,
                    lines: vec![bottom],
                },
            ],
        }
    );
}

#[test]
#[should_panic(expected = "column_gap_min")]
fn none_column_gap_min_panics() {
    let params = params_with_gaps(None, Some(10.0));
    let left = line(rect(0.0, 0.0, 100.0, 10.0));
    let right = line(rect(200.0, 0.0, 300.0, 10.0));
    segment(vec![left, right], &params);
}

#[test]
#[should_panic(expected = "row_gap_min")]
fn none_row_gap_min_panics() {
    let params = params_with_gaps(Some(10.0), None);
    let top = line(rect(0.0, 220.0, 100.0, 230.0));
    let bottom = line(rect(0.0, 0.0, 100.0, 10.0));
    segment(vec![top, bottom], &params);
}

#[test]
fn widest_gap_is_chosen_when_multiple_candidate_gaps_exist() {
    // Both gaps clear column_gap_min(5.0), so widest_gap must actually
    // compare them rather than just unwrapping a single candidate.
    let params = params_with_gaps(Some(5.0), Some(5.0));
    let a = line(rect(0.0, 0.0, 50.0, 10.0));
    let b = line(rect(60.0, 0.0, 100.0, 10.0)); // 10pt gap from a
    let c = line(rect(150.0, 0.0, 200.0, 10.0)); // 50pt gap from b: the widest
    let region = segment(vec![a.clone(), b.clone(), c.clone()], &params);
    match region {
        Region::Split {
            orientation: Orientation::Vertical,
            children,
            ..
        } => {
            assert_eq!(children.len(), 2);
            // the widest (b|c) gap is the top-level cut; the narrower (a|b)
            // gap still clears column_gap_min, so the left side recurses
            // into its own split.
            assert_eq!(
                children[0],
                Region::Split {
                    bbox: rect(0.0, 0.0, 100.0, 10.0),
                    orientation: Orientation::Vertical,
                    children: vec![
                        Region::Leaf {
                            bbox: a.bbox,
                            lines: vec![a],
                        },
                        Region::Leaf {
                            bbox: b.bbox,
                            lines: vec![b],
                        },
                    ],
                }
            );
            match &children[1] {
                Region::Leaf { lines, .. } => assert_eq!(lines, &vec![c]),
                _ => panic!("expected a leaf on the right of the widest gap"),
            }
        }
        other => panic!("expected a vertical split, got {other:?}"),
    }
}

#[test]
fn full_width_title_forces_a_horizontal_split_above_two_columns() {
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        ..Default::default()
    };
    // title spans the full width the two columns cover together (0..220)
    let title = line(rect(0.0, 120.0, 220.0, 140.0));
    let left = line(rect(0.0, 0.0, 100.0, 10.0));
    let right = line(rect(120.0, 0.0, 220.0, 10.0));
    let region = segment(vec![title.clone(), left.clone(), right.clone()], &params);
    match region {
        Region::Split {
            orientation: Orientation::Horizontal,
            children,
            ..
        } => {
            assert_eq!(children.len(), 2);
            assert_eq!(
                children[0],
                Region::Leaf {
                    bbox: title.bbox,
                    lines: vec![title],
                }
            );
            assert_eq!(
                children[1],
                Region::Split {
                    bbox: rect(0.0, 0.0, 220.0, 10.0),
                    orientation: Orientation::Vertical,
                    children: vec![
                        Region::Leaf {
                            bbox: left.bbox,
                            lines: vec![left],
                        },
                        Region::Leaf {
                            bbox: right.bbox,
                            lines: vec![right],
                        },
                    ],
                }
            );
        }
        other => panic!("expected a horizontal split, got {other:?}"),
    }
}

#[test]
fn ordinary_paragraph_with_a_short_wrapped_last_line_is_not_split() {
    // regression: full-width lines outnumbering the wrapped last line must
    // not force a split (see full_width_title_... for the real title case).
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        ..Default::default()
    };
    let l1 = line(rect(0.0, 30.0, 395.0, 40.0));
    let l2 = line(rect(0.0, 18.0, 395.0, 28.0));
    let l3 = line(rect(0.0, 6.0, 395.0, 16.0));
    let l4 = line(rect(0.0, -6.0, 150.0, 4.0));
    let region = segment(vec![l1.clone(), l2.clone(), l3.clone(), l4.clone()], &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: rect(0.0, -6.0, 395.0, 40.0),
            lines: vec![l1, l2, l3, l4],
        }
    );
}

#[test]
fn consecutive_full_width_lines_band_together() {
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        ..Default::default()
    };
    // two title lines, close together (1pt gap, below row_gap_min), band into one leaf
    let title1 = line(rect(0.0, 132.0, 220.0, 140.0));
    let title2 = line(rect(0.0, 120.0, 220.0, 131.0));
    let left = line(rect(0.0, 0.0, 100.0, 10.0));
    let right = line(rect(120.0, 0.0, 220.0, 10.0));
    let region = segment(
        vec![title1.clone(), title2.clone(), left.clone(), right.clone()],
        &params,
    );
    match region {
        Region::Split {
            orientation: Orientation::Horizontal,
            children,
            ..
        } => {
            assert_eq!(children.len(), 2);
            assert_eq!(
                children[0],
                Region::Leaf {
                    bbox: rect(0.0, 120.0, 220.0, 140.0),
                    lines: vec![title1, title2],
                }
            );
        }
        other => panic!("expected a horizontal split, got {other:?}"),
    }
}
