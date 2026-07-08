# Stage A2 조사 보고 — 측정은 정확, 갭은 중첩셀 유닛 분할 (pivot)

- 브랜치: `task/2019-repursue-impl` / 앵커: #2007 42065 pi=7

## ★ Pivot: A2(측정 교정)는 불필요 — 측정은 이미 정확

`RHWP_CELLMEAS` 계측으로 확정:
```
CELLMEAS r=0 nested=true text_h=8164 nested_bottom=1367 content_h=8164 declared_h=791 paras=135
```
- pi=7 중첩셀 content_height = **8164px** (실제 콘텐츠, 압축 아님). declared 791px(압축)를 **안 씀**.
- `height_measurer.rs:1024-1042`가 `cell_nested_controls_bottom`으로 중첩 표를 **재귀 실측**(measure_table_impl)
  → text_h=8164 정상. **측정은 버그 아님.**

## 진짜 갭: split이 8164px 단일행을 못 쪼갬

- pi=7 = 1×1 RowBreak 표. 표는 8164px로 측정되나 페이지(895px)에 통째 배치 → 콘텐츠 오버플로.
- `advance_row_cut`(table_layout.rs:5622)은 `cell_units`로 셀을 유닛(문단) 분해해 컷.
- pi=7 외부 셀은 `preserve_linear_single_cell_vpos`(1×1 RowBreak, voff=0) 경로.
- **결정적**: 외부 셀 콘텐츠(135문단)가 **중첩 표(표 안의 표) 안**에 있어, `cell_units_uncached`가
  이를 **단일 atomic 유닛**으로 취급(중첩 표 콘텐츠를 splittable 유닛으로 flatten 안 함).
  → 외부 셀에 쪼갤 문단 경계가 사실상 1개 → 통째 배치 → under-pagination.
- 대조: pi=2(2×1, **직접** 셀)는 문단이 유닛으로 분해되어 start_cut=[9],[13]로 정상 3분할.

## 진짜 수정 방향 (Stage A3, 본체)

`cell_units_uncached`(table_layout.rs:4607)가 **중첩 표만 담긴 셀**을 만나면 중첩 표의 셀 문단을
**재귀적으로 flatten**해 splittable 유닛으로 산출. 그러면 `advance_row_cut`이 중첩셀 135문단을
페이지 경계로 컷 가능 → 42065 6p→~17p.

## 위험 평가 (정직)

- `cell_units`/`advance_row_cut`은 **전 표 분할의 최핵심 hot-path**(#700/#1025/#1105/#1486/#874 등
  수십 특수케이스 얽힘). 중첩 flatten은 rowspan·블록컷·vpos 동기화와 상호작용 → **광범위 표 회귀
  위험 큼**. #2019 8c46ca2(잘못된 수정 머지)의 교훈 상, 게이트 정밀 한정 + 표 통합 테스트 전수 +
  코퍼스 랜덤 회귀 필수.
- 이는 **신규 기능(중첩 표 셀 콘텐츠 페이지 분할)** 구현이며, 며칠 규모의 전용 작업 + 검증.

## ★ 정확한 수정 지점 + 왜 substantial 한가 (최종 특성화)

- **수정 지점: `table_layout.rs:4933`** — `nested_tables[0].row_count >= 2` 조건이 **1×1 단일 셀
  중첩 표를 splittable 유닛 분해에서 제외**. pi=7의 1×1 중첩 표(135문단 8164px)는 이 조건 탈락 →
  atomic 폴백(5169) → 통째 배치.
- per-중첩행 경로(4933-4989)는 각 중첩 **행**을 `nested_row: Some(ri)` 유닛으로 산출(2행+ 전용).
- **1×1은 행이 1개라 행 분해 불가 → 그 단일 셀의 문단을 쪼개야 함.**

### 왜 단순 재귀가 안 되는가 (핵심)
- `cell_units_uncached`를 중첩 셀에 재귀 호출하면 유닛의 `para_idx`가 **중첩 셀 문단 인덱스**가
  되는데, `advance_row_cut`/렌더는 `para_idx`를 **외부 셀 문단 인덱스**로 해석 → 불일치.
- per-중첩행은 `nested_row: Some(ri)`로 "중첩 행 ri 렌더"를 표시하나, **"중첩 셀 문단 조각 [a,b)
  렌더"를 위한 CellUnit 변형·렌더 지원이 없음.**
- 즉 수정 = **CellUnit 모델 확장(nested-cell-paragraph fragment: 중첩 셀 + 문단 범위) + 렌더러가
  그 조각을 그리도록 지원** = 신규 기능(다일 규모), 게이트 한정 + 광범위 표 회귀검증 필수.

## 판정 (정직)

deep-class 재설계를 **분석 수준에서 완전 규명·de-risk 완료**:
- A1: 통일 근본원인(stored vpos ≠ 실제 extent).
- A2: 측정은 정확(버그 아님), 갭은 split의 중첩셀 유닛 분해.
- 정확한 지점(table_layout.rs:4933) + 정확한 이유(1×1 제외) + 구현 난이도 근거(CellUnit para_idx
  외부기준 → nested-cell-paragraph fragment 변형 신설 필요).

**구현은 CellUnit 모델 확장 + 렌더 지원의 실질 기능**으로, 며칠 규모 전용 작업 + 표 전수 회귀검증.
blind 프로토타입은 para_idx 불일치로 렌더 붕괴 위험 → 모델 확장 없이는 부적합. 소스 미수정 유지.
**권장: 이 특성화를 기반으로, 별지/셀 콘텐츠 페이지네이션 전용 타스크에서 CellUnit fragment 모델
확장부터 단계적 구현.**
