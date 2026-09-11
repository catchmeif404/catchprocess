// 위젯 UI 문자열 사전 (EN/KO). 워크스페이스 이중언어 규칙에 따라 v0.1부터 제공.
// 런타임 상태가 있으므로 Svelte 5 runes 모듈(.svelte.ts)로 작성한다.

export type Locale = 'en' | 'ko';

/** 로케일별 UI 문자열. 새 문자열 추가 시 두 로케일을 모두 채운다. */
export const MESSAGES = {
  en: {
    title: 'DevTopology',
    servicesUp: 'subjects: {count}',
    updatedAt: 'filed {time}',
    empty: 'the desk is clear.',
    searchPlaceholder: 'search services…',
    noMatches: 'nothing matches.',
    scanFailed: 'scan failed — retrying…',
    cardAria: '{process} on port {ports}',
    close: 'close widget',
    language: 'language',
    switchToEnglish: 'switch to English',
    switchToKorean: 'switch to Korean',
    stop: 'stop service',
    stopping: 'stopping…',
    stopFailed: 'stop failed',
    connections: 'connections',
    connectedTo: '→ {target}:{port}',
    apiTarget: 'API → {target}:{port}',
  },
  ko: {
    title: 'DevTopology',
    servicesUp: '감시 대상 {count}개',
    updatedAt: '보고 시각 {time}',
    empty: '책상이 깨끗하네.',
    searchPlaceholder: '서비스 검색…',
    noMatches: '일치하는 게 없네.',
    scanFailed: '스캔 실패 — 재시도 중…',
    cardAria: '{process} 포트 {ports}',
    close: '위젯 닫기',
    language: '언어',
    switchToEnglish: '영어로 전환',
    switchToKorean: '한국어로 전환',
    stop: '서비스 중지',
    stopping: '중지 중…',
    stopFailed: '중지 실패',
    connections: '연결',
    connectedTo: '→ {target}:{port}',
    apiTarget: 'API → {target}:{port}',
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
