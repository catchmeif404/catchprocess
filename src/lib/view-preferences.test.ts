import { expect, it } from 'vitest';
import { parseEnvironment } from './view-preferences';

it('parses environment text while ignoring comments and blank lines', () => {
  expect(parseEnvironment('# local\nPORT=8080\n\nPROFILE = dev')).toEqual({ PORT: '8080', PROFILE: 'dev' });
});
