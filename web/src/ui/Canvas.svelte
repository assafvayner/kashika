<script lang="ts">
  import { tick } from "svelte";

  import { hitTest } from "../document/hit.js";
  import type { Document } from "../document/store.svelte.js";

  let { doc }: { doc: Document } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let zoom = $state(1);
  let pan = $state({ x: 0, y: 0 });
  let canvas = $state<HTMLDivElement | null>(null);
  let drag: { x: number; y: number; px: number; py: number } | null = null;
  let dragged = false;

  const selected = $derived(doc.result.elements.find((e) => e.id === doc.selection) ?? null);

  function svgPoint(ev: MouseEvent): { x: number; y: number } | null {
    const svg = host?.querySelector("svg");
    if (!svg) return null;
    const ctm = svg.getScreenCTM();
    if (!ctm) return null;
    const p = new DOMPoint(ev.clientX, ev.clientY).matrixTransform(ctm.inverse());
    return { x: p.x, y: p.y };
  }

  function onClick(ev: MouseEvent) {
    if (dragged) return;
    const p = svgPoint(ev);
    doc.selection = p === null ? null : hitTest(doc.result.elements, p.x, p.y);
  }

  function onWheel(ev: WheelEvent) {
    ev.preventDefault();
    const factor = Math.exp(-ev.deltaY * 0.0015);
    zoom = Math.min(8, Math.max(0.1, zoom * factor));
  }

  function onDown(ev: MouseEvent) {
    drag = { x: ev.clientX, y: ev.clientY, px: pan.x, py: pan.y };
    dragged = false;
  }
  function onMove(ev: MouseEvent) {
    if (!drag) return;
    const dx = ev.clientX - drag.x;
    const dy = ev.clientY - drag.y;
    if (Math.abs(dx) + Math.abs(dy) > 3) dragged = true;
    if (dragged) pan = { x: drag.px + dx, y: drag.py + dy };
  }
  function onUp() {
    drag = null;
  }

  let fitted: { w: number; h: number } | null = null;

  $effect(() => {
    const svg = doc.result.svg;
    if (!svg || !host) {
      if (!svg) fitted = null;
      return;
    }
    void tick().then(() => {
      if (drag || !canvas?.clientWidth || !canvas.clientHeight) return;
      const el = host?.querySelector("svg");
      const w = Number.parseFloat(el?.getAttribute("width") ?? "");
      const h = Number.parseFloat(el?.getAttribute("height") ?? "");
      if (!(w > 0 && h > 0)) return;
      const grew =
        fitted === null ||
        Math.abs(w - fitted.w) / fitted.w > 0.25 ||
        Math.abs(h - fitted.h) / fitted.h > 0.25;
      if (grew) {
        fitted = { w, h };
        fit();
      }
    });
  });

  export function fit() {
    const svg = host?.querySelector("svg");
    const w = Number.parseFloat(svg?.getAttribute("width") ?? "");
    const h = Number.parseFloat(svg?.getAttribute("height") ?? "");
    const cw = canvas?.clientWidth ?? 0;
    const ch = canvas?.clientHeight ?? 0;
    if (!(w > 0 && h > 0 && cw > 0 && ch > 0)) {
      zoom = 1;
      pan = { x: 0, y: 0 };
      return;
    }
    const scale = Math.min(cw / w, ch / h, 1);
    zoom = scale;
    pan = { x: (cw - w * scale) / 2 - 24 * scale, y: (ch - h * scale) / 2 - 24 * scale };
  }
</script>

<div
  class="canvas"
  bind:this={canvas}
  role="presentation"
  onwheel={onWheel}
  onmousedown={onDown}
  onmousemove={onMove}
  onmouseup={onUp}
  onmouseleave={onUp}
>
  <div class="stage" style="transform: translate({pan.x}px, {pan.y}px) scale({zoom})">
    <div class="svg" bind:this={host} role="presentation" onclick={onClick}>
      {@html doc.result.svg ?? ""}
      {#if selected}
        <div
          class="selection"
          style="left: {selected.bbox[0][0]}px; top: {selected.bbox[0][1]}px; width: {selected
            .bbox[1][0] - selected.bbox[0][0]}px; height: {selected.bbox[1][1] -
            selected.bbox[0][1]}px"
        ></div>
      {/if}
    </div>
  </div>
  <button class="fit" onclick={fit} title="Fit">⤢</button>
</div>

<style>
  .canvas {
    position: relative;
    overflow: hidden;
    height: 100%;
    background-image: radial-gradient(var(--border) 1px, transparent 1px);
    background-size: 20px 20px;
    cursor: grab;
    user-select: none;
  }
  .stage {
    transform-origin: 0 0;
    padding: 24px;
  }
  .svg {
    position: relative;
    display: inline-block;
    line-height: 0;
  }
  .svg :global(svg) {
    display: block;
    background: var(--bg);
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.15);
  }
  .selection {
    position: absolute;
    outline: 2px solid #e67e22;
    outline-offset: 2px;
    pointer-events: none;
  }
  .fit {
    position: absolute;
    right: 12px;
    bottom: 12px;
    border: 1px solid var(--border);
    background: var(--panel);
    border-radius: 6px;
    padding: 4px 8px;
    cursor: pointer;
  }
</style>
