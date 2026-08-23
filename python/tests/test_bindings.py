"""Bindings-surface tests: FFI type conversion, call shape, exception mapping."""

import pytest

from laytext import Char, FontInfo, Line, Params, Rect, group_lines


def test_rect_round_trips_fields():
    r = Rect(1.0, 2.0, 3.0, 4.0)
    assert (r.x0, r.y0, r.x1, r.y1) == (1.0, 2.0, 3.0, 4.0)


def test_char_default_font_is_none():
    c = Char(Rect(0.0, 0.0, 1.0, 1.0), 'a')
    assert c.text == 'a'
    assert c.font is None


def test_char_accepts_font_info():
    font = FontInfo(name='Helvetica', size=10.0, bold=True, italic=False)
    c = Char(Rect(0.0, 0.0, 1.0, 1.0), 'a', font)
    assert c.font is not None
    assert c.font.name == 'Helvetica'
    assert c.font.bold is True


def test_char_rejects_multi_character_text():
    with pytest.raises(ValueError):
        Char(Rect(0.0, 0.0, 1.0, 1.0), 'ab')


def test_params_defaults():
    p = Params()
    assert p.char_margin == 2.0
    assert p.column_gap_min is None


def test_group_lines_empty_input_returns_empty_list():
    assert group_lines([], Params()) == []


def test_group_lines_returns_line_objects_with_expected_shape():
    a = Char(Rect(0.0, 0.0, 6.0, 10.0), 'a')
    lines = group_lines([a], Params())
    line = lines[0]
    assert isinstance(line, Line)
    assert isinstance(line.bbox, Rect)
    assert isinstance(line.chars[0], Char)


def test_group_lines_rejects_wrong_argument_type():
    with pytest.raises(TypeError):
        group_lines('not a list', Params())  # ty: ignore[invalid-argument-type]
