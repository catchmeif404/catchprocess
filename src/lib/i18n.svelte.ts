// 위젯 UI 문자열 사전 (EN/KO). 워크스페이스 이중언어 규칙에 따라 v0.1부터 제공.
// 런타임 상태가 있으므로 Svelte 5 runes 모듈(.svelte.ts)로 작성한다.

export type Locale = 'en' | 'ko';

/** 로케일별 UI 문자열. 새 문자열 추가 시 두 로케일을 모두 채운다. */
export const MESSAGES = {
  en: {
    title: 'DevTopology',
    servicesUp: '{count} running',
    updatedAt: 'updated {time}',
    empty: 'no dev services detected',
    scanFailed: 'scan failed — retrying…',
    cardAria: '{process} on port {ports}',
    close: 'close widget',
    stop: 'stop service',
    confirmStop: 'confirm?',
    stopping: 'stopping…',
    stopFailed: 'stop failed',
  },
  ko: {
    title: 'DevTopology',
    servicesUp: '{count}개 실행 중',
    updatedAt: '{time} 갱신',
    empty: '감지된 dev 서비스 없음',
    scanFailed: '스캔 실패 — 재시도 중…',
    cardAria: '{process} 포트 {ports}',
    close: '위젯 닫기',
    stop: '서비스 중지',
    confirmStop: '정말?',
    stopping: '중지 중…',
    stopFailed: '중지 실패',
  },
} as const;

export type MessageKey = keyof (typeof MESSAGES)['en'];

function detectLocale(): Locale {
  // 브라우저(웹뷰) 언어로 초기 로케일을 정한다. ko 계열이 아니면 en.
  return typeof navigator !== 'undefined' && navigator.language.startsWith('ko')
    ? 'ko'
    : 'en';
}

let locale = $state<Locale>(detectLocale());

export function getLocale(): Locale {
  return locale;
}

export function setLocale(next: Locale): void {
  locale = next;
}

/** 현재 로케일의 문자열을 반환한다. {param} 플레이스홀더를 치환한다. */
export function t(key: MessageKey, params: Record<string, string | number> = {}): string {
  let text: string = MESSAGES[locale][key];
  for (const [name, value] of Object.entries(params)) {
    text = text.replaceAll(`{${name}}`, String(value));
  }
  return text;
}
