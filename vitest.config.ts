// Vitest 설정.
// - sveltekit() 플러그인은 SSR(server) 조건으로 Svelte를 컴파일해 컴포넌트 mount 테스트가
//   실패하므로, 테스트에서는 svelte() 플러그인 + browser resolve 조건을 사용한다.
// - $lib 별칭은 SvelteKit이 제공하는 것이므로 테스트용으로 직접 선언한다.
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
    conditions: ['browser'],
  },
  test: {
    environment: 'jsdom',
    globals: true,
  },
});
