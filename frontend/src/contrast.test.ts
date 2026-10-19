// @vitest-environment node
// Checks every colour pair the UI uses against WCAG AA, in both themes, straight from the
// tokens in app.css. Node environment so the stylesheet can be read from disk.

import { readFileSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

const css = readFileSync(new URL('./app.css', import.meta.url), 'utf8');

type Rgba = [number, number, number, number];

const tokens = new Map<string, [string, string]>();
for (const match of css.matchAll(
  /--([\w-]+):\s*light-dark\(((?:[^()]|\([^()]*\))+),\s*((?:[^()]|\([^()]*\))+)\);/g,
)) {
  tokens.set(match[1]!, [match[2]!.trim(), match[3]!.trim()]);
}

function parse(colour: string): Rgba {
  const hex = colour.match(/^#([0-9a-f]{6})$/i);
  if (hex) {
    const n = parseInt(hex[1]!, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 1];
  }
  const rgb = colour.match(/^rgb\((\d+) (\d+) (\d+) \/ (\d+)%\)$/);
  if (rgb) return [+rgb[1]!, +rgb[2]!, +rgb[3]!, +rgb[4]! / 100];
  throw new Error(`unsupported colour ${colour}`);
}

function over(fg: Rgba, bg: Rgba): Rgba {
  return [0, 1, 2].map((i) => fg[i]! * fg[3] + bg[i]! * (1 - fg[3])).concat(1) as Rgba;
}

function luminance([r, g, b]: Rgba): number {
  const channel = (v: number) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

function ratio(a: Rgba, b: Rgba): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

// WCAG AA minimums: normal text, large text (pick numbers), and UI parts such as borders.
const TEXT = 4.5;
const LARGE = 3;
const UI = 3;

// [foreground token, background, minimum]. `tag` and `policy-tag` are the tinted tag backgrounds.
const pairs: [string, string, number][] = [
  ['text', 'bg', TEXT],
  ['text', 'card', TEXT],
  ['text-2', 'bg', TEXT],
  ['text-2', 'card', TEXT],
  ['text-2', 'surface', TEXT],
  ['muted', 'bg', TEXT],
  ['muted', 'card', TEXT],
  ['muted', 'surface', TEXT],
  ['accent', 'card', TEXT],
  ['accent', 'bg', TEXT],
  ['on-accent', 'accent', TEXT],
  ['up', 'card', TEXT],
  ['down', 'card', TEXT],
  ['closed', 'card', TEXT],
  ['approval', 'bg', TEXT],
  ['open', 'tag', TEXT],
  ['approval', 'tag', TEXT],
  ['closed', 'tag', TEXT],
  ['policy', 'policy-tag', TEXT],
  ['gold', 'card', LARGE],
  ['silver', 'card', LARGE],
  ['bronze', 'card', LARGE],
  ['control-border', 'card', UI],
  ['control-border', 'bg', UI],
  ['accent', 'surface', UI],
];

describe.each([
  ['light', 0],
  ['dark', 1],
] as const)('%s theme colour contrast', (_theme, index) => {
  const colour = (name: string): Rgba => {
    const pair = tokens.get(name);
    if (!pair) throw new Error(`missing token --${name}`);
    return parse(pair[index]);
  };
  const card = colour('card');
  const background = (name: string, fg: Rgba): Rgba => {
    if (name === 'tag') return over([fg[0], fg[1], fg[2], 0.15], card);
    if (name === 'policy-tag') return over([fg[0], fg[1], fg[2], 0.12], card);
    return over(colour(name), card);
  };

  it.each(pairs)('--%s on %s meets %s:1', (fgName, bgName, min) => {
    const fg = colour(fgName);
    const bg = background(bgName, fg);
    expect(ratio(over(fg, bg), bg)).toBeGreaterThanOrEqual(min);
  });
});
