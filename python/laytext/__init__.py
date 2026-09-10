"""Laytext."""

from enum import StrEnum

from laytext._core import (
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
from laytext._version import __version__


class Strategy(StrEnum):
    """Block-detection/reading-order strategy. See `Params.segmentation`."""

    Pdfminer = 'pdfminer'
    XyCut = 'xycut'


__all__ = [
    'Block',
    'Char',
    'FontInfo',
    'Line',
    'Page',
    'PageInput',
    'Params',
    'Rect',
    'Strategy',
    '__version__',
    'analyze_document',
    'analyze_page',
    'group_lines',
    'group_lines_document',
]
