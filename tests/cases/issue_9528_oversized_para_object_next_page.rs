//! 관계형 — [#9528] 본문보다 높은 문단 기준 자리차지 그림이 본문 아래 끝을 넘으면 그림만 다음 쪽
//! 본문 맨 위에서 시작하고, 앞 쪽 글은 가리지 않는다. 쪽수 검사는 고정형이다.
//! 주석 갱신: 2026-11-28 KST (처음 작성)
//!
//! 입력: `samples/issue9528/oversized_para_picture.hwp` — 공개 보도자료(방송통신위원회 2020.6.29
//! 「인터넷윤리 창작콘텐츠 공모전」) 원본, HWP 9.0.0.627 저장. pi=24 그림 48190×73133HU(258mm),
//! 본문 높이 252mm.
//! 기준: `pdf/issue9528/oversized_para_picture-2020.pdf` — 한/글 2020(11.0.0.9172) Hancom PDF Print, 3쪽.
//!
//! 측정: MEASURED
#![cfg(not(target_arch = "wasm32"))]

use rhwp::document_core::DocumentCore;
use rhwp::renderer::render_tree::{RenderNode, RenderNodeType};

const FIXTURE: &str = "samples/issue9528/oversized_para_picture.hwp";
const HANCOM_PAGES: u32 = 3;
const PICTURE_PARA: usize = 24;

fn open() -> DocumentCore {
    let bytes = std::fs::read(FIXTURE).expect("fixture");
    DocumentCore::from_bytes(&bytes).expect("문서 로드")
}

fn find<'a>(
    node: &'a RenderNode,
    pred: &dyn Fn(&RenderNode) -> bool,
    out: &mut Vec<&'a RenderNode>,
) {
    if pred(node) {
        out.push(node);
    }
    for child in &node.children {
        find(child, pred, out);
    }
}

fn body(root: &RenderNode) -> (f64, f64) {
    let mut v = Vec::new();
    find(
        root,
        &|n| matches!(n.node_type, RenderNodeType::Body { .. }),
        &mut v,
    );
    let b = &v.first().expect("본문 영역").bbox;
    (b.y, b.y + b.height)
}

fn pictures_of(root: &RenderNode, para: usize) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    find(
        root,
        &|n| matches!(&n.node_type, RenderNodeType::Image(img) if img.para_index == Some(para)),
        &mut v,
    );
    v.iter()
        .map(|n| (n.bbox.y, n.bbox.y + n.bbox.height))
        .collect()
}

fn text_of(node: &RenderNode, out: &mut String) {
    if let RenderNodeType::TextRun(run) = &node.node_type {
        out.push_str(run.display_text.as_deref().unwrap_or(&run.text));
    }
    for child in &node.children {
        text_of(child, out);
    }
}

/// 쪽의 글줄을 위에서부터 (위끝, 아래끝, 글) 로 모은다.
fn lines(root: &RenderNode) -> Vec<(f64, f64, String)> {
    let mut v = Vec::new();
    find(
        root,
        &|n| matches!(n.node_type, RenderNodeType::TextLine(_)),
        &mut v,
    );
    v.iter()
        .map(|n| {
            let mut t = String::new();
            text_of(n, &mut t);
            (n.bbox.y, n.bbox.y + n.bbox.height, t)
        })
        .collect()
}

#[test]
fn oversized_picture_starts_the_next_page_and_leaves_the_text_uncovered() {
    let mut core = open();
    assert_eq!(core.page_count(), HANCOM_PAGES, "한/글 Print 의 전체 쪽수");

    // 그림은 셋째 쪽에만 한 번 놓이고, 본문 위 끝에서 시작해 본문 아래 끝을 넘는다.
    let mut owners = Vec::new();
    for page in 0..HANCOM_PAGES {
        let tree = core.build_page_render_tree(page).unwrap();
        for frame in pictures_of(&tree.root, PICTURE_PARA) {
            owners.push((page, frame, body(&tree.root)));
        }
    }
    let [(page, (top, bottom), (body_top, body_bottom))] = owners.as_slice() else {
        panic!("pi={PICTURE_PARA} 그림은 한 쪽에 한 번: {owners:?}");
    };
    assert_eq!(*page, 2, "그림은 셋째 쪽");
    assert!(bottom - top > body_bottom - body_top, "본문보다 높은 그림");
    assert!(
        (top - body_top).abs() <= 0.5,
        "그림 위끝 {top:.2} 은 본문 위끝 {body_top:.2}"
    );
    assert!(bottom > body_bottom, "아래 끝은 본문 밖으로 넘친다");

    // 둘째 쪽 글은 순서대로 남고 그림에 가려지지 않는다.
    let tree = core.build_page_render_tree(1).unwrap();
    assert!(pictures_of(&tree.root, PICTURE_PARA).is_empty());
    let page2 = lines(&tree.root);
    let index = |needle: &str| {
        page2
            .iter()
            .position(|(_, _, t)| t.contains(needle))
            .unwrap_or_else(|| panic!("둘째 쪽 「{needle}」"))
    };
    let first = index("또한, 병영 내");
    let last = index("붙임");
    assert!(first < last);
    let (_, body_bottom) = body(&tree.root);
    assert!(page2[last].1 <= body_bottom, "끝 줄은 본문 안");
}

#[test]
fn oversized_picture_already_at_the_body_top_stays_on_its_page() {
    // 반례: 앞 문단을 모두 지워 그림 문단이 첫 쪽 본문 맨 위에 오면, 다음 쪽 맨 위로 옮겨도
    // 같은 자리이므로 그 쪽에 남는다(빈 쪽을 만들지 않는다).
    let mut core = open();
    for _ in 0..PICTURE_PARA {
        core.delete_paragraph_native(0, 0).expect("앞 문단 삭제");
    }
    assert_eq!(core.document().sections[0].paragraphs.len(), 1);
    assert_eq!(core.page_count(), 1, "그림 하나뿐인 문서는 한 쪽");
    let tree = core.build_page_render_tree(0).unwrap();
    let frames = pictures_of(&tree.root, 0);
    let (body_top, _) = body(&tree.root);
    let [(top, _)] = frames.as_slice() else {
        panic!("첫 쪽 그림 하나: {frames:?}");
    };
    assert!(
        (top - body_top).abs() <= 0.5,
        "그림 위끝 {top:.2} 은 본문 위끝 {body_top:.2}"
    );
    for page in 1..core.page_count() {
        let tree = core.build_page_render_tree(page).unwrap();
        assert!(
            pictures_of(&tree.root, 0).is_empty(),
            "{}쪽에 다시 그리지 않는다",
            page + 1
        );
    }
}
