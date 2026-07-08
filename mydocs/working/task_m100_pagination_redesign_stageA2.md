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

## 판정

측정·근본원인 완전 규명 완료(A1: 통일 근본원인 / A2: 측정 정확, 갭은 중첩셀 유닛 분할). Stage A3
(중첩 flatten 분할)는 최심층 hot-path 신규 기능이라, 신중한 단계적 구현 + 광범위 회귀검증이 필수.
소스 미수정(조사·계측만). 다음: A3 프로토타입을 게이트 한정으로 시도하되 매 단계 오라클+회귀 검증.
