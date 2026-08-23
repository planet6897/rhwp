//! [Issue #5943] HWPX 저장 lineseg 의 `textpos` 가 HWP5 축 그대로 나간다.
//!
//! HWP5 문단 축에서 확장 제어는 예외 없이 8 UTF-16 유닛을 차지한다 — 구역 정의(`secd`)와
//! 단 정의(`cold`)도 그렇다. 그런데 HWPX 문단에는 그 두 자리가 없다. `hp:secPr` 은
//! 문단이 아니라 **구역 머리 run** 이 싣고, 구역 첫 문단의 첫 `hp:colPr` 은 그 템플릿에
//! 흡수된다. 그래서 구역 첫 문단에서 두 축이 슬롯 개수 × 8 만큼 어긋난다.
//!
//! 저장 lineseg 의 `textpos` 를 HWP5 값 그대로 실으면 한글이 세는 자리보다 뒤를 가리키고,
//! 한글 2024 는 그 문단부터 본문을 통째로 폐기한다. 코퍼스 02502 의 h2x 산출은
//! **9쪽 6,040자 → 1쪽 423자**(`secd:2→1, tbl:16→2`)였다. 그 문단의 슬롯 사다리는
//! `secd@0 · cold@8 · pgnp@16,24,32,40 · tbl@48` 이고 원본 lineseg 는 `[0, 48]` 이다.
//!
//! 축이 얼마나 짧은지는 오라클로 직접 쟀다 — `textpos` 를 48 그대로 두면 실패, 40 으로
//! 내려도 실패, **32 로 내리면 원본과 완전히 같아진다**(9쪽 6,040자, 본문 텍스트 SHA-256
//! 과 컨트롤 인구조사까지 일치). 즉 한글이 쓰는 HWPX 축에서 표는 32 — `secd`·`cold`
//! 두 자리 16유닛만큼 짧다. 한글 2022 는 같은 파일을 관대하게 열었다.
//!
//! 계약: 방출 XML 을 한 글자도 내지 않은 슬롯은 HWPX 축을 차지하지 않으므로, 그 뒤의
//! `textpos` 는 슬롯당 8 씩 내려서 낸다.
//!
//! [#5961] 종전에는 여기에 **HWPX 출처 예외**가 붙어 있었다 — 파서가 파일 값을 그대로
//! 담아 출처마다 축이 달랐기 때문이다. 이제 HWPX 파서가 읽을 때 HWP5 축으로 올리므로
//! (`parser/hwpx/section.rs` 의 `hwp5_only_leading_slots`) IR 의 축은 하나이고, 내보낼
//! 때는 출처를 묻지 않고 언제나 내린다. 올리는 쪽과 내리는 쪽이 같은 규칙(앞머리
//! 비방출 슬롯 × 8)을 쓰므로 왕복이 고정점이다 — `aift.hwpx` 문단 0 은 파서가 24 → 40
//! 으로 올리고 직렬화기가 40 → 24 로 되돌린다(`task1391_aift_memo_roundtrips`).

use std::io::Read;

use rhwp::model::control::{Control, PageNumberPos};
use rhwp::model::document::{Document, Section, SectionDef};
use rhwp::model::page::ColumnDef;
use rhwp::model::page::PageDef;
use rhwp::model::paragraph::{LineSeg, Paragraph};
use rhwp::model::style::{CharShape, ParaShape};
use rhwp::model::table::{Cell, Table};

fn secdef() -> SectionDef {
    SectionDef {
        page_def: PageDef {
            width: 59528,
            height: 84188,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn text_para(text: &str) -> Paragraph {
    Paragraph {
        text: text.to_string(),
        char_count: text.chars().count() as u32,
        ..Default::default()
    }
}

fn line_seg(text_start: u32) -> LineSeg {
    LineSeg {
        text_start,
        vertical_pos: 320,
        line_height: 20629,
        text_height: 20629,
        baseline_distance: 17535,
        line_spacing: 600,
        column_start: 0,
        segment_width: 49108,
        // bit17|bit18 — 줄의 첫/마지막 세그먼트. 구현 편의(bit31)가 아니어야
        // `render_paragraph_parts` 가 원본 캐시로 보고 방출한다.
        tag: 393216,
    }
}

fn one_cell_table() -> Table {
    Table {
        col_count: 1,
        row_count: 1,
        cells: vec![Cell {
            col: 0,
            row: 0,
            col_span: 1,
            row_span: 1,
            width: 20000,
            paragraphs: vec![text_para("표 안 문단")],
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// 02502 문단 0 의 슬롯 사다리를 그대로 옮긴 최소 문서.
///
/// `secd@0 · cold@8 · pgnp@16,24,32,40 · tbl@48 · 문단부호@56` = `char_count` 57,
/// 원본 lineseg `[0, 48]`.
fn section_first_paragraph_document() -> Document {
    let mut para = Paragraph {
        text: String::new(),
        char_count: 57,
        ..Default::default()
    };
    para.controls.push(Control::SectionDef(Box::new(secdef())));
    para.controls.push(Control::ColumnDef(ColumnDef::default()));
    for _ in 0..4 {
        para.controls
            .push(Control::PageNumberPos(PageNumberPos::default()));
    }
    para.controls
        .push(Control::Table(Box::new(one_cell_table())));
    para.line_segs = vec![line_seg(0), line_seg(48)];

    let mut section = Section {
        section_def: secdef(),
        ..Default::default()
    };
    section.paragraphs.push(para);
    section.paragraphs.push(text_para("둘째 문단"));

    let mut doc = Document::default();
    doc.doc_info.para_shapes = vec![ParaShape::default()];
    doc.doc_info.char_shapes = vec![CharShape::default()];
    doc.doc_properties.section_count = 1;
    doc.sections.push(section);
    doc
}

fn section_xml(doc: &Document) -> String {
    let bytes = rhwp::serializer::hwpx::serialize_hwpx(doc).expect("serialize hwpx");
    let mut zin = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip 열기");
    let mut out = String::new();
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).expect("zip 항목");
        let name = f.name().to_string();
        if name.starts_with("Contents/section") && name.ends_with(".xml") {
            let mut s = String::new();
            f.read_to_string(&mut s).expect("section xml 읽기");
            out.push_str(&s);
        }
    }
    out
}

fn textpos_values(xml: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for (idx, _) in xml.match_indices("<hp:lineseg ") {
        let tail = &xml[idx..];
        let Some(at) = tail.find("textpos=\"") else {
            continue;
        };
        let rest = &tail[at + "textpos=\"".len()..];
        let end = rest.find('"').expect("textpos 닫는 따옴표");
        out.push(rest[..end].parse().expect("textpos 는 정수"));
    }
    out
}

/// 방출되지 않는 `secd`·`cold` 두 슬롯만큼 축을 내려야 한다 — 48 이 아니라 32.
#[test]
fn section_first_paragraph_line_seg_rebases_to_the_hwpx_axis() {
    let xml = section_xml(&section_first_paragraph_document());
    let positions = textpos_values(&xml);

    assert!(
        positions.contains(&32),
        "구역 첫 문단 lineseg 가 HWPX 축으로 내려오지 않았다 (#5943 회귀). \
         `secd`·`cold` 는 HWPX 문단 축을 차지하지 않으므로 표는 48 이 아니라 32 다. \
         실측 textpos={positions:?}\n{xml}"
    );
    assert!(
        !positions.contains(&48),
        "HWP5 축 값 48 이 그대로 나갔다 — 한글 2024 가 이 문단부터 본문을 폐기한다. \
         실측 textpos={positions:?}"
    );
}

/// 첫 줄(0)은 그대로다 — 앞선 빈-방출 슬롯이 없으므로 내릴 것이 없다.
#[test]
fn the_first_line_keeps_position_zero() {
    let xml = section_xml(&section_first_paragraph_document());
    let positions = textpos_values(&xml);
    assert!(
        positions.first() == Some(&0),
        "첫 줄의 textpos 가 0 이 아니다 — 재기준화가 앞선 슬롯 수를 잘못 셌다. \
         실측 textpos={positions:?}"
    );
}

/// 구역 정의가 없는 평범한 문단은 축을 건드리지 않는다.
#[test]
fn a_plain_paragraph_axis_is_untouched() {
    let mut para = text_para("가나다라마바사아자차카타파하");
    para.line_segs = vec![line_seg(0), line_seg(7)];

    let mut section = Section {
        section_def: secdef(),
        ..Default::default()
    };
    // 구역 첫 문단이 secd/cold 를 흡수하므로, 시험 대상은 둘째 문단에 둔다.
    section.paragraphs.push(text_para("구역 첫 문단"));
    section.paragraphs.push(para);

    let mut doc = Document::default();
    doc.doc_info.para_shapes = vec![ParaShape::default()];
    doc.doc_info.char_shapes = vec![CharShape::default()];
    doc.doc_properties.section_count = 1;
    doc.sections.push(section);

    let positions = textpos_values(&section_xml(&doc));
    assert!(
        positions.contains(&7),
        "빈-방출 슬롯이 없는 문단의 textpos 가 움직였다 — 재기준화가 과잉 적용됐다. \
         실측 textpos={positions:?}"
    );
}

/// [#5961] 재기준화는 **출처를 묻지 않는다**.
///
/// #5943 때는 HWPX 출처의 `textpos` 가 이미 HWPX 축이라 예외로 건너뛰어야 했다. 이제
/// HWPX 파서가 읽을 때 HWP5 축으로 올리므로 IR 은 출처와 무관하게 언제나 HWP5 축이고,
/// 내보낼 때는 언제나 내린다. 같은 IR 을 출처만 바꿔 내도 결과가 같아야 한다 — 축이
/// 출처에 따라 갈리면 #5961 이 없애려던 조건 분기가 되살아난 것이다.
#[test]
fn the_rebase_does_not_depend_on_the_source_format() {
    let hwp5_positions = textpos_values(&section_xml(&section_first_paragraph_document()));

    let mut doc = section_first_paragraph_document();
    doc.provenance.format = rhwp::model::provenance::SourceFormat::Hwpx;
    let hwpx_positions = textpos_values(&section_xml(&doc));

    assert_eq!(
        hwp5_positions, hwpx_positions,
        "출처만 바꿨는데 textpos 가 갈렸다 — 축이 IR 안에서 통일되지 않았다(#5961)."
    );
    assert!(
        hwpx_positions.contains(&32),
        "HWP5 축 48 이 HWPX 축 32 로 내려가지 않았다. 실측 textpos={hwpx_positions:?}"
    );
}
