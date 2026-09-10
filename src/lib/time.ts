// 스냅샷 시각 표시 유틸. 로케일 태그를 받아 지역 시각 시:분:초로 포맷한다.

const TIME_FORMAT_OPTIONS = {
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
} as const satisfies Intl.DateTimeFormatOptions;

export function localeTag(locale: 'en' | 'ko'): string {
  return locale === 'ko' ? 'ko-KR' : 'en-US';
}

/** epoch 밀리초를 지역 시각 "HH:MM:SS" 형태로 변환한다. */
export function formatClock(epochMs: number, locale: 'en' | 'ko'): string {
  return new Date(epochMs).toLocaleTimeString(localeTag(locale), TIME_FORMAT_OPTIONS);
}
