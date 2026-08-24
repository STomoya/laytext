"""Bindings-surface tests: FFI type conversion, call shape, exception mapping."""

import pytest

from laytext import (
    Block,
    Char,
    FontInfo,
    Line,
    Page,
    PageInput,
    Params,
    Rect,
    analyze_document,
    analyze_page,
    group_lines,
    group_lines_document,
)


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


def test_group_lines_document_empty_input_returns_empty_list():
    assert group_lines_document([], Params()) == []


def test_group_lines_document_returns_one_line_group_per_page_in_order():
    a = Char(Rect(0.0, 0.0, 6.0, 10.0), 'a')
    pages = group_lines_document([[a], []], Params())
    assert len(pages) == 2
    assert isinstance(pages[0][0], Line)
    assert pages[1] == []


def test_group_lines_document_rejects_wrong_argument_type():
    with pytest.raises(TypeError):
        group_lines_document('not a list', Params())  # ty: ignore[invalid-argument-type]


def test_analyze_page_empty_input_returns_empty_page():
    page = analyze_page([], 100.0, 200.0, Params(column_gap_min=10.0, row_gap_min=10.0))
    assert isinstance(page, Page)
    assert page.blocks == []


def test_analyze_page_returns_page_objects_with_expected_shape():
    a = Char(Rect(0.0, 0.0, 6.0, 10.0), 'a')
    page = analyze_page([a], 100.0, 200.0, Params(column_gap_min=10.0, row_gap_min=10.0))
    assert isinstance(page, Page)
    assert page.width == 100.0
    assert page.height == 200.0
    block = page.blocks[0]
    assert isinstance(block, Block)
    assert block.reading_order == 0
    assert isinstance(block.lines[0], Line)


def test_analyze_page_rejects_wrong_argument_type():
    with pytest.raises(TypeError):
        analyze_page('not a list', 100.0, 200.0, Params())  # ty: ignore[invalid-argument-type]


def test_analyze_page_accepts_none_gap_params():
    page = analyze_page([], 100.0, 200.0, Params())
    assert page.blocks == []


def test_page_input_round_trips_chars_and_dimensions():
    a = Char(Rect(0.0, 0.0, 6.0, 10.0), 'a')
    page_input = PageInput([a], 100.0, 200.0)
    assert page_input.chars[0].text == 'a'
    assert page_input.width == 100.0
    assert page_input.height == 200.0


def test_analyze_document_empty_input_returns_empty_list():
    assert analyze_document([], Params(column_gap_min=10.0, row_gap_min=10.0)) == []


def test_analyze_document_returns_one_page_per_page_input_in_order():
    a = Char(Rect(0.0, 0.0, 6.0, 10.0), 'a')
    params = Params(column_gap_min=10.0, row_gap_min=10.0)
    pages = analyze_document([PageInput([a], 100.0, 200.0), PageInput([], 100.0, 200.0)], params)
    assert [isinstance(p, Page) for p in pages] == [True, True]
    assert len(pages[0].blocks) == 1
    assert pages[1].blocks == []


def test_analyze_document_rejects_wrong_argument_type():
    with pytest.raises(TypeError):
        analyze_document('not a list', Params())  # ty: ignore[invalid-argument-type]


def test_analyze_document_accepts_none_gap_params():
    assert analyze_document([], Params()) == []
