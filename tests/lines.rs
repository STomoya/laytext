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

#[test]
fn estimate_page_skew_returns_zero_for_empty_input() {
    assert_eq!(estimate_page_skew(&[]), 0.0);
}

#[test]
fn estimate_page_skew_returns_zero_when_too_little_text_to_estimate_confidently() {
    // A single 4-char band (one band, under the 3-band minimum for a
    // confident page-level estimate) must not produce a guess from noise.
    let chars = vec![
        ch(0.0, 100.0, 6.0, 110.0),
        ch(20.0, 100.0, 26.0, 110.0),
        ch(40.0, 100.0, 46.0, 110.0),
        ch(60.0, 100.0, 66.0, 110.0),
    ];
    assert_eq!(estimate_page_skew(&chars), 0.0);
}

#[test]
fn estimate_page_skew_is_zero_for_axis_aligned_text() {
    let mut chars = Vec::new();
    for band in 0..3 {
        let y0 = 300.0 - (band as f64) * 100.0;
        for i in 0..4 {
            let x0 = (i as f64) * 20.0;
            chars.push(ch(x0, y0, x0 + 6.0, y0 + 10.0));
        }
    }
    assert_eq!(estimate_page_skew(&chars), 0.0);
}

#[test]
fn estimate_page_skew_robust_to_a_single_outlier_band() {
    let mut chars = Vec::new();
    // 5 flat (0 degree) bands, well separated in y.
    for band in 0..5 {
        let y0 = 1000.0 - (band as f64) * 100.0;
        for i in 0..4 {
            let x0 = (i as f64) * 20.0;
            chars.push(ch(x0, y0, x0 + 6.0, y0 + 10.0));
        }
    }
    // One extreme-angle outlier band (mirrors the spike's observed
    // formula/watermark outliers), well separated in y from every flat
    // band. Angle chosen so per-step y-drift (~7.3) fits within the
    // bucketing window (15.0), allowing all 4 chars to correctly merge
    // into one band before fitting.
    let outlier_angle_rad = 20.0_f64.to_radians();
    let outlier_baseline = -500.0;
    for i in 0..4 {
        let x0 = (i as f64) * 20.0;
        let drift = x0 * outlier_angle_rad.tan();
        let y0 = outlier_baseline - drift;
        chars.push(ch(x0, y0, x0 + 6.0, y0 + 10.0));
    }
    // Median of [0, 0, 0, 0, 0, ~20] = 0.0 exactly: the outlier band
    // is correctly fit and included, but the median discipline (not mean)
    // keeps the page estimate at 0.0.
    assert_eq!(estimate_page_skew(&chars), 0.0);
}

#[test]
fn estimate_page_skew_nan_bbox_does_not_panic() {
    let chars = vec![
        ch(0.0, f64::NAN, 6.0, 10.0),
        ch(20.0, 100.0, 26.0, 110.0),
        ch(40.0, 100.0, 46.0, 110.0),
        ch(60.0, 100.0, 66.0, 110.0),
    ];
    let _ = estimate_page_skew(&chars); // must not panic
}

#[test]
fn skewed_lines_group_correctly_and_output_geometry_is_never_sheared() {
    // Three page-level "lines" (bands), each with a small, consistent
    // 1.5deg skew (drift = pitch * tan(1.5deg) per char step, ~5.24pt)
    // large enough that WITHOUT correction halign's voverlap check fails
    // between adjacent chars (drift > height(10) * (1 - line_overlap(0.5))
    // = 5), fragmenting each band into 4 separate single-char lines. A
    // generous char_margin keeps the horizontal-gap check passing
    // regardless of correction, isolating the vertical-drift effect under
    // test. After correction each band must merge into one Line, and the
    // output chars/bbox must exactly match the original, uncorrected
    // input coordinates (the shear must never leak into output geometry).
    let params = Params {
        char_margin: 25.0,
        // Isolates the vertical-drift effect under test: at pitch=200 with
        // 10pt-wide chars, the real horizontal gap (190pt) exceeds the
        // default word_margin threshold and would otherwise insert
        // synthetic space chars, unrelated to skew correction.
        word_margin: 0.0,
        ..Params::default()
    };
    let pitch = 200.0;
    let drift_per_step = pitch * 1.5_f64.to_radians().tan();

    let band = |baseline_y: f64| -> Vec<Char> {
        (0..4)
            .map(|i| {
                let x0 = (i as f64) * pitch;
                let y0 = baseline_y - (i as f64) * drift_per_step;
                ch(x0, y0, x0 + 10.0, y0 + 10.0)
            })
            .collect()
    };

    let band0 = band(200.0);
    let band1 = band(100.0);
    let band2 = band(0.0);
    let chars: Vec<Char> = band0
        .iter()
        .chain(band1.iter())
        .chain(band2.iter())
        .cloned()
        .collect();

    let lines = group_lines(chars, &params);

    assert_eq!(lines.len(), 3);
    for (line, expected_band) in lines.iter().zip([&band0, &band1, &band2]) {
        assert_eq!(&line.chars, expected_band);
        assert_eq!(line.bbox, union_all(expected_band.iter().map(|c| c.bbox)));
    }
}

#[test]
fn zero_skew_page_produces_unchanged_grouping_output() {
    // Perfectly flat text (no skew): the estimate is 0.0, well under the
    // noise floor, so group_lines must behave exactly as it did before
    // this feature existed - the primary no-regression guarantee.
    let params = Params::default();
    let mut chars = Vec::new();
    for band in 0..3 {
        let y0 = 300.0 - (band as f64) * 100.0;
        for i in 0..4 {
            let x0 = (i as f64) * 6.5;
            chars.push(ch(x0, y0, x0 + 6.0, y0 + 10.0));
        }
    }
    let lines = group_lines(chars.clone(), &params);
    assert_eq!(lines.len(), 3);
    for (line, band_chars) in lines.iter().zip(chars.chunks(4)) {
        assert_eq!(line.chars, band_chars.to_vec());
    }
}
