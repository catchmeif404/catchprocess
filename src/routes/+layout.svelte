<script lang="ts">
  // 전역 스타일 로딩 레이아웃. 위젯은 단일 페이지이므로 이것만 한다.
  import '../app.css';

  let { children } = $props();
</script>

{@render children()}
