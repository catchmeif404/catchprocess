// 검색 필터 검증.
import { describe, expect, it } from 'vitest';
import { filterServices } from './search';
import type { Service } from './types';

const services: Service[] = [
  { pid: 20292, process: 'java', ports: [8080], project: { name: 'shop_new', path: '/x', branch: 'feature/snsb-pm-coin' } },
  { pid: 85610, process: 'node', ports: [3001] },
  { pid: 48823, process: 'postgres', ports: [5432] },
];

describe('filterServices', () => {
  it('빈 쿼리는 전체를 반환한다', () => {
    expect(filterServices(services, '')).toHaveLength(3);
    expect(filterServices(services, '   ')).toHaveLength(3);
  });

  it('프로젝트명을 대소문자 무시하고 부분 일치한다', () => {
    expect(filterServices(services, 'SHOP')).toEqual([services[0]]);
  });

  it('프로세스명으로 찾는다', () => {
    expect(filterServices(services, 'post')).toEqual([services[2]]);
  });

  it('포트 번호로 찾는다', () => {
    expect(filterServices(services, '3001')).toEqual([services[1]]);
  });

  it('브랜치명으로 찾는다', () => {
    expect(filterServices(services, 'pm-coin')).toEqual([services[0]]);
  });

  it('일치가 없으면 빈 목록을 반환한다', () => {
    expect(filterServices(services, 'nomatch')).toEqual([]);
  });
});
