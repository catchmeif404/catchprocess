// 리스트를 마우스로 잡아당겨 스크롤하는 Svelte 액션.
// 스크롤바가 숨겨져 있으므로 휠 외에 드래그 탐색을 제공한다.
// 4px 미만 이동은 클릭으로 간주해 버튼(중지) 동작을 방해하지 않고,
// 실제 드래그가 시작되면 그 프레임 이후의 클릭은 무시한다.

interface DragState {
  pointerId: number;
  startY: number;
  startScrollTop: number;
}

export function dragScroll(node: HTMLElement): { destroy: () => void } {
  let drag: DragState | null = null;
  let moved = false;
  let suppressClick = false;

  function isDragging(): boolean {
    return drag !== null && moved;
  }

  function onPointerDown(event: PointerEvent): void {
    if (event.button !== 0) return;
    drag = {
      pointerId: event.pointerId,
      startY: event.clientY,
      startScrollTop: node.scrollTop,
    };
    moved = false;
  }

  function onPointerMove(event: PointerEvent): void {
    if (drag === null || event.pointerId !== drag.pointerId) return;
    const delta = event.clientY - drag.startY;
    if (!moved && Math.abs(delta) < 4) return;
    if (!moved) {
      moved = true;
      node.setPointerCapture(event.pointerId);
    }
    node.scrollTop = drag.startScrollTop - delta;
  }

  function onPointerUp(event: PointerEvent): void {
    if (drag === null || event.pointerId !== drag.pointerId) return;
    if (moved) {
      // 드래그 직후 클릭(버튼 오작동) 방지 — 다음 틱까지 무시.
      suppressClick = true;
      setTimeout(() => {
        suppressClick = false;
      }, 0);
    }
    drag = null;
    moved = false;
  }

  function onClickCapture(event: MouseEvent): void {
    if (suppressClick) {
      event.stopPropagation();
      event.preventDefault();
    }
  }

  node.addEventListener('pointerdown', onPointerDown);
  node.addEventListener('pointermove', onPointerMove);
  node.addEventListener('pointerup', onPointerUp);
  node.addEventListener('pointercancel', onPointerUp);
  node.addEventListener('click', onClickCapture, true);

  return {
    destroy(): void {
      node.removeEventListener('pointerdown', onPointerDown);
      node.removeEventListener('pointermove', onPointerMove);
      node.removeEventListener('pointerup', onPointerUp);
      node.removeEventListener('pointercancel', onPointerUp);
      node.removeEventListener('click', onClickCapture, true);
    },
  };
}
