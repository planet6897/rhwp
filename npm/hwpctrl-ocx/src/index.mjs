/**
 * `@rhwp/hwpctrl` — 웹한글컨트롤(WebHwpCtrl) API v2.4 호환 층.
 *
 * 계약의 출처는 규격서(`spec/webhwpctrl_api.json`)와 **설치된 한글의 실측**이다. 문서가
 * 모호한 자리(서명과 `Parameters N` 이 어긋나는 18건 등)는 오라클이 답한 대로 맞춘다.
 * 대조 하니스: `tools/hwpctrl_compat/`.
 *
 * ## 이 파일이 지키는 규칙
 *
 * - **반환형을 규격대로 돌려준다.** `PutFieldText`·`RenameField` 는 값을 돌려주지 않는다
 *   (오라클 `null`). "성공했으니 true" 는 규격 위반이고, 기존 studio 층이 그렇게 했다.
 * - **없는 것을 있는 척하지 않는다.** 아직 못 하는 API 는 규격의 실패값(`false`/`''`/`-1`)을
 *   돌려주고 `console.warn` 으로 이유를 남긴다.
 * - **브라우저 제약은 규격이 이미 답을 정해 놓았다**(v2.4 §2.2). `Open` 은 업로드된 File,
 *   `SaveAs` 는 다운로드다. Node 에서 돌릴 때는 호스트가 넣어 준 `onSave` 싱크를 쓴다.
 */

/** 필드 목록 구분자 — 규격 §8.3.9. 마지막 필드에는 붙지 않는다. */
const SEP = String.fromCharCode(2);

/** `SetFieldViewOption` 값. 표시 전용이라 문서를 바꾸지 않는다. */
const FIELD_VIEW_DEFAULT = 0;

function parseJson(raw, fallback) {
  try {
    return JSON.parse(raw);
  } catch {
    return fallback;
  }
}

/** `name`, `name{{3}}` 두 표기를 (이름, 순번)으로 가른다. */
function splitOccurrence(token) {
  const m = /^(.*?)\{\{(\d+)\}\}$/.exec(token);
  if (m) return { name: m[1], occurrence: Number(m[2]) };
  return { name: token, occurrence: 0 };
}

export class HwpCtrl {
  #wasm;
  #doc;
  #onSave;
  #cursor = { list: 0, para: 0, pos: 0 };
  #fieldViewOption = FIELD_VIEW_DEFAULT;
  #listeners = new Map();

  constructor({ wasm, doc, onSave } = {}) {
    this.#wasm = wasm;
    this.#doc = doc ?? (wasm ? wasm.HwpDocument.createEmpty() : null);
    this.#onSave = onSave;
  }

  /** 내부: 현재 문서. 하니스와 호스트가 쓴다. */
  getWasmDoc() {
    return this.#doc;
  }

  // ── 문서 관리 (규격 §8.3.1, 8.3.22, 8.3.33, 8.3.39, 8.3.50~52) ──

  /**
   * 문서 열기. 규격 §8.3.33 — 반환값은 **인자가 제대로 들어왔는지**에 대한 답이고,
   * 실제 성공 여부는 콜백 인자로 온다.
   *
   * 브라우저에서는 업로드된 `File`, Node 에서는 바이트 배열을 받는다.
   */
  Open(source, format, arg, callback, callbackUserData) {
    if (source == null) {
      callback?.(false, callbackUserData);
      return false;
    }
    try {
      const bytes = this.#toBytes(source);
      if (!bytes) {
        // File 은 비동기로만 읽을 수 있다 — 규격이 콜백을 둔 이유다.
        source
          .arrayBuffer()
          .then((buf) => {
            this.#doc = new this.#wasm.HwpDocument(new Uint8Array(buf));
            this.#resetCursor();
            callback?.(true, callbackUserData);
          })
          .catch((e) => {
            console.warn('[hwpctrl] Open 실패:', e);
            callback?.(false, callbackUserData);
          });
        return true;
      }
      this.#doc = new this.#wasm.HwpDocument(bytes);
      this.#resetCursor();
      callback?.(true, callbackUserData);
      return true;
    } catch (e) {
      console.warn('[hwpctrl] Open 실패:', e);
      callback?.(false, callbackUserData);
      return false;
    }
  }

  /** 규격 §8.3.50 — `Open` 의 간소화판. */
  OpenDocument(path, format, callback) {
    return this.Open(path, format, '', callback);
  }

  /**
   * 규격 §8.3.39 — 브라우저에서는 **다운로드**다(v2.4 §2.2). 파일 이름만 지정할 수 있다.
   * Node 에서는 호스트가 넣어 준 `onSave(bytes, fileName)` 싱크로 흘린다.
   */
  SaveAs(fileName, format, arg, callback, callbackUserData) {
    try {
      const bytes = this.#exportBytes(format, fileName);
      if (!bytes) return false;
      if (this.#onSave) {
        this.#onSave(bytes, fileName);
      } else if (typeof document !== 'undefined') {
        this.#download(bytes, fileName);
      } else {
        console.warn('[hwpctrl] SaveAs: 저장 싱크가 없다 (onSave 미지정)');
        return false;
      }
      callback?.(true, callbackUserData);
      return true;
    } catch (e) {
      console.warn('[hwpctrl] SaveAs 실패:', e);
      callback?.(false, callbackUserData);
      return false;
    }
  }

  /** 규격 §8.3.51 — `SaveAs` 의 간소화판. */
  SaveDocument(fileName, format, callback) {
    return this.SaveAs(fileName, format, '', callback);
  }

  /** 규격 §8.3.1 — 문서를 닫고 빈 문서로 만든다. */
  Clear(option) {
    try {
      this.#doc = this.#wasm.HwpDocument.createEmpty();
      this.#resetCursor();
    } catch (e) {
      console.warn('[hwpctrl] Clear 실패:', e);
    }
  }

  /** 규격 §8.3.22 — 문서 끼워넣기. 아직 구현하지 않았다. */
  Insert(path, format, arg, callback, callbackUserData) {
    console.warn('[hwpctrl] Insert: 미구현 (문서 끼워넣기)');
    callback?.(false, callbackUserData);
    return false;
  }

  /** 규격 §8.3.52 — `Insert` 의 간소화판. */
  InsertDocument(path, callback) {
    return this.Insert(path, '', '', callback);
  }

  /** 규격 §8.3.66 — 브라우저 인쇄 대화상자. */
  PrintDocument() {
    if (typeof window !== 'undefined' && typeof window.print === 'function') {
      window.print();
      return;
    }
    console.warn('[hwpctrl] PrintDocument: 브라우저 밖에서는 할 일이 없다');
  }

  // ── 필드 (규격 §8.3.3, 8.3.7~10, 8.3.29, 8.3.34, 8.3.36, 8.3.41~42) ──

  /**
   * 규격 §8.3.9 — 필드 이름을 `0x02` 로 이어 붙인 **문자열**을 돌려준다.
   *
   * - `number` 가 1 이면 이름 뒤에 `{{순번}}` 을 붙인다.
   * - `option` 이 2 이면 **안내문을 가진 누름틀만** 준다(오라클 실측: 165개 중 14개이고,
   *   그 14개는 안내문이 있는 필드와 정확히 일치한다).
   */
  GetFieldList(number = 0, option = 0) {
    const fields = this.#fields();
    const picked = option === 2 ? fields.filter((f) => f.guide) : fields;
    const seen = new Map();
    return picked
      .map((f) => {
        const n = seen.get(f.name) ?? 0;
        seen.set(f.name, n + 1);
        return number === 1 ? `${f.name}{{${n}}}` : f.name;
      })
      .join(SEP);
  }

  /** 규격 §8.3.7 — 존재 여부. 순번 접미사(`이름#0`)는 오라클이 받지 않는다. */
  FieldExist(field) {
    if (typeof field !== 'string' || !field) return false;
    return this.#fields().some((f) => f.name === field);
  }

  /** 규격 §8.3.10 — 여러 필드를 `0x02` 로 묶어 물으면 같은 순서로 돌려준다. */
  GetFieldText(fieldlist) {
    if (typeof fieldlist !== 'string' || !fieldlist) return '';
    return fieldlist
      .split(SEP)
      .map((token) => this.#fieldValue(token))
      .join(SEP);
  }

  /**
   * 규격 §8.3.34 — **반환값이 없다.** 현재 필드 내용은 지워지고 새 값이 들어간다.
   * 필드 개수와 텍스트 개수는 같아야 하며, 없는 필드는 무시한다.
   */
  PutFieldText(fieldlist, textlist) {
    if (typeof fieldlist !== 'string' || !fieldlist) return;
    const names = fieldlist.split(SEP);
    const values = typeof textlist === 'string' ? textlist.split(SEP) : [];
    names.forEach((token, idx) => {
      const value = values[idx] ?? '';
      const { name } = splitOccurrence(token);
      try {
        const raw = this.#doc.setFieldValueByName(name, value);
        const parsed = parseJson(raw, { ok: false });
        if (!parsed.ok) console.warn(`[hwpctrl] PutFieldText("${name}") 실패`);
      } catch (e) {
        // 없는 필드는 무시한다 — 규격 §8.3.34 Remarks.
        console.warn(`[hwpctrl] PutFieldText("${name}"): ${e}`);
      }
    });
  }

  /** 규격 §8.3.8 — 캐럿이 든 필드의 이름. 없으면 빈 문자열. */
  GetCurFieldName(option = 0) {
    try {
      const raw = this.#doc.getFieldInfoAt(
        this.#cursor.list,
        this.#cursor.para,
        this.#cursor.pos,
      );
      const parsed = parseJson(raw, null);
      return parsed?.name ?? '';
    } catch {
      return '';
    }
  }

  /** 규격 §8.3.41 — 캐럿 위치의 필드 이름을 바꾼다(없으면 만든다). */
  SetCurFieldName(fieldname, option, direction, memo) {
    const current = this.GetCurFieldName(0);
    if (current) return this.#renameField(current, fieldname);
    return this.CreateField(direction ?? '', memo ?? '', fieldname);
  }

  /** 규격 §8.3.3 — 캐럿 위치에 누름틀을 만든다. */
  CreateField(direction, memo, name) {
    try {
      const raw = this.#doc.insertClickHereField(
        this.#cursor.list,
        this.#cursor.para,
        this.#cursor.pos,
        direction ?? '',
        memo ?? '',
        name ?? '',
        true,
      );
      return parseJson(raw, { ok: false }).ok === true;
    } catch (e) {
      console.warn('[hwpctrl] CreateField 실패:', e);
      return false;
    }
  }

  /** 규격 §8.3.36 — **반환값이 없다.** */
  RenameField(oldname, newname) {
    this.#renameField(oldname, newname);
  }

  /**
   * 규격 §8.3.29 — 필드 속성 비트를 지우고(remove) 더한다(add).
   * 음수는 오류를 뜻한다. 아직 편집 가능 비트만 다룬다.
   */
  ModifyFieldProperties(field, remove, add) {
    const target = this.#fields().find((f) => f.name === field);
    if (!target) return -1;
    if (!remove && !add) return 1; // 조회만 — 오라클 실측 반환값
    try {
      const raw = this.#doc.updateClickHereProps(
        target.fieldId,
        target.guide ?? '',
        target.memo ?? '',
        target.name,
        (target.editableInForm && !remove) || Boolean(add),
      );
      return parseJson(raw, { ok: false }).ok === true ? 1 : -1;
    } catch (e) {
      console.warn('[hwpctrl] ModifyFieldProperties 실패:', e);
      return -1;
    }
  }

  /** 규격 §8.3.42 — 표시 옵션. 설정된 값을 그대로 돌려준다(오라클 실측). */
  SetFieldViewOption(option) {
    if (typeof option !== 'number') return 0;
    this.#fieldViewOption = option;
    return option;
  }

  // ── 커서·문서 정보 (P2 축이지만 시나리오가 딛고 서야 한다) ──

  /** 규격 §8.2.10 — 전체 쪽수. */
  PageCount() {
    try {
      return this.#doc.pageCount();
    } catch {
      return 0;
    }
  }

  /** 규격 §8.3.12 — 웹은 객체를 돌려준다. */
  GetPos() {
    return { ...this.#cursor };
  }

  /** 규격 §8.3.43. */
  SetPos(list, para, pos) {
    this.#cursor = { list, para, pos };
    return true;
  }

  /** 규격 §8.3.31 — 필드로 이동. */
  MoveToField(field, text, start, select) {
    const target = this.#fields().find((f) => f.name === field);
    if (!target) return false;
    this.#cursor = {
      list: target.location?.sectionIndex ?? target.location?.section ?? 0,
      para: target.location?.paraIndex ?? target.location?.paragraph ?? 0,
      pos: 0,
    };
    return true;
  }

  /** 규격 §8.3.67 — 이벤트 등록. 발화는 아직 없다. */
  AddEventListener(eventType, listener) {
    if (!this.#listeners.has(eventType)) this.#listeners.set(eventType, []);
    this.#listeners.get(eventType).push(listener);
  }

  // ── 내부 ──

  #resetCursor() {
    this.#cursor = { list: 0, para: 0, pos: 0 };
  }

  #fields() {
    try {
      const parsed = parseJson(this.#doc.getFieldList(), []);
      return Array.isArray(parsed) ? parsed : (parsed.fields ?? []);
    } catch {
      return [];
    }
  }

  #fieldValue(token) {
    const { name } = splitOccurrence(token);
    try {
      const parsed = parseJson(this.#doc.getFieldValueByName(name), null);
      return parsed?.ok ? parsed.value : '';
    } catch {
      return '';
    }
  }

  #renameField(oldname, newname) {
    const target = this.#fields().find((f) => f.name === oldname);
    if (!target) return false;
    try {
      const raw = this.#doc.updateClickHereProps(
        target.fieldId,
        target.guide ?? '',
        target.memo ?? '',
        newname,
        target.editableInForm ?? true,
      );
      return parseJson(raw, { ok: false }).ok === true;
    } catch (e) {
      console.warn('[hwpctrl] RenameField 실패:', e);
      return false;
    }
  }

  #toBytes(source) {
    if (source instanceof Uint8Array) return source;
    if (source instanceof ArrayBuffer) return new Uint8Array(source);
    return null; // File — 비동기 경로로 넘긴다
  }

  #exportBytes(format, fileName) {
    const wanted = String(format ?? '').toLowerCase();
    const ext = String(fileName ?? '').toLowerCase();
    if (wanted === 'hwpx' || ext.endsWith('.hwpx')) return this.#doc.exportHwpx();
    if (wanted === 'hml' || ext.endsWith('.hml')) return this.#doc.exportHml();
    return this.#doc.exportHwp();
  }

  #download(bytes, fileName) {
    const blob = new Blob([bytes], { type: 'application/x-hwp' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = fileName || 'document.hwp';
    a.click();
    URL.revokeObjectURL(url);
  }
}

/** 하니스·호스트 공통 진입점. */
export function createHwpCtrl(options = {}) {
  return new HwpCtrl(options);
}
