// ServiceCard 렌더링 검증: 프로젝트 이름(있으면)/프로세스/포트/PID 노출을 확인한다.
import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import ServiceCard from './ServiceCard.svelte';
import type { Service } from '$lib/types';

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
    // 명령줄은 말줄임 표시 대상이지만 툴팁(title)으로 풀 텍스트를 제공한다.
    expect(screen.getByTitle('java -jar build/libs/api.jar')).toBeInTheDocument();
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
});
