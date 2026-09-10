// 시각 포맷 유틸 검증.
import { describe, expect, it } from 'vitest';
import { formatClock } from './time';

// 2025-10-09T06:13:20Z. 지역 시각으로 바뀌므로 시:분:초 패턴만 검증한다.
const EPOCH_MS = 1760000000000;

describe('formatClock', () => {
  it('HH:MM:SS 형태의 문자열을 반환한다', () => {
    expect(formatClock(EPOCH_MS, 'en')).toMatch(/^\d{2}:\d{2}:\d{2}$/);
    expect(formatClock(EPOCH_MS, 'ko')).toMatch(/^\d{2}:\d{2}:\d{2}$/);
  });
});
