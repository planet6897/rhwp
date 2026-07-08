# 재설계 계획서 — 절대개체/셀-extent 기반 페이지네이션 (deep-class 통합)

- 브랜치: `task/2019-repursue-impl` (fork/revert/2019-incomplete-fix 기준 = devel − 잘못된 #2019 수정)
- 대상 이슈(공통 근본원인): #2019, #2007, #2006, #1995 (+ 부분 #1921/#2017)
- 선행: #2019 RCA(vpos-flow 압축 / Paper-앵커 extent), #2007 RCA(intra-cell 미분할)

## 0. 철칙 (이전 실패 교훈)

1. **페이지 수 정합 ≠ pi 정렬 정합.** 매 단계 수정 바이너리로 **pi-page 오라클**(한글 SetPos ↔
   rhwp dump-pages) 정렬 검증. 페이지 수만 보고 "완성" 판단 금지(8c46ca2 오답 → revert #2035).
2. **stored vpos는 개체별로 의미가 다름**: 인라인/Para-앵커는 흐름 좌표, Paper-앵커 절대개체는
   절대 좌표(압축). 개체 종류별 판별 후 처리.
3. hot pagination 경로(process_multicolumn_break/Distribute #853/#866/#874, RowBreak split #1025)
   와 얽힘 → 광범위 회귀검증(다단·표 통합 테스트 + 코퍼스 랜덤 N) 통과를 머지 조건.

## 1. 하위 문제 분해 (측정 확정)

deep-class는 방향이 다른 3개 하위 문제:

| # | 하위 문제 | 대표 | 현행 | 방향 | 메커니즘 |
|---|----------|------|------|------|---------|
| A | **intra-cell 미분할** | #2007 42065 | 6 vs 17 | under | 1×1 RowBreak 표 중첩셀(135문단 8177px)을 선언높이(877px)로 배치, 콘텐츠 오버플로 미분할 |
| B | **부동 이미지/개체 under-pag** | #2006 1790387 | 130 vs 146 | under | 부동 이미지+텍스트가 한 쪽에 스택 |
| C | **부동 폼 over-pag** | #2019 74312 | 81 vs 18 | over | Paper-앵커 절대 vpos를 압축 흐름으로 계상 → 과분할 |

- A/B = under-pagination(콘텐츠 extent가 선언/vpos보다 큼 → 실제 콘텐츠 높이로 측정·분할 필요).
- C = over-pagination(절대 vpos를 흐름으로 오계상 → 절대배치·흐름 0 필요).
- **공통 근본**: **개체/셀의 "실제 렌더 extent"와 "stored vpos/선언높이"가 불일치**. 페이지네이션이
  후자를 쓰기 때문에 both-directions 오류.

## 2. 첫 앵커: A (intra-cell, #2007) — under-pagination 이 더 tractable

이유: (a) under 방향은 콘텐츠 extent 측정이 명확(문단 실높이 합산), (b) 기존 RowBreak split
메커니즘(start_cut/end_cut 문단 인덱스 컷, #1025 거대셀)이 **직접 셀엔 이미 동작**(pi=2 2×1은
문단 9/13에서 컷 분할). 갭은 **중첩 표(표 안의 표) 셀 콘텐츠**로 재귀 분할이 안 되는 것.

### 조사 확정 필요 (Stage A1)
- pi=7 effective_height 계산: 선언높이(877px) vs 콘텐츠높이(8177px) — 어느 것을 쓰는가.
- RowBreak split 진입 조건: pi=7이 split 루프에 들어가는가, 아니면 fit 판정으로 통째 배치되는가.
- 중첩 표 셀 콘텐츠 측정/분할이 `advance_row_cut`/셀 measure에서 재귀되는가.

## 3. 단계 (앵커 A 기준, 이후 B·C 확장)

- **Stage A1 — 오라클 하네스 + 메커니즘 확정**: deep-class repro 4종(42065/1790387/1613000/74312)
  pi-page 오라클 기준선 + A의 정확한 미분할 지점 계측. **소스 미수정.**
- **Stage A2 — intra-cell 콘텐츠 측정**: 중첩셀 문단 실높이 합산이 effective_height에 반영되게
  (선언높이 대신 콘텐츠 extent). 42065 오라클 개선 확인.
- **Stage A3 — intra-cell split**: 콘텐츠 extent가 페이지 초과 시 문단 경계로 셀 분할(중첩 표 재귀).
  42065 pi-page MATCH(±2) 목표. 광범위 회귀(표 통합 테스트 + 랜덤 N).
- **Stage A4 — 회귀테스트 + 보고**. 이후 B(#2006)·C(#2019)로 확장 스코프 재평가.

## 4. 승인/진행

자동승인 하에 Stage A1(하네스+계측, 소스 미수정)부터 착수. 각 단계 오라클 정렬 검증 + 회귀검증.
난제이므로 단계별 정직 보고(부분 진전/막힘 명시), blind 수정 금지.
