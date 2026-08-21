use _core::geometry::Rect;
use _core::lines::group_lines;
use _core::params::Params;
use _core::types::Char;

fn ch(x0: f64, y0: f64, x1: f64, y1: f64) -> Char {
    Char { bbox: Rect { x0, y0, x1, y1 }, text: 'x', font: None }
}

#[test]
fn empty_input_produces_no_lines() {
    let params = Params::default();
    assert_eq!(group_lines(&[], &params), vec![]);
}

#[test]
fn single_char_produces_single_line() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let lines = group_lines(&[a.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![a]);
    assert!(lines[0].upright);
}

#[test]
fn close_chars_on_same_row_merge_into_one_line() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(6.5, 0.0, 12.5, 10.0); // 0.5pt gap: well within char_margin, within word_margin
    let lines = group_lines(&[a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![a, b]);
    assert_eq!(lines[0].bbox, Rect { x0: 0.0, y0: 0.0, x1: 12.5, y1: 10.0 });
}

#[test]
fn far_apart_chars_split_into_separate_lines() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(21.0, 0.0, 27.0, 10.0); // 15pt gap: exceeds char_margin(2.0) * width(6) = 12
    let lines = group_lines(&[a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].chars, vec![a]);
    assert_eq!(lines[1].chars, vec![b]);
}

#[test]
fn wide_but_mergeable_gap_inserts_word_margin_space() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(9.0, 0.0, 15.0, 10.0); // 3pt gap: > word_margin(0.1)*10=1.0, < char_margin(2.0)*6=12
    let lines = group_lines(&[a.clone(), b.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars.len(), 3);
    assert_eq!(lines[0].chars[0], a);
    assert_eq!(lines[0].chars[1].text, ' ');
    assert_eq!(lines[0].chars[2], b);
    // the synthetic space must not widen the line's bbox beyond the real chars
    assert_eq!(lines[0].bbox, Rect { x0: 0.0, y0: 0.0, x1: 15.0, y1: 10.0 });
}

#[test]
fn vertical_text_grouped_only_when_detect_vertical_enabled() {
    let mut params = Params::default();
    let top = ch(0.0, 10.0, 6.0, 20.0);
    let bottom = ch(0.0, 1.5, 6.0, 9.5); // stacked, horizontally aligned, 0.5pt vertical gap

    params.detect_vertical = false;
    let lines = group_lines(&[top.clone(), bottom.clone()], &params);
    assert_eq!(lines.len(), 2, "vertical grouping must be off by default");

    params.detect_vertical = true;
    let lines = group_lines(&[top.clone(), bottom.clone()], &params);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].chars, vec![top, bottom]);
    assert!(!lines[0].upright);
}

#[test]
fn multiple_lines_preserve_order_across_a_run_of_chars() {
    let params = Params::default();
    let a = ch(0.0, 0.0, 6.0, 10.0);
    let b = ch(21.0, 0.0, 27.0, 10.0); // far from a: breaks the line
    let c = ch(27.5, 0.0, 33.5, 10.0); // close to b: merges with b
    let lines = group_lines(&[a.clone(), b.clone(), c.clone()], &params);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].chars, vec![a]);
    assert_eq!(lines[1].chars, vec![b, c]);
}
