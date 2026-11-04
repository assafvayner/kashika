/// <reference types="vite/client" />
/// <reference types="svelte" />

declare module "*.md?raw" {
  const text: string;
  export default text;
}
