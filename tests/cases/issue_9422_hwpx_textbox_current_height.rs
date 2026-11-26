//! 관계형 — [#9422] 한/글이 저장한 HWPX 의 넘치는 글상자는 `curSz` 폭 0 이어도 저장 현재 높이로 그린다.
//! 주석 갱신: 2026-11-26 23:49 KST (새 파일).
//!
//! - 입력: `samples/issue9419/kaf_appendix_2020.hwpx` — `samples/issue9379/kaf_appendix.hwp` 를 한/글 2020
//!   (11.0.0.1623)으로 HWPX 저장한 54쪽 문서(#9419 fixture, sha256 57f6d3a6…2676).
//!   기준: `pdf/issue9419/kaf_appendix_2020-2020.pdf`(한/글 2020 11.0.0.1623 `Hancom PDF` Print, 54쪽).
//! - 36쪽 용어 글상자(구역 1 문단 46)는 `<hp:sz height="7855">`, `<hp:orgSz 47685×7855>`,
//!   `<hp:curSz width="0" height="8743">`, 세로 배율 1.113049 를 저장한다. 한/글은 높이만 바꾼 상자의
//!   현재 폭을 0 으로 적는다. 같은 문서의 HWP 재저장본(#9395)은 현재 폭 47685 를 적으며 두 형식의
//!   현재 높이·행렬·저장 글줄(vpos 0·3243·6486, 줄 높이 1691, 안 여백 283)이 같다.
//! - 검사(관계형): 인쇄 경계 높이가 저장 현재 높이 8743 HU 이고, 첫 줄은 경계 위끝 아래, 셋째 줄
//!   「and Cadastre」의 아래끝은 경계 아래끝 위에 있다. 수정 전에는 폭 0 표시 때문에 개체 높이
//!   7855 HU 로 잘라 셋째 줄 아래가 잘린다.
//! - 반례(관계형): 높이 0 표시(`current_height_was_zero`)는 현재 높이의 증언이 아니므로 같은 글상자를
//!   선언 높이 7855 HU 로 그린다.
//!
//! 측정일 2026-11-26. raw 측정: `scripts/visual_sweep.py --silhouette-only --compat 2022 --dpi 96` 의
//! `silhouette_raw_match_percent`(색상 경계 보정 전 2px 내용 실루엣). 이 fixture 의 21·23쪽은 #9419(PR #9427)
//! 수정이 있어야 90% 이상이므로 base = upstream/devel `48a6783990` + PR #9427 head `f38a68e198`,
//! head = 그 위에 이 수정(source `b6a767c9c6`). 전체 54쪽 = 한/글 2020 PDF 54쪽.
//!
//! | 쪽 | Native base | Native head | fresh WASM base | fresh WASM head |
//! |---|---|---|---|---|
//! | 36 | 95.96 | 100.00 | 96.26 | 99.95 |
//!
//! 바뀐 쪽은 36쪽뿐이다. head 전쪽 최저: Native 94.32(1쪽), fresh WASM 93.13(1쪽). 90% 미만 없음.
//! `curSz width="0"` 과 0 아닌 높이를 가진 samples HWPX 37건(편람 383쪽 포함)의 SVG 는 이 문서 36쪽 밖에서
//! 수정 전후 바이트 단위로 같다.
//! 증거: `mydocs/pr/assets/issue_9422/{native,wasm,base-native,base-wasm}/` 의 전쪽 TSV 와 36쪽 review·overlay PNG.
#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use rhwp::document_core::DocumentCore;
use rhwp::model::control::Control;
use rhwp::model::shape::ShapeObject;
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};

const SAMPLE: &str = "samples/issue9419/kaf_appendix_2020.hwpx";
/// 한/글 2020 Print 쪽수.
const HANCOM_PAGES: u32 = 54;
/// 36쪽(0 기준 35).
const PAGE: u32 = 35;
const SECTION: usize = 1;
const PARA: usize = 46;
/// 저장 개체 높이와 현재 요소 높이(HWPUNIT).
const DECLARED_HU: f64 = 7855.0;
const CURRENT_HU: f64 = 8743.0;

fn px(hu: f64) -> f64 {
    hu * 96.0 / 7200.0
}

fn open() -> DocumentCore {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {SAMPLE}: {e}"));
    let core = DocumentCore::from_bytes(&bytes).unwrap_or_else(|e| panic!("open {SAMPLE}: {e:?}"));
    assert_eq!(
        core.page_count(),
        HANCOM_PAGES,
        "한/글 2020 Print 와 같은 쪽수"
    );
    core
}

fn terminology_textbox(core: &mut DocumentCore) -> &mut rhwp::model::shape::RectangleShape {
    let Control::Shape(shape) =
        &mut core.document_mut().sections[SECTION].paragraphs[PARA].controls[0]
    else {
        panic!("36쪽 용어 글상자 도형");
    };
    let ShapeObject::Rectangle(rectangle) = shape.as_mut() else {
        panic!("36쪽 용어 글상자 사각형");
    };
    rectangle
}

fn find_textbox_with_line<'a>(
    node: &'a RenderNode,
    first_word: &str,
) -> Option<(&'a RenderNode, &'a RenderNode)> {
    if matches!(node.node_type, RenderNodeType::TextBox) {
        let matching_line = node.children.iter().find(|line| {
            matches!(line.node_type, RenderNodeType::TextLine(_))
                && line.children.iter().any(|child| {
                    matches!(&child.node_type, RenderNodeType::TextRun(run)
                        if run.text.trim_start().starts_with(first_word))
                })
        });
        if let Some(line) = matching_line {
            return Some((node, line));
        }
    }
    node.children
        .iter()
        .find_map(|child| find_textbox_with_line(child, first_word))
}

#[test]
fn hwpx_width_zero_textbox_uses_the_saved_current_height_for_its_print_frame() {
    let mut core = open();
    // 입력 증언: 폭은 0 표시(원래 폭으로 채움), 높이는 저장 현재 높이.
    let rectangle = terminology_textbox(&mut core);
    let attr = &rectangle.drawing.shape_attr;
    assert!(attr.current_width_was_zero, "curSz width=\"0\"");
    assert!(!attr.current_height_was_zero, "curSz height 는 저장값");
    assert_eq!(attr.current_width, attr.original_width);
    assert_eq!(f64::from(attr.current_height), CURRENT_HU);
    assert_eq!(f64::from(rectangle.common.height), DECLARED_HU);

    let tree = core.build_page_render_tree(PAGE).unwrap();
    let (textbox, first_line) =
        find_textbox_with_line(&tree.root, "In each").expect("36쪽 용어 글상자 첫 줄");
    let (last_box, last_line) =
        find_textbox_with_line(&tree.root, "and Cadastre").expect("36쪽 용어 글상자 셋째 줄");
    assert_eq!(last_box.id, textbox.id, "세 줄이 같은 글상자에 있다");
    let clip = textbox.print_clip_bbox.expect("글상자 인쇄 경계");
    assert!(
        (clip.height - px(CURRENT_HU)).abs() < 1e-6,
        "인쇄 경계 높이 {:.2}px 는 저장 현재 높이 {:.2}px — 개체 높이 {:.2}px 로 자르면 셋째 줄이 잘린다",
        clip.height,
        px(CURRENT_HU),
        px(DECLARED_HU)
    );
    assert!(first_line.bbox.y >= clip.y, "첫 줄은 경계 안에서 시작한다");
    let line_bottom = last_line.bbox.y + last_line.bbox.height;
    let clip_bottom = clip.y + clip.height;
    assert!(
        line_bottom < clip_bottom,
        "셋째 줄 아래끝 {line_bottom:.2} 은 경계 아래끝 {clip_bottom:.2} 위에 있다"
    );
}

#[test]
fn hwpx_height_zero_marker_keeps_the_declared_frame() {
    let mut core = open();
    let rectangle = terminology_textbox(&mut core);
    rectangle.drawing.shape_attr.current_height_was_zero = true;
    core.set_document(core.document().clone());
    let tree = core.build_page_render_tree(PAGE).unwrap();
    let (textbox, _) =
        find_textbox_with_line(&tree.root, "In each").expect("36쪽 용어 글상자 첫 줄");
    let clip = textbox.print_clip_bbox.expect("글상자 인쇄 경계");
    assert!(
        (clip.height - px(DECLARED_HU)).abs() < 1e-6,
        "높이 0 표시면 선언 높이 {:.2}px 를 유지한다 (실제 {:.2}px)",
        px(DECLARED_HU),
        clip.height
    );
}
