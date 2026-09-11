// ServiceCard 렌더링 검증: 프로젝트 이름(있으면)/프로세스/포트/PID 노출과
// 중지 버튼의 2단계 확인 흐름을 확인한다.
import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ServiceCard from './ServiceCard.svelte';
import { stopService } from '$lib/api';
import type { Service } from '$lib/types';

vi.mock('$lib/api', () => ({
  stopService: vi.fn(),
}));

const mockedStop = vi.mocked(stopService);

describe('ServiceCard', () => {
  it('프로젝트가 있으면 프로젝트 이름을 제목으로, 프로세스/포트/PID를 표시한다', () => {
    const service: Service = {
      pid: 55231,
      process: 'java',
      ports: [8080, 8081],
      command: 'java -jar build/libs/api.jar',
      project: {
        name: 'my-app-backend',
        path: '/Users/k/dev/my-app/backend',
        branch: 'feature/login',
      },
    };

    render(ServiceCard, { props: { service } });

    expect(screen.getByText('my-app-backend')).toBeInTheDocument();
    expect(screen.getByText(':8080 :8081')).toBeInTheDocument();
    expect(screen.getByText('java · #55231 · feature/login')).toBeInTheDocument();
    // 명령줄(폴더 경로 포함)은 화면에 표시하지 않는다 — API 데이터로만 존재.
    expect(screen.queryByText('java -jar build/libs/api.jar')).not.toBeInTheDocument();
  });

  it('프로젝트가 없으면 프로세스명이 제목이 된다', () => {
    const service: Service = { pid: 3332, process: 'postgres', ports: [5432] };

    render(ServiceCard, { props: { service } });

    expect(screen.getByText('postgres')).toBeInTheDocument();
    expect(screen.getByText(':5432')).toBeInTheDocument();
  });

  it('포트가 여러 개면 오름차순 공백 구분 라벨로 이어 붙인다', () => {
    const service: Service = { pid: 1, process: 'postgres', ports: [5432, 5433, 5434] };

    render(ServiceCard, { props: { service } });

    expect(screen.getByText(':5432 :5433 :5434')).toBeInTheDocument();
  });

  describe('중지 버튼', () => {
    beforeEach(() => {
      mockedStop.mockReset();
      mockedStop.mockResolvedValue(undefined);
    });

    const baseService: Service = { pid: 85610, process: 'node', ports: [3001] };

    it('확인 후에만 중지 신호를 보낸다', async () => {
      const onstop = vi.fn();
      render(ServiceCard, { props: { service: baseService, onstop } });

      await fireEvent.click(screen.getByRole('button', { name: 'stop service' }));
      expect(mockedStop).not.toHaveBeenCalled();
      await fireEvent.click(screen.getByRole('button', { name: 'Stop process' }));

      await waitFor(() => expect(mockedStop).toHaveBeenCalledTimes(1));
      await waitFor(() => expect(mockedStop).toHaveBeenCalledWith(85610));
      await waitFor(() => expect(onstop).toHaveBeenCalledWith(85610));
    });

    it('중지 실패 시 카드에 실패 표시를 낸다', async () => {
      mockedStop.mockRejectedValue(new Error('operation not permitted'));
      const onstop = vi.fn();
      render(ServiceCard, { props: { service: baseService, onstop } });

      await fireEvent.click(screen.getByRole('button', { name: 'stop service' }));
      await fireEvent.click(screen.getByRole('button', { name: 'Stop process' }));

      await waitFor(() => expect(screen.getByText('stop failed')).toBeInTheDocument());
      expect(onstop).not.toHaveBeenCalled();
    });
  });
  it('목록 숨기기는 프로세스를 종료하지 않는다', async () => {
    mockedStop.mockClear();
    const onhide = vi.fn();
    render(ServiceCard, { service: { pid: 42, process: 'node', ports: [3000] }, onhide });
    await fireEvent.click(screen.getByRole('button', { name: 'Hide from list' }));
    expect(onhide).toHaveBeenCalledOnce();
    expect(mockedStop).not.toHaveBeenCalled();
  });
});
