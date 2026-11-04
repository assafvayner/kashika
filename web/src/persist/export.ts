export function download(name: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  URL.revokeObjectURL(url);
}

export function exportSvg(title: string, svg: string): void {
  download(`${title}.svg`, new Blob([svg], { type: "image/svg+xml" }));
}

export function exportKash(title: string, source: string): void {
  download(`${title}.kash`, new Blob([source], { type: "text/plain" }));
}

/** Rasterizes the SVG at `scale` with a canvas; the SVG's own width/height set the size. */
export async function exportPng(title: string, svg: string, scale = 2): Promise<void> {
  const img = new Image();
  const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  try {
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error("cannot decode the SVG"));
      img.src = url;
    });
    const canvas = document.createElement("canvas");
    canvas.width = Math.ceil(img.naturalWidth * scale);
    canvas.height = Math.ceil(img.naturalHeight * scale);
    const ctx = canvas.getContext("2d")!;
    ctx.scale(scale, scale);
    ctx.drawImage(img, 0, 0);
    const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, "image/png"));
    if (blob !== null) download(`${title}.png`, blob);
  } finally {
    URL.revokeObjectURL(url);
  }
}
