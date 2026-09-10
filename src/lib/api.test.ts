// API 클라이언트 검증: Tauri invoke를 모킹해 명령 이름/반환값 전달을 확인한다.
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fetchSnapshot } from './api';
import type { Snapshot } from './types';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

const fixture: Snapshot = {
  services: [{ pid: 44121, process: 'node', ports: [5173] }],
  generatedAt: 1760000000000,
  host: { os: 'macos', hostname: 'dev-machine' },
};

describe('fetchSnapshot', () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  it('get_services 명령을 호출하고 스냅샷을 반환한다', async () => {
    mockedInvoke.mockResolvedValue(fixture);

    const snapshot = await fetchSnapshot();

    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(mockedInvoke).toHaveBeenCalledWith('get_services');
    expect(snapshot).toEqual(fixture);
  });

  it('실패 시 예외를 그대로 전파한다 (폴링 UI가 실패 상태를 표시)', async () => {
    mockedInvoke.mockRejectedValue(new Error('scan failed'));

    await expect(fetchSnapshot()).rejects.toThrow('scan failed');
  });
});
