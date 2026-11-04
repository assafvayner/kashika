import data from "./icons/bundled.json" with { type: "json" };

export { ICON_KEYWORDS } from "./icons/keywords.js";

/// A bundled icon as `decode_bundled_icon` decodes it, its `svg` re-serialized in the
/// sentinel color `#010203`, plus the icon file's `source`.
export interface BundledIcon {
  readonly natural: readonly [number, number];
  readonly unit_hull: readonly (readonly [number, number])[];
  readonly current_color: boolean;
  readonly dropped: readonly (readonly [string, number])[];
  readonly svg: string;
  readonly source: string;
}

const ICONS = data as unknown as Readonly<Record<string, BundledIcon>>;

/// The neutral icon set shipped with Kashika, addressed by bare name (`icon="server"`).
export const BUNDLED_ICONS: readonly string[] = Object.keys(ICONS);

export const ICON_SENTINEL_COLOR = "#010203";

export function bundledIcon(name: string): BundledIcon | null {
  return Object.hasOwn(ICONS, name) ? ICONS[name]! : null;
}

export function bundled(name: string): [string, string] | null {
  const n = BUNDLED_ICONS.find((x) => x === name);
  return n === undefined ? null : [n, ICONS[n]!.source];
}
