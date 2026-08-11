# -*- coding: utf-8 -*-
"""오라클 PDF 캐시 검증 — 침묵-거부로 생성된 '빈 PDF'(텍스트·도형·이미지 0)를 찾아낸다.

배경: FilePathCheckerModule 미등록 상태의 한글 COM 은 Open 을 침묵-거부하고 빈 문서를
남기며, 그 상태로 FileSaveAsPdf 를 실행하면 5~6KB 빈 PDF 가 만들어진다(#4533 캐시 구축
실측). 크기만으로는 판별이 불안정하므로 pymupdf 로 페이지 내용을 직접 센다.

사용:
  python tools/verify_oracle_pdf_cache.py <cache_root> [--delete-empty]
출력(TSV): path, pages, empty_pages, verdict(OK/EMPTY/ERR)
종료코드: 0=전부 OK, 1=EMPTY 또는 ERR 존재.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

import pymupdf

sys.stdout.reconfigure(encoding="utf-8", errors="backslashreplace")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("--delete-empty", action="store_true", help="EMPTY 판정 PDF 삭제(재실행 시 재생성)")
    args = ap.parse_args()

    bad = 0
    for pdf in sorted(Path(args.root).rglob("*.pdf")):
        try:
            doc = pymupdf.open(pdf)
            empty_pages = 0
            for page in doc:
                has_text = bool(page.get_text("words"))
                has_draw = bool(page.get_drawings())
                has_img = bool(page.get_images())
                if not (has_text or has_draw or has_img):
                    empty_pages += 1
            verdict = "EMPTY" if empty_pages == len(doc) else "OK"
            print(f"{pdf}\t{len(doc)}\t{empty_pages}\t{verdict}")
            if verdict == "EMPTY":
                bad += 1
                if args.delete_empty:
                    doc.close()
                    pdf.unlink()
        except Exception as e:  # noqa: BLE001 — 한 파일이 검증을 못 죽이게
            print(f"{pdf}\t-\t-\tERR {str(e)[:80]}")
            bad += 1
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
