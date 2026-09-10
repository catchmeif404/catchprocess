// ServiceCard 렌더링 검증: 프로세스명/포트/PID가 카드에 그대로 노출되는지 확인.
import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import ServiceCard from './ServiceCard.svelte';
import type { Service } from '$lib/types';

describe('ServiceCard', () => {
  it('프로세스명, 포트, PID를 렌더링한다', () => {
    const service: Service = { pid: 55231, process: 'java', ports: [8080, 8081] };

    render(ServiceCard, { props: { service } });

    expect(screen.getByText('java')).toBeInTheDocument();
    expect(screen.getByText(':8080 :8081')).toBeInTheDocument();
    expect(screen.getByText('#55231')).toBeInTheDocument();
  });

  it('포트가 여러 개면 오름차순 공백 구분 라벨로 이어 붙인다', () => {
    const service: Service = { pid: 1, process: 'postgres', ports: [5432, 5433, 5434] };

    render(ServiceCard, { props: { service } });

    expect(screen.getByText(':5432 :5433 :5434')).toBeInTheDocument();
  });
});
