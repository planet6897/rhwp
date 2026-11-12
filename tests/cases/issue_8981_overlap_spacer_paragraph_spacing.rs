//! [#8981] 관계형 회귀 — 겹침 걸음 사다리(음수 줄간격) 칸의 빈 줄 문단 전진이 다음 문단의
//! 앞 간격을 두 번 더하지 않는다.
//!
//! 주석 갱신: 2026-11-12 KST(+09:00), 새 파일.
//!
//! - 입력: `samples/issue8981/1611000-201100106-overlap-spacer-spacing.hwp` — 공개 해양수산부
//!   정책연구 평가결과서(HWP5, 1쪽). 본문 문단 1 의 9×6 표 칸 17(「평가 결과」 오른쪽, 12문단)은
//!   음수 줄간격 줄 상자가 겹쳐 적힌 저장 사다리이고, 빈 줄 문단 셋(문단 모양 줄간격 60%) 다음
//!   문단은 문단 앞 간격(300HU)을 가진다.
//! - 독립 기준: `pdf/issue8981/1611000-201100106-overlap-spacer-spacing-2020.pdf`(한/글 2022
//!   12.0.0.4547 Print, 1쪽).
//! - 수정 전 rhwp(devel `80b1c354c2`)는 빈 줄의 저장 전진(다음 vpos − 이 vpos, 다음 앞 간격 포함)에
//!   다음 문단 앞 간격을 다시 더해, 빈 줄을 지날 때마다 뒤 문단이 4px 씩(칸 끝 12px) 내려가고
//!   마지막 줄이 칸 아래 경계에 잘렸다.
//!
//! 검사: 원본 계약(겹침 사다리, 빈 줄 문단 뒤 문단의 앞 간격 > 0), 쪽수 1 = 한/글(고정형), 칸 안
//! 글이 있는 문단의 첫 줄 위끝 간격이 저장 vpos 간격과 1px 안에서 같고, 마지막 줄이 칸 안에
//! 있다(관계형).
#![cfg(not(target_arch = "wasm32"))]

use rhwp::document_core::DocumentCore;
use rhwp::model::control::Control;
use rhwp::renderer::hwpunit_to_px;
use rhwp::renderer::render_tree::{BoundingBox, RenderNode, RenderNodeType};

const SAMPLE: &str = "samples/issue8981/1611000-201100106-overlap-spacer-spacing.hwp";
const HOST_PARA: usize = 1;
const CELL_INDEX: usize = 17;
const DPI: f64 = 96.0;

fn load() -> DocumentCore {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    DocumentCore::from_bytes(&bytes).unwrap_or_else(|e| panic!("parse: {e:?}"))
}

/// 행·열로 칸 노드를 찾는다.
fn find_cell(node: &RenderNode, row: u16, col: u16) -> Option<&RenderNode> {
    if let RenderNodeType::TableCell(cell) = &node.node_type {
        if cell.row == row && cell.col == col {
            return Some(node);
        }
    }
    node.children.iter().find_map(|c| find_cell(c, row, col))
}

/// 칸 안 글줄 (칸 문단, 줄 번호, 상자).
fn lines(node: &RenderNode, out: &mut Vec<(usize, u32, BoundingBox)>) {
    if let RenderNodeType::TextLine(line) = &node.node_type {
        if let (Some(para), Some(index)) = (line.para_index, line.line_index) {
            out.push((para, index, node.bbox));
        }
    }
    for child in &node.children {
        lines(child, out);
    }
}

#[test]
fn overlap_spacer_does_not_add_next_paragraph_spacing_twice() {
    let core = load();
    let doc = core.document();
    let host = &doc.sections[0].paragraphs[HOST_PARA];
    let Some(Control::Table(table)) = host.controls.first() else {
        panic!("문단 {HOST_PARA} 첫 컨트롤은 표");
    };
    let cell = &table.cells[CELL_INDEX];
    let paragraphs = &cell.paragraphs;
    assert_eq!(paragraphs.len(), 12, "칸 {CELL_INDEX} 문단 12개");

    // 원본 계약: 겹침 걸음 사다리이고, 빈 줄 문단 다음 문단이 앞 간격을 가진다.
    let mut spacers = 0;
    for (index, para) in paragraphs.iter().enumerate() {
        if !para.text.trim().is_empty() || index + 1 == paragraphs.len() {
            continue;
        }
        let [seg] = para.line_segs.as_slice() else {
            panic!("빈 줄 문단 {index} 는 저장 줄 하나");
        };
        let next = &paragraphs[index + 1];
        let forward = next.line_segs[0].vertical_pos - seg.vertical_pos;
        assert!(
            seg.line_spacing < 0 && forward > 0 && forward < seg.line_height,
            "빈 줄 문단 {index} 는 겹침 걸음: {seg:?} → {}",
            next.line_segs[0].vertical_pos
        );
        let before = doc.doc_info.para_shapes[next.para_shape_id as usize].spacing_before;
        assert!(before > 0, "빈 줄 뒤 문단 {} 은 앞 간격을 가진다", index + 1);
        spacers += 1;
    }
    assert_eq!(spacers, 3, "빈 줄 문단 셋");

    assert_eq!(core.page_count(), 1, "전체 쪽수 = 한/글 1쪽");
    let page = core.build_page_render_tree(0).expect("1쪽");
    let cell_node = find_cell(&page.root, cell.row, cell.col).expect("칸 노드");
    let mut found = Vec::new();
    lines(cell_node, &mut found);

    let first_line_top = |para: usize| {
        found
            .iter()
            .find(|(owner, line, _)| *owner == para && *line == 0)
            .map(|(_, _, bbox)| bbox.y)
            .unwrap_or_else(|| panic!("칸 문단 {para} 첫 줄"))
    };
    let origin_y = first_line_top(0);
    let origin_vpos = paragraphs[0].line_segs[0].vertical_pos;
    for (index, para) in paragraphs.iter().enumerate() {
        if para.text.trim().is_empty() {
            continue;
        }
        let expected = hwpunit_to_px(para.line_segs[0].vertical_pos - origin_vpos, DPI);
        let actual = first_line_top(index) - origin_y;
        assert!(
            (actual - expected).abs() <= 1.0,
            "칸 문단 {index} 첫 줄이 저장 사다리 자리에 있다: 실제 {actual:.2}px, 저장 {expected:.2}px"
        );
    }

    let cell_bottom = cell_node.bbox.y + cell_node.bbox.height;
    let last_bottom = found
        .iter()
        .map(|(_, _, bbox)| bbox.y + bbox.height)
        .fold(f64::MIN, f64::max);
    assert!(
        last_bottom <= cell_bottom + 0.5,
        "마지막 줄이 칸 안에 있다: 줄 아래 {last_bottom:.2}, 칸 아래 {cell_bottom:.2}"
    );
}
