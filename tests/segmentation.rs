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
fn none_column_gap_min_derives_threshold_from_median_line_height_and_splits() {
    // line height is 10.0 everywhere here, so the derived threshold is 20.0.
    let params = params_with_gaps(None, Some(10.0));
    let left = line(rect(0.0, 0.0, 100.0, 10.0));
    let right = line(rect(125.0, 0.0, 225.0, 10.0)); // 25pt gap: exceeds derived 20.0
    let region = segment(vec![left.clone(), right.clone()], &params);
    assert_eq!(
        region,
        Region::Split {
            bbox: rect(0.0, 0.0, 225.0, 10.0),
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
fn none_column_gap_min_derived_threshold_keeps_narrower_gap_a_single_leaf() {
    // line height is 10.0 everywhere here, so the derived threshold is 20.0.
    let params = params_with_gaps(None, Some(10.0));
    let left = line(rect(0.0, 0.0, 100.0, 10.0));
    let right = line(rect(115.0, 0.0, 215.0, 10.0)); // 15pt gap: below derived 20.0
    let region = segment(vec![left.clone(), right.clone()], &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: rect(0.0, 0.0, 215.0, 10.0),
            lines: vec![left, right],
        }
    );
}

#[test]
fn none_row_gap_min_derives_threshold_from_median_line_height_and_splits() {
    // line height is 10.0 everywhere here, so the derived threshold is 15.0.
    let params = params_with_gaps(Some(1000.0), None); // column cut disabled
    let top = line(rect(0.0, 120.0, 100.0, 130.0));
    let bottom = line(rect(0.0, 0.0, 100.0, 10.0)); // 110pt gap: exceeds derived 15.0
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
    let region = segment(
        vec![l1.clone(), l2.clone(), l3.clone(), l4.clone()],
        &params,
    );
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

#[test]
fn alternating_full_and_narrow_lines_within_one_paragraph_are_not_split() {
    // regression: a justified paragraph whose wrapped line widths happen to
    // straddle full_width_threshold line-by-line (common in real body text)
    // must not be torn into a chain of single-line bands. full_count(3) <=
    // narrow_count(4) still passes the majority guard, but banding these
    // produces 7 alternating one-line bands, not the 2-band title/body
    // shape every other force-split case here produces.
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        ..Default::default()
    };
    // all left-aligned at x0=0.0 (no column gap), stacked with 1pt gaps
    // (below row_gap_min) so only the full-width heuristic is in play.
    // width 95 >= 0.9*95 -> "full"; width 80 < 85.5 -> "narrow".
    let l1 = line(rect(0.0, 60.0, 80.0, 70.0)); // narrow
    let l2 = line(rect(0.0, 49.0, 95.0, 59.0)); // full
    let l3 = line(rect(0.0, 38.0, 80.0, 48.0)); // narrow
    let l4 = line(rect(0.0, 27.0, 95.0, 37.0)); // full
    let l5 = line(rect(0.0, 16.0, 80.0, 26.0)); // narrow
    let l6 = line(rect(0.0, 5.0, 95.0, 15.0)); // full
    let l7 = line(rect(0.0, -6.0, 80.0, 4.0)); // narrow
    let lines = vec![
        l1.clone(),
        l2.clone(),
        l3.clone(),
        l4.clone(),
        l5.clone(),
        l6.clone(),
        l7.clone(),
    ];
    let region = segment(lines.clone(), &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: rect(0.0, -6.0, 95.0, 70.0),
            lines,
        }
    );
}

#[test]
fn repeated_heading_and_body_pairs_are_not_force_split() {
    // regression: a document with several short inline headings, each
    // immediately followed by its own multi-line paragraph (e.g. structured
    // abstracts: 目的/方法/結果/考察), bands into a long alternating chain
    // (narrow heading, full body, narrow heading, full body, ...) where the
    // body bands aren't singletons, so the pre-existing "all bands are
    // singleton" guard doesn't suppress it - each heading was getting torn
    // away from its own paragraph into a separate region. A real title/
    // body/footer page bands into 3 groups (see
    // title_two_column_body_and_footer_still_splits_into_three_bands); a
    // long repeating chain like this one is the same paragraph-noise
    // signature as the single-line-chain case, just with wider bands, so it
    // must also fall through undisturbed to the gap-based cuts (or a leaf).
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        ..Default::default()
    };
    // all left-aligned at x0=0.0, stacked with 1pt gaps (below row_gap_min).
    // width 95 >= 0.9*95 -> "full"; width 40 < 85.5 -> "narrow". Each
    // repeat is heading(narrow) + 2 full-width lines + a wrapped narrow
    // last line, keeping full_count == narrow_count (6 each) so the
    // majority guard doesn't reject it before banding is even tried.
    let heading1 = line(rect(0.0, 100.0, 40.0, 109.0));
    let body1a = line(rect(0.0, 90.0, 95.0, 99.0));
    let body1b = line(rect(0.0, 80.0, 95.0, 89.0));
    let last1 = line(rect(0.0, 70.0, 40.0, 79.0));
    let heading2 = line(rect(0.0, 60.0, 40.0, 69.0));
    let body2a = line(rect(0.0, 50.0, 95.0, 59.0));
    let body2b = line(rect(0.0, 40.0, 95.0, 49.0));
    let last2 = line(rect(0.0, 30.0, 40.0, 39.0));
    let heading3 = line(rect(0.0, 20.0, 40.0, 29.0));
    let body3a = line(rect(0.0, 10.0, 95.0, 19.0));
    let body3b = line(rect(0.0, 0.0, 95.0, 9.0));
    let last3 = line(rect(0.0, -10.0, 40.0, -1.0));
    let lines = vec![
        heading1.clone(),
        body1a.clone(),
        body1b.clone(),
        last1.clone(),
        heading2.clone(),
        body2a.clone(),
        body2b.clone(),
        last2.clone(),
        heading3.clone(),
        body3a.clone(),
        body3b.clone(),
        last3.clone(),
    ];
    let region = segment(lines.clone(), &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: rect(0.0, -10.0, 95.0, 109.0),
            lines,
        }
    );
}

#[test]
fn two_line_justified_paragraph_is_not_split() {
    // regression: bands.len() == 2 (one full-width line, one narrower
    // wrapped line) is the most common paragraph shape and must not be
    // force-split just because both bands happen to be singletons.
    let params = Params {
        column_gap_min: Some(10.0),
        row_gap_min: Some(10.0),
        full_width_threshold: 0.9,
        ..Default::default()
    };
    let l1 = line(rect(0.0, 6.0, 395.0, 16.0)); // full width
    let l2 = line(rect(0.0, -6.0, 150.0, 4.0)); // narrow, 2pt gap (below row_gap_min)
    let region = segment(vec![l1.clone(), l2.clone()], &params);
    assert_eq!(
        region,
        Region::Leaf {
            bbox: rect(0.0, -6.0, 395.0, 16.0),
            lines: vec![l1, l2],
        }
    );
}

#[test]
fn title_two_column_body_and_footer_still_splits_into_three_bands() {
    // regression: bands.len() != 2 must not reject every band count above
    // 2 - only the all-singleton-band chain that signals paragraph noise
    // (see alternating_full_and_narrow_lines_...). A real title/body/footer
    // page bands into 3 groups (title, [left,right], footer), and the
    // middle band has more than one line, so this must still force-split;
    // otherwise the title/footer's full-width bboxes swallow the column
    // gap in try_axis_cut_x and the whole page collapses into one leaf,
    // merging the left and right columns together.
    let params = Params {
        column_gap_min: Some(15.0),
        row_gap_min: Some(20.0), // wider than any band-to-band gap below
        full_width_threshold: 0.9,
        ..Default::default()
    };
    let title = line(rect(0.0, 15.0, 220.0, 25.0)); // full width, 5pt gap to body
    let left = line(rect(0.0, 0.0, 100.0, 10.0)); // narrow, 20pt gap to right
    let right = line(rect(120.0, 0.0, 220.0, 10.0)); // narrow, 12pt gap to footer
    let footer = line(rect(0.0, -12.0, 220.0, -3.0)); // full width
    let region = segment(
        vec![title.clone(), left.clone(), right.clone(), footer.clone()],
        &params,
    );
    assert_eq!(
        region,
        Region::Split {
            bbox: rect(0.0, -12.0, 220.0, 25.0),
            orientation: Orientation::Horizontal,
            children: vec![
                Region::Leaf {
                    bbox: title.bbox,
                    lines: vec![title],
                },
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
                },
                Region::Leaf {
                    bbox: footer.bbox,
                    lines: vec![footer],
                },
            ],
        }
    );
}
