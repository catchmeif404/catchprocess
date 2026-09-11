import { expect, it } from 'vitest';
import { apiLabel, serviceLabel } from './service-presentation';

it('does not infer frontend from Node alone', () => {
  expect(serviceLabel({ pid: 1, process: 'node', ports: [3000] })).toBe('node');
});
it('matches loopback only, excludes self and ambiguous listeners', () => {
  const source = { pid: 1, process: 'node', ports: [3000] };
  const backend = { pid: 2, process: 'java', ports: [8080], framework: 'spring' };
  expect(apiLabel({ host: 'api.example.com', port: 8080 }, source, [backend])).toBe('api.example.com');
  expect(apiLabel({ host: 'localhost', port: 8080 }, source, [backend])).toBe('Spring Boot');
  expect(apiLabel({ host: 'localhost', port: 8080 }, source, [backend, { ...backend, pid: 3 }])).toBe('localhost');
  expect(apiLabel({ host: 'localhost', port: 3000 }, source, [source])).toBe('localhost');
});
