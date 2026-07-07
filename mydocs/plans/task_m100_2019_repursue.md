# #2019 재추진 계획서 (올바른 모델 확정 후) — 부동 폼 별지 서식 과분할

- 이슈 #2019 / 재추진 브랜치: `task/2019-repursue` (origin/devel 기준, Stage1 하네스 + Stage2 findings 포함)
- 재현: `samples/hwpx/issue2019_floating_form_74312.hwpx` (rhwp 81p vs 한글 18p)
- 선행 근거: #2019 이슈 코멘트(올바른 모델 확정), `task_m100_2019_stage2.md`(다층 RCA)

## 0. 이전 시도 교훈 (반드시 준수)

1. **페이지 수 정합 ≠ 정렬 정합.** 반드시 **수정 바이너리로 pi-page 오라클**(한글 SetPos+current_page ↔
   rhwp dump-pages)을 돌려 **pi별 페이지 정렬(PI_MISMATCH/PAGE_DELTA)**을 검증한다. 첫 시도는 페이지
   수(18)만 보고 "완성" 오판 → 내용 붕괴(PI_MISMATCH 154).
2. **"부동 앵커 흐름 footprint 0" 접근은 오답.** stored vpos는 정확히 누적(vpos[N+1]=vpos[N]+lh[N])
   이라 **개체 높이는 정상 흐름의 일부**다. 높이를 0으로 만들면 내용이 붕괴/시프트된다.
3. 오라클/paramap 도구: `output/poc/survey10k_r8_0707/run_oracle.py`(--list), `hwp_paramap.py`
   (문단별 한글 페이지), 하네스 `output/poc/task2019/capture_pages.py` + baseline.tsv.

## 1. 확정된 올바른 모델 (한글 paramap ↔ rhwp vpos 대조)

- stored LINE_SEG vpos = **섹션 누적 흐름 좌표**(개체 높이 포함). 폼은 겹치지 않고 순차 stack(~17p).
- 한글 페이지 경계 18개 = **[쪽나누기] 17개(16 정확 일치) + 자연 오버플로 1개(pi117)**.
- **[단나누기] 71개 + ColumnDef(1↔2↔3단) zone 전환은 페이지 분할을 하지 않는다**(같은 페이지에 zone stack).
- 목표 동작: **높이 유지 + 쪽나누기/자연 오버플로에서만 분할 + 단나누기·zone 전환 허위 분할 억제.**

## 2. 구현 스코프 (수정 지점, typeset.rs)

세 가지는 이미 국소 검증됨(81→18~19 수렴, 폼 렌더 정상, 무회귀 0):

- **② 단나누기**(단일 단 + ColumnDef 동반 폼 구분자, `has_diff_col_def=false`): 페이지 분할 억제.
  (col-break 핸들러 ~2270)
- **③-a vpos_zone_height 교정**: leaving zone 높이는 실제 흐름 위치 `current_height`와 같아야 한다.
  page-상대 stored vpos(max_vpos_px)가 `current_height + one_line` 을 크게 초과하면(누적 vpos/쪽분할
  직후 신호) `current_height` 사용. (process_multicolumn_break ~13931)

## 3. ★★난제 ⑤ (최심층, 진짜 벽) — Paper-앵커 개체 extent vs 압축 vpos flow

**추가 계측(2차 재추진)으로 ④보다 깊은 근본원인 확정.** ②③-a+④(zone-offset 누적 + pad 제외 +
마진 최소)를 구현해 zone 전환 허위 분할은 **전부 제거**(81→22p, zone break 0)했으나, **자연 오버플로
타이밍이 한글과 어긋난다**(p7 pi95~137 과밀 = pi117 오버플로 놓침; pi212/258/271/320/377 과다 분할).

원인: **stored vpos flow 가 실제 렌더 높이보다 압축**되어 있다. 예) 한글 p7 = pi95~116(vpos delta
pi95→117 = 28985HU ≈ 386px)가 **한 페이지(1027px)를 꽉 채운다** → vpos 386px 가 렌더 1027px 에
해당(≈2.6×). Paper(용지)-앵커 절대 개체는 vpos flow 를 진행시키지 않고 **절대 위치에 full 높이로
렌더**되기 때문이다(반면 pi86 글상자처럼 vpos 를 개체높이만큼 진행시키는 개체도 섞여 있음 = 이질적).

**결론**: 올바른 페이지네이션은 **Paper-앵커 개체의 실제 렌더 extent(절대 Y 상·하한) 기준**으로
페이지를 나눠야 한다. 압축된 vpos flow 로 높이를 세는 현재 typeset 경로로는 불가능. 이는 **절대위치
개체 extent 기반 페이지네이션**이라는 신규 레이아웃 기능(#2004 이상 규모)이다. 흑박스 계측의 한계.

**재추진 시 우선 조사**: (a) 각 개체가 vpos 를 진행시키는지(Para-앵커) vs 진행 안 시키는지(Paper-앵커
절대) 판별 규칙, (b) 페이지네이션에서 Paper-앵커 개체의 절대 Y extent 를 페이지 분할 기준에 반영,
(c) Para-앵커/인라인 개체는 기존 vpos flow 유지. 한글 렌더(개체별 실제 Y)를 오라클로 대조 필요.

## 4. 난제 ④ — zone offset 누적 (해결됨, 참고)

한 페이지에 여러 ColumnDef zone이 stack될 때(p7 = pi95~137, ~6 zone) **zone-offset 누적이
`current_height` 리셋(typeset.rs:14082 `current_height = 0.0`)으로 끊긴다**. 그 결과 각 zone이
offset≈0 에 겹쳐 페이지가 안 차고, **pi117 자연 오버플로 분할을 놓쳐** pi95~137 을 한 쪽에 과밀 →
pi117~270 off-by-1.

- 핵심: **zone 전환 시 페이지 내 누적 y = (직전까지의 zone offset) + (leaving zone 흐름 높이)** 를
  정확히 유지하고, 이 누적이 본문 높이를 넘을 때만 분할해야 한다.
- 난이도: `process_multicolumn_break`는 #853/#866/#874/#702 등 다단 특수케이스(디자인 spacing,
  헤더 띠 tac_band, solo_zone_pad, Distribute 균형)가 얽혀 있다. candidate_offset 에 pad 를 71회
  누적하면 과분할, current_height 로 리셋하면 누적 손실 → **양쪽을 정합시키는 재설계 필요**.
- 부수 확인 사항: Distribute(배분) 2단 zone의 전폭(가로=용지 155~175mm) 절대 도형이 균형 컬럼(짧음)에
  안 맞아 조기 컬럼/페이지 분할되는 것도 동일 zone 처리에서 함께 정합 필요(avail = body − zone_off,
  zone_off 팽창이 원인이었음).

## 5. 단계 (예정 5단계)

1. **Stage A — 오라클 하네스 정비**: 74312 pi-page 오라클 + 다단 회귀 샘플(shortcut/hwpspec/exam)
   pi-page 기준선 캡처. before/after 정렬 대조 스크립트.
2. **Stage B — ②③-a 재적용**: 단나누기 억제 + vpos_zone_height 교정. 오라클로 정렬 확인(부분 개선).
3. **Stage C — ④ zone-offset 누적 재설계**: 페이지 내 누적 y 를 zone 전환 전반에서 정확히 유지 +
   자연 오버플로 분할. 74312 pi-page **MATCH** 목표(pi117 자연분할 복원, pi271 허위분할 제거).
4. **Stage D — 광범위 무회귀**: 80 baseline + 300 랜덤 + MORE 44 페이지수 불변 + **다단 통합
   테스트 전건**(exam_eng/1082/1156/1375/1488) + svg_snapshot/opengov 시각 스냅샷 그린.
5. **Stage E — 회귀테스트 + 최종보고**: `tests/issue_2019_*.rs`(페이지수 + pi-page 정렬 assert).

## 6. 승인 요청

위 재추진 계획(확정 모델 기준, ④ 집중)으로 진행 승인 요청합니다. 승인 시 Stage A(오라클 하네스)부터
착수하며, **각 단계마다 pi-page 오라클로 정렬을 검증**하고 보고합니다. 난제 ④는 다단 hot-path
재설계이므로 광범위 회귀검증(Stage D) 통과를 머지 조건으로 합니다.
