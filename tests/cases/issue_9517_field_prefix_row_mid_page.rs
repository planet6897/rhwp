//! [#9517] 관계형·고정형 회귀 — 글자 없는 누름틀 host 의 한/글 저장 prefix 줄 뒤 넓은 글자처럼
//! 취급 표가 쪽 중간에 있을 때, 그 prefix 줄의 전진을 버리지 않고 표를 둘째 저장 줄에 둔다.
//!
//! 주석 갱신: 2026-11-28 23:40 KST(+09:00), 새 파일.
//!
//! 입력: `samples/issue9517/field_only_wide_table_hancom.hwp` — #8773 원본(공개 결재문서 36479199,
//! `samples/issue8773/field_led_wide_table.hwpx`) 문단 5 에서 끝 공백을 지운 글자 없는 누름틀 host
//! 를 원격 한/글 2020(11.0.0.9172)이 HWP 로 다시 저장한 것. 문단 5 = [누름틀 시작][16×16 표(바깥
//! 50346HU > 줄 49608HU)][누름틀 끝], 저장 줄 2개: 첫 줄 ts 0 vpos 13496 lh = th = 1100 줄 간격 220,
//! 표 줄 ts 8 vpos 14816(= 13496 + 1100 + 220) lh 57408. 바로 앞 문단 4 에 결재 표 두 개(글자처럼
//! 취급)가 있다.
//!
//! 독립 기준: `pdf/issue9517/field_only_wide_table_hancom-2020.pdf` — 원격 한/글 2020 11.0.0.9172
//! Print → Hancom PDF 1.3.0.550, 2쪽. 1쪽 래스터가 #8773 누름틀 원본의 한/글 출력과 같다(본문 표
//! 위 괘선 ≈ 255.7px).
//!
//! 원인: 쪽 나눔은 `owned_tac_prefix_rows` 로 첫 줄을 표 앞 prefix 조각(PartialParagraph 0..1)으로
//! 내고 그 높이를 예약한다. 배치의 "TAC 뒤 공백 조각 건너뛰기"는 직전 항목이 어느 문단이든 TAC
//! 였으면(`prev_tac_seg_applied`) 이 조각을 표 뒤 공백 꼬리로 보고 전진 없이 돌려보냈다 — 쪽 맨
//! 위에서는 직전 TAC 가 없어 맞고, 앞 문단에 결재 표가 있는 쪽 중간에서만 표가 첫 줄 원점에 남았다.
//!
//! 측정: 2026-11-28, `scripts/visual_sweep.py --silhouette-only --compat 2022 --dpi 96`(4열
//! `silhouette_raw_match_percent`, 색상 경계 보정 전 2px 실루엣).
//! base devel `274ae9d4c8`, 수정 `86b164b59a`. 전체 2쪽 = PDF 2쪽.
//! - 1쪽: Native 100.00 / fresh WASM 98.89 (수정 전 50.52 / 51.52).
//! - 2쪽: Native 94.15 / fresh WASM 94.50 (수정 전과 같음). 이 수정이 바꾸지 않은 #8773 원본과 같은 쪽이며
//!   HY 계열 본문 글꼴(HY헤드라인M·HY견고딕·HY그래픽·휴먼명조 등)을 공급하지 못한 글꼴 차이만 남는다.
//!   같은 fresh WASM 으로 잰 #8773 누름틀 원본 1쪽도 base·수정 모두 98.89 다.
//! - 전쪽 최저 Native 94.15 · fresh WASM 94.50 (둘 다 2쪽, 글꼴).
//! - 증거: `mydocs/pr/assets/issue_9517/{native,native-base,wasm,wasm-base}/`(TSV·review·overlay PNG).
//!
//! 검사: 원시 순서와 저장 줄 조건(입력 조건), 표 위끝 = #8773 누름틀 원본 표 위끝(관계형, ±1px),
//! 표는 앞 결재 표 아래이고 뒤 문단 6 의 글줄은 표 아래(관계형), 쪽수 = 한/글 2쪽(고정형).
#![cfg(not(target_arch = "wasm32"))]

use rhwp::document_core::DocumentCore;
use rhwp::model::control::Control;
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};

const SAMPLE: &str = "samples/issue9517/field_only_wide_table_hancom.hwp";
const FIELD_LED: &str = "samples/issue8773/field_led_wide_table.hwpx";
/// 표를 담은 본문 문단(구역 0).
const HOST: usize = 5;

fn load(sample: &str) -> DocumentCore {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(sample);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    DocumentCore::from_bytes(&bytes).unwrap_or_else(|e| panic!("parse {sample}: {e:?}"))
}

fn find_table(node: &RenderNode, rows: u16, cols: u16) -> Option<&RenderNode> {
    if let RenderNodeType::Table(table) = &node.node_type {
        if table.row_count == rows && table.col_count == cols {
            return Some(node);
        }
    }
    node.children
        .iter()
        .find_map(|child| find_table(child, rows, cols))
}

fn text_line_tops(node: &RenderNode, para: usize, out: &mut Vec<f64>) {
    if let RenderNodeType::TextLine(line) = &node.node_type {
        if line.para_index == Some(para) {
            out.push(node.bbox.y);
        }
    }
    for child in &node.children {
        text_line_tops(child, para, out);
    }
}

fn body_table(core: &DocumentCore, sample: &str) -> (f64, f64, RenderNode) {
    let page = core
        .build_page_render_tree(0)
        .unwrap_or_else(|e| panic!("{sample} 1쪽: {e:?}"));
    let table = find_table(&page.root, 16, 16).expect("16×16 본문 표");
    let (top, bottom) = (table.bbox.y, table.bbox.y + table.bbox.height);
    (top, bottom, page.root)
}

#[test]
fn hancom_field_prefix_row_keeps_the_mid_page_table_on_the_second_row() {
    let core = load(SAMPLE);
    let para = &core.document().sections[0].paragraphs[HOST];

    // 입력 조건: 글자 없는 누름틀 host, 원시 순서 = 누름틀 시작 → 표.
    assert!(para.text.is_empty(), "글자 없는 host");
    let table_index = para
        .controls
        .iter()
        .position(|control| matches!(control, Control::Table(_)))
        .expect("문단 5 표");
    assert!(
        table_index > 0
            && para.controls[..table_index]
                .iter()
                .all(|control| matches!(control, Control::Field(_))),
        "원시 순서 = 누름틀 시작 → 표"
    );
    // 입력 조건: 한/글 저장 줄 = 글자 높이 prefix 줄(lh = th) + 그 바로 아래 표 줄.
    assert_eq!(para.line_segs.len(), 2, "한/글 저장 줄 2개");
    let (prefix, row) = (&para.line_segs[0], &para.line_segs[1]);
    assert_eq!(
        (prefix.text_start, row.text_start),
        (0, 8),
        "prefix 줄 + 표 줄"
    );
    assert_eq!(
        prefix.line_height, prefix.text_height,
        "prefix 줄은 글자 높이 줄"
    );
    assert_eq!(
        row.vertical_pos,
        prefix.vertical_pos + prefix.text_height + prefix.line_spacing,
        "표 줄은 prefix 줄 바로 아래"
    );
    // 입력 조건: 앞 문단에 글자처럼 취급 결재 표가 있어 쪽 중간이다.
    let previous = &core.document().sections[0].paragraphs[HOST - 1];
    assert!(
        previous
            .controls
            .iter()
            .any(|control| matches!(control, Control::Table(t) if t.common.treat_as_char)),
        "앞 문단 4 = 글자처럼 취급 결재 표"
    );

    assert_eq!(core.page_count(), 2, "쪽수 = 한/글 2쪽");
    let (top, bottom, root) = body_table(&core, SAMPLE);

    // 앞 결재 표 아래.
    let mut approval_bottom: f64 = 0.0;
    for (rows, cols) in [(3, 2), (3, 4)] {
        let table = find_table(&root, rows, cols).expect("문단 4 결재 표");
        approval_bottom = approval_bottom.max(table.bbox.y + table.bbox.height);
    }
    assert!(
        top >= approval_bottom - 0.5,
        "본문 표 {top:.1} 는 앞 결재 표 {approval_bottom:.1} 아래"
    );

    // 표 위끝 = #8773 누름틀 원본(한/글 출력과 같은 래스터)의 표 위끝.
    let reference_core = load(FIELD_LED);
    let (reference, _, _) = body_table(&reference_core, FIELD_LED);
    assert!(
        (top - reference).abs() <= 1.0,
        "표 위끝 {top:.1} = 누름틀 원본 표 위끝 {reference:.1}(prefix 줄 아래)"
    );

    // 뒤 문단 6 의 글줄은 표 아래.
    let mut following = Vec::new();
    text_line_tops(&root, HOST + 1, &mut following);
    let first_following = following.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        first_following.is_finite() && first_following >= bottom - 0.5,
        "문단 6 첫 글줄 {first_following:.1} 는 표 아래끝 {bottom:.1} 아래"
    );
}
