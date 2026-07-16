//! Issue #2279 / PR #2284 — footer(발신명의) 흡수/분할 페이지 수 직접 oracle.
//!
//! 성분② 수정(재구성 사다리 lead 판별 + 마진 62→50)의 대표 판정을 한글
//! 오라클 쪽수로 고정한다 (maintainer 리뷰 후속 요청):
//! - `36395825` (동대문소방서 소명자료): 한글 = **2쪽** (분할) — #2246 대표.
//!   수정 전 rhwp 1쪽(흡수 오판, footer 오차 성분①+②).
//! - `36376848` (의원면직 조회 회신): 한글 = **1쪽** (흡수).
//!   사다리 수정 단독 적용 시 2쪽으로 뒤집혔던 케이스 — 마진 재보정(50px)과
//!   결합해야 1쪽 유지 (압축-사다리 좌표계와 마진의 결합 회귀 검출).

use std::fs;
use std::path::Path;

fn page_count(sample: &str) -> u32 {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(repo_root).join(sample);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    rhwp::wasm_api::HwpDocument::from_bytes(&bytes)
        .unwrap_or_else(|e| panic!("parse {sample}: {e:?}"))
        .page_count()
}

#[test]
fn issue_2279_footer_split_36395825_is_two_pages() {
    assert_eq!(
        page_count("samples/task2279/36395825_gyeoljae.hwpx"),
        2,
        "36395825 한글=2쪽(발신명의 분할) — 1쪽이면 footer 흡수 오판 회귀 (#2246 대표)"
    );
}

#[test]
fn issue_2279_footer_absorb_36376848_is_one_page() {
    assert_eq!(
        page_count("samples/task2279/36376848_gyeoljae.hwpx"),
        1,
        "36376848 한글=1쪽(발신명의 흡수) — 2쪽이면 사다리/마진 결합 회귀 \
         (사다리 수정 단독 시 분할로 뒤집혔던 케이스)"
    );
}
