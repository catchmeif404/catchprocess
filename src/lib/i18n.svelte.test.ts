// i18n 사전 검증: 두 로케일 모두 같은 키를 가지며, 파라미터 치환이 동작해야 한다.
import { beforeEach, describe, expect, it } from 'vitest';
import { getLocale, setLocale, t } from './i18n.svelte';

describe('i18n', () => {
  beforeEach(() => {
    setLocale('en');
  });

  it('로케일을 바꾸면 같은 키의 문자열이 바뀐다', () => {
    expect(t('empty')).toBe('no dev services detected');

    setLocale('ko');
    expect(getLocale()).toBe('ko');
    expect(t('empty')).toBe('감지된 dev 서비스 없음');
  });

  it('{param} 플레이스홀더를 치환한다', () => {
    setLocale('en');
    expect(t('servicesUp', { count: 4 })).toBe('4 running');

    setLocale('ko');
    expect(t('servicesUp', { count: 4 })).toBe('4개 실행 중');
  });

  it('여러 파라미터와 숫자를 치환한다', () => {
    setLocale('en');
    expect(t('cardAria', { process: 'node', ports: '5173' })).toBe('node on port 5173');
  });

  it('모든 로케일이 같은 키 집합을 가진다', async () => {
    // 새 문자열 추가 시 한쪽 로케일만 채우는 실수를 잡는다.
    const { MESSAGES } = await import('./i18n.svelte');
    const enKeys = Object.keys(MESSAGES.en).sort();
    const koKeys = Object.keys(MESSAGES.ko).sort();
    expect(koKeys).toEqual(enKeys);
  });
});
