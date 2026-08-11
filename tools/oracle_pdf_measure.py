# -*- coding: utf-8 -*-
"""한글 오라클 PDF 좌표 측정 — 텍스트 줄 y 와 수평 테두리(표 경계 후보) y 를 rhwp px 단위로.

rhwp dump-extents 의 px 좌표(96dpi)와 직접 대조하도록 pt×4/3 으로 환산해 출력한다.
#4533 ⑤-a 밴드 규칙 실측에 쓴 스크래치 도구(pdfpos/pdflines)의 저장소 판.

사용:
  python tools/oracle_pdf_measure.py text  <pdf> <page(1-base)> [부분문자열]
  python tools/oracle_pdf_measure.py lines <pdf> <page(1-base)> [ymin_px] [ymax_px]

`text` 는 스팬 병합 줄 단위 y0..y1 과 x, 내용을 출력한다. `lines` 는 폭 100px 이상의
수평 선분/얇은 사각형(표 외곽·행 경계 후보)만 y 정렬로 출력한다 — 테두리 없는 표는
`text` 로 첫 행 셀 텍스트를 대조한다.
"""
from __future__ import annotations

import sys

import pymupdf

sys.stdout.reconfigure(encoding="utf-8", errors="backslashreplace")
PX = 4.0 / 3.0  # pt(72dpi) → rhwp px(96dpi)


def cmd_text(pdf: str, page_no: int, needle: str | None) -> None:
    page = pymupdf.open(pdf)[page_no - 1]
    print(f"page {page.rect.width * PX:.1f}x{page.rect.height * PX:.1f}px")
    for block in page.get_text("dict")["blocks"]:
        if block.get("type") != 0:
            continue
        for line in block["lines"]:
            text = "".join(s["text"] for s in line["spans"]).strip()
            if not text or (needle and needle not in text):
                continue
            x0, y0, _, y1 = line["bbox"]
            print(f"y={y0 * PX:7.1f}..{y1 * PX:7.1f} x={x0 * PX:6.1f} {text[:60]!r}")


def cmd_lines(pdf: str, page_no: int, ymin: float, ymax: float) -> None:
    page = pymupdf.open(pdf)[page_no - 1]
    rows = []
    for d in page.get_drawings():
        for item in d["items"]:
            if item[0] == "l":
                p1, p2 = item[1], item[2]
                if abs(p1.y - p2.y) < 0.5 and abs(p1.x - p2.x) * PX > 100:
                    rows.append((p1.y * PX, min(p1.x, p2.x) * PX, abs(p1.x - p2.x) * PX))
            elif item[0] == "re":
                r = item[1]
                if r.width * PX > 100:
                    rows.append((r.y0 * PX, r.x0 * PX, r.width * PX))
                    if r.height * PX > 3:
                        rows.append((r.y1 * PX, r.x0 * PX, r.width * PX))
    seen = set()
    for y, x, w in sorted(rows):
        if not (ymin <= y <= ymax):
            continue
        key = round(y * 2)
        if key in seen:
            continue
        seen.add(key)
        print(f"hline y={y:7.1f} x={x:6.1f} w={w:6.1f}")


def main() -> int:
    if len(sys.argv) < 4 or sys.argv[1] not in ("text", "lines"):
        print(__doc__)
        return 2
    mode, pdf, page_no = sys.argv[1], sys.argv[2], int(sys.argv[3])
    if mode == "text":
        cmd_text(pdf, page_no, sys.argv[4] if len(sys.argv) > 4 else None)
    else:
        ymin = float(sys.argv[4]) if len(sys.argv) > 4 else 0.0
        ymax = float(sys.argv[5]) if len(sys.argv) > 5 else 1e9
        cmd_lines(pdf, page_no, ymin, ymax)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
