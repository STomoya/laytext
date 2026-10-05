use _core::geometry::Rect;
use _core::geometry::union_all;
use _core::lines::group_lines;
use _core::params::Params;
use _core::skew::estimate_page_skew;
use _core::types::Char;

fn ch(x0: f64, y0: f64, x1: f64, y1: f64) -> Char {
    Char {
        bbox: Rect { x0, y0, x1, y1 },
        text: 'x',
        font: None,
    }
}

#[test]
fn empty_input_produces_no_lines() {
    let params = Params::default();
    assert_eq!(group_lines(vec![], &params), vec![]);
}

#[test]
fn single_char_produces_single_line() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let lines = group_lines(vec![a.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![a]);
    assert!(lines[0].upright);
}

#[test]
fn close_chars_on_same_row_merge_into_one_line() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(6.5, 0.0, 12.5, 10.0); // 0.5pt gap: well within char_margin, within word_margin
    let lines = group_lines(vec![a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![a, b]);
    assert_eq!(
        lines[0].bbox,
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 12.5,
            y1: 10.0
        }
    );
}

#[test]
fn single_char_line_has_confidence_one() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let lines = group_lines(vec![a], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].confidence, 1.0);
}

#[test]
fn line_confidence_is_maximal_when_chars_touch_exactly() {
    let params = Params {
        char_margin: 2.0,
        word_margin: 0.0,
        ..Params::default()
    };
    let a = ch(0.0, 0.0, 10.0, 10.0);
    let b = ch(10.0, 0.0, 20.0, 10.0); // touching: hdistance = 0
    let lines = group_lines(vec![a, b], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].confidence, 1.0);
}

#[test]
fn line_confidence_is_low_when_a_char_barely_clears_char_margin() {
    let params = Params {
        char_margin: 2.0,
        word_margin: 0.0,
        ..Params::default()
    };
    // threshold d = width.max(10) * char_margin(2.0) = 20; hdistance = 19,
    // just under the threshold: ratio = 1.0 - 19.0/20.0 = 0.05.
    let a = ch(0.0, 0.0, 10.0, 10.0);
    let b = ch(29.0, 0.0, 39.0, 10.0);
    let lines = group_lines(vec![a, b], &params);
    assert_eq!(lines.len(), 1);
    assert!((lines[0].confidence - 0.05).abs() < 1e-9);
}

#[test]
fn line_confidence_reflects_the_weakest_merge_in_a_multi_char_line() {
    let params = Params {
        char_margin: 2.0,
        word_margin: 0.0,
        ..Params::default()
    };
    // a-b touch exactly (ratio 1.0); b-c barely clears char_margin (ratio
    // 0.05). The line's overall confidence must be the minimum across all
    // its merges, not an average.
    let a = ch(0.0, 0.0, 10.0, 10.0);
    let b = ch(10.0, 0.0, 20.0, 10.0);
    let c = ch(39.0, 0.0, 49.0, 10.0);
    let lines = group_lines(vec![a, b, c], &params);
    assert_eq!(lines.len(), 1);
    assert!((lines[0].confidence - 0.05).abs() < 1e-9);
}

#[test]
fn far_apart_chars_split_into_separate_lines() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(21.0, 0.0, 27.0, 10.0); // 15pt gap: exceeds char_margin(2.0) * width(6) = 12
    let lines = group_lines(vec![a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].chars, vec![a]);
    assert_eq!(lines[1].chars, vec![b]);
}

#[test]
fn wide_but_mergeable_gap_inserts_word_margin_space() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(9.0, 0.0, 15.0, 10.0); // 3pt gap: > word_margin(0.1)*10=1.0, < char_margin(2.0)*6=12
    let lines = group_lines(vec![a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars.len(), 3);
    assert_eq!(lines[0].chars[0], a);
    assert_eq!(lines[0].chars[1].text, ' ');
    assert_eq!(lines[0].chars[2], b);
    // the synthetic space must not widen the line's bbox beyond the real chars
    assert_eq!(
        lines[0].bbox,
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 15.0,
            y1: 10.0
        }
    );
}

#[test]
fn zero_word_margin_disables_space_insertion() {
    let params = Params {
        word_margin: 0.0,
        ..Default::default()
    };
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(9.0, 0.0, 15.0, 10.0); // same gap as the word-margin-space test above
    let lines = group_lines(vec![a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![a, b]);
}

#[test]
fn negative_word_margin_still_inserts_space_matching_python_truthiness() {
    // pdfminer gates word-margin space insertion on `if self.word_margin:`,
    // which is true for any nonzero value, including negative ones.
    let params = Params {
        word_margin: -0.1,
        ..Default::default()
    };
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(9.0, 0.0, 15.0, 10.0); // same gap as wide_but_mergeable_gap_inserts_word_margin_space
    let lines = group_lines(vec![a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars.len(), 3);
    assert_eq!(lines[0].chars[1].text, ' ');
}

#[test]
fn vertical_text_grouped_only_when_detect_vertical_enabled() {
    let mut params = Params::default();
    let top = ch(0.0, 10.0, 6.0, 20.0);
    let bottom = ch(0.0, 1.5, 6.0, 9.5); // stacked, horizontally aligned, 0.5pt vertical gap

    params.detect_vertical = false;
    let lines = group_lines(vec![top.clone(), bottom.clone()], &params);
    assert_eq!(lines.len(), 2, "vertical grouping must be off by default");

    params.detect_vertical = true;
    let lines = group_lines(vec![top.clone(), bottom.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![top, bottom]);
    assert!(!lines[0].upright);
}

#[test]
fn three_close_chars_extend_an_already_open_horizontal_line() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(6.5, 0.0, 12.5, 10.0);
    let c = ch(13.0, 0.0, 19.0, 10.0);
    let lines = group_lines(vec![a.clone(), b.clone(), c.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![a, b, c]);
}

#[test]
fn three_close_chars_extend_an_already_open_vertical_line() {
    let params = Params {
        detect_vertical: true,
        ..Default::default()
    };
    let top = ch(0.0, 20.0, 6.0, 30.0);
    let mid = ch(0.0, 10.5, 6.0, 19.5);
    let bottom = ch(0.0, 0.0, 6.0, 10.0);
    let lines = group_lines(vec![top.clone(), mid.clone(), bottom.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![top, mid, bottom]);
}

#[test]
fn vertical_wide_but_mergeable_gap_inserts_word_margin_space() {
    let params = Params {
        detect_vertical: true,
        ..Default::default()
    };
    let top = ch(0.0, 20.0, 6.0, 30.0);
    let bottom = ch(0.0, 5.0, 6.0, 15.0); // 5pt gap: > word_margin(0.1)*10=1.0, < char_margin(2.0)*10=20
    let lines = group_lines(vec![top.clone(), bottom.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars.len(), 3);
    assert_eq!(lines[0].chars[0], top);
    assert_eq!(lines[0].chars[1].text, ' ');
    assert_eq!(lines[0].chars[2], bottom);
}

#[test]
fn orientation_mismatch_closes_open_line_without_extending_it() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(6.5, 0.0, 12.5, 10.0); // opens a horizontal line with a
    let c = ch(6.5, -6.0, 12.5, -1.0); // below b, not horizontally aligned with it
    let lines = group_lines(vec![a.clone(), b.clone(), c.clone()], &params);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].chars, vec![a, b]);
    assert_eq!(lines[1].chars, vec![c]);
}

#[test]
fn multiple_lines_preserve_order_across_a_run_of_chars() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(21.0, 0.0, 27.0, 10.0); // far from a: breaks the line
    let c = ch(27.5, 0.0, 33.5, 10.0); // close to b: merges with b
    let lines = group_lines(vec![a.clone(), b.clone(), c.clone()], &params);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].chars, vec![a]);
    assert_eq!(lines[1].chars, vec![b, c]);
}

/// One line of `n` dense 6x10 chars at a 7pt pitch starting at `x_start`,
/// tilted by `angle_deg` (each char's y0 rises by `x_center * tan(angle)`).
fn tilted_run(baseline: f64, x_start: f64, n: usize, angle_deg: f64) -> Vec<Char> {
    let slope = angle_deg.to_radians().tan();
    (0..n)
        .map(|i| {
            let x0 = x_start + (i as f64) * 7.0;
            let y0 = baseline + (x0 + 3.0) * slope;
            ch(x0, y0, x0 + 6.0, y0 + 10.0)
        })
        .collect()
}

/// `lines` dense 40-char lines at a 12pt (1.2x char height) baseline
/// pitch, in reading order — tight enough that adjacent lines' y ranges
/// interleave once tilted.
fn dense_page(lines: usize, angle_deg: f64) -> Vec<Char> {
    (0..lines)
        .flat_map(|l| tilted_run(500.0 - (l as f64) * 12.0, 0.0, 40, angle_deg))
        .collect()
}

#[test]
fn estimate_page_skew_returns_zero_for_empty_input() {
    assert_eq!(estimate_page_skew(&[]), 0.0);
}

#[test]
fn estimate_page_skew_returns_zero_when_too_little_text_to_estimate_confidently() {
    // Two runs, under the 3-run minimum for a confident page estimate.
    assert_eq!(estimate_page_skew(&dense_page(2, 2.0)), 0.0);
}

#[test]
fn estimate_page_skew_is_zero_for_axis_aligned_text() {
    assert!(estimate_page_skew(&dense_page(5, 0.0)).abs() < 1e-9);
}

#[test]
fn estimate_page_skew_measures_a_dense_tilted_page() {
    assert!((estimate_page_skew(&dense_page(5, 2.0)) - 2.0).abs() < 1e-6);
    assert!((estimate_page_skew(&dense_page(5, -1.0)) + 1.0).abs() < 1e-6);
}

#[test]
fn estimate_page_skew_robust_to_a_single_outlier_run() {
    let mut chars = dense_page(5, 0.0);
    chars.extend(tilted_run(0.0, 0.0, 40, 10.0));
    assert!(estimate_page_skew(&chars).abs() < 1e-9);
}

#[test]
fn estimate_page_skew_ignores_short_runs() {
    // Short runs (table cells, numbers) are dominated by per-glyph box
    // differences, not page tilt: on the real corpus, rows of tilted-
    // looking 8-char cells drove a flat page's estimate to ~2deg. Only
    // runs spanning at least 10 char heights count.
    let mut chars = dense_page(3, 0.0);
    for cell in 0..5 {
        chars.extend(tilted_run(200.0 - (cell as f64) * 30.0, 0.0, 8, 2.0));
    }
    assert!(estimate_page_skew(&chars).abs() < 1e-9);
}

#[test]
fn estimate_page_skew_ignores_steep_runs() {
    // Beyond 15deg a run is rotated text (axis labels, stamps), not scan
    // skew; a page of only such runs has no estimate.
    assert_eq!(estimate_page_skew(&dense_page(4, 20.0)), 0.0);
}

#[test]
fn estimate_page_skew_ignores_vertical_text() {
    // Chars stacked top-to-bottom never form a left-to-right run.
    let chars: Vec<Char> = (0..4)
        .flat_map(|col| {
            (0..40).map(move |i| {
                let x0 = 500.0 - (col as f64) * 14.0;
                let y0 = 500.0 - (i as f64) * 11.0;
                ch(x0, y0, x0 + 10.0, y0 + 10.0)
            })
        })
        .collect();
    assert_eq!(estimate_page_skew(&chars), 0.0);
}

#[test]
fn estimate_page_skew_nan_bbox_does_not_panic() {
    let mut chars = dense_page(5, 2.0);
    chars[3].bbox.y0 = f64::NAN;
    let _ = estimate_page_skew(&chars); // must not panic
}

/// Three 2deg-tilted lines, each two 20-char runs separated by a wide
/// (~260pt) gap. Within a run the per-char drift is tiny, but across the
/// gap it is ~9.3pt, which fails halign's voverlap check (> height(10) *
/// (1 - line_overlap(0.5)) = 5) without correction and splits every line
/// in two. A generous char_margin keeps the horizontal-gap check passing
/// either way, and word_margin=0 avoids synthetic spaces across the gap.
fn gapped_tilted_lines() -> (Params, Vec<Vec<Char>>) {
    let params = Params {
        char_margin: 50.0,
        word_margin: 0.0,
        ..Params::default()
    };
    let lines = [500.0, 470.0, 440.0]
        .into_iter()
        .map(|baseline| {
            let mut line = tilted_run(baseline, 0.0, 20, 2.0);
            line.extend(tilted_run(baseline, 400.0, 20, 2.0));
            line
        })
        .collect();
    (params, lines)
}

#[test]
fn skewed_lines_group_correctly_and_output_geometry_is_never_sheared() {
    let (params, expected) = gapped_tilted_lines();
    let params = Params {
        deskew: true,
        ..params
    };
    let chars: Vec<Char> = expected.concat();

    let lines = group_lines(chars, &params);

    assert_eq!(lines.len(), 3);
    for (line, expected_line) in lines.iter().zip(&expected) {
        assert_eq!(&line.chars, expected_line);
        assert_eq!(line.bbox, union_all(expected_line.iter().map(|c| c.bbox)));
    }
}

#[test]
fn skew_correction_is_off_by_default() {
    let (params, expected) = gapped_tilted_lines();
    let chars: Vec<Char> = expected.concat();

    assert!((estimate_page_skew(&chars) - 2.0).abs() < 1e-6);
    assert_eq!(group_lines(chars, &params).len(), 6);
}

#[test]
fn zero_skew_page_produces_unchanged_grouping_output() {
    // Perfectly flat text with deskew on: the estimate is 0.0, under the
    // noise floor, so group_lines must behave exactly as with deskew off.
    let params = Params {
        deskew: true,
        ..Params::default()
    };
    let chars = dense_page(3, 0.0);
    let lines = group_lines(chars.clone(), &params);
    assert_eq!(lines.len(), 3);
    for (line, band_chars) in lines.iter().zip(chars.chunks(40)) {
        assert_eq!(line.chars, band_chars.to_vec());
    }
}
