class Rect:
    x0: float
    y0: float
    x1: float
    y1: float
    def __init__(self, x0: float, y0: float, x1: float, y1: float) -> None: ...

class FontInfo:
    name: str | None
    size: float | None
    bold: bool | None
    italic: bool | None
    def __init__(
        self,
        name: str | None = None,
        size: float | None = None,
        bold: bool | None = None,
        italic: bool | None = None,
    ) -> None: ...

class Char:
    bbox: Rect
    # Always a single Unicode scalar. `str` is the closest available stub
    # type; the constructor raises ValueError for multi-character strings.
    text: str
    font: FontInfo | None
    def __init__(self, bbox: Rect, text: str, font: FontInfo | None = None) -> None: ...

class Line:
    bbox: Rect
    upright: bool
    chars: list[Char]
    def __init__(self, bbox: Rect, upright: bool, chars: list[Char]) -> None: ...

class Block:
    bbox: Rect
    reading_order: int
    lines: list[Line]
    def __init__(self, bbox: Rect, reading_order: int, lines: list[Line]) -> None: ...

class Page:
    width: float
    height: float
    blocks: list[Block]
    def __init__(self, width: float, height: float, blocks: list[Block]) -> None: ...

class PageInput:
    width: float
    height: float
    chars: list[Char]
    def __init__(self, chars: list[Char], width: float, height: float) -> None: ...

class Params:
    char_margin: float
    line_overlap: float
    line_margin: float
    word_margin: float
    column_gap_min: float | None
    row_gap_min: float | None
    full_width_threshold: float
    detect_vertical: bool
    def __init__(
        self,
        char_margin: float = 2.0,
        line_overlap: float = 0.5,
        line_margin: float = 0.5,
        word_margin: float = 0.1,
        column_gap_min: float | None = None,
        row_gap_min: float | None = None,
        full_width_threshold: float = 0.9,
        detect_vertical: bool = False,
    ) -> None: ...

def group_lines(chars: list[Char], params: Params) -> list[Line]: ...
def analyze_page(chars: list[Char], width: float, height: float, params: Params) -> Page: ...
def analyze_document(pages: list[PageInput], params: Params) -> list[Page]: ...
