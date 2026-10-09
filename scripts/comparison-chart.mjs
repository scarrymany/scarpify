/**
 * Draws the README resource comparison in the hand-sketched style of star-history.com charts.
 *
 * Usage: `node scripts/comparison-chart.mjs`. Writes docs/resources.svg (English) and
 * docs/resources.ru.svg (Russian). Update MEASUREMENTS after measuring a new release.
 * The Neucha font (SIL Open Font License) is embedded so GitHub renders it inside <img>.
 */
import { writeFileSync } from "node:fs";

const MEASUREMENTS = {
  cpu: { scarpify: 1.3, spotify: 2.7, max: 3, step: 1 },
  memory: { scarpify: 240, scarpifyLite: 200, spotify: 835, max: 1000, step: 250 },
};

const TEXT = {
  en: {
    file: "docs/resources.svg",
    title: "Lighter than Spotify",
    cpu: "CPU, %",
    memory: "Memory, MB",
    lite: "no animations",
    note: "Same Windows 11 PC, music playing, every process of each app counted",
    number: (value) => String(value),
  },
  ru: {
    file: "docs/resources.ru.svg",
    title: "Легче, чем Spotify",
    cpu: "Процессор, %",
    memory: "Память, МБ",
    lite: "без анимаций",
    note: "Один ПК с Windows 11, играет музыка, учтены все процессы приложений",
    number: (value) => String(value).replace(".", ","),
  },
};

const FONT_FILES = [
  "https://fonts.gstatic.com/s/neucha/v18/q5uGsou0JOdh94bfvQlt.woff2",
  "https://fonts.gstatic.com/s/neucha/v18/q5uGsou0JOdh94bfuQltOxU.woff2",
];

const WIDTH = 900;
const HEIGHT = 590;
const INK = "#1b1b1f";
const ACCENT = "#ec6a45";
const MUTED = "#8a8a93";
const ACCENT_LIGHT = "#f2a48b";

/** Deterministic pseudo-random numbers, so regenerating gives an identical image. */
function random(seed) {
  let state = seed;
  return () => {
    state = (state * 1664525 + 1013904223) % 4294967296;
    return state / 4294967296;
  };
}

const rand = random(17);
const jitter = (amount) => (rand() - 0.5) * 2 * amount;

/** A line drawn twice with slight wobble, like a pen stroke. */
function sketchLine(x1, y1, x2, y2, { color = INK, width = 2.6, wobble = 1.6 } = {}) {
  const strokes = [0, 1].map(() => {
    const sx = x1 + jitter(wobble);
    const sy = y1 + jitter(wobble);
    const ex = x2 + jitter(wobble);
    const ey = y2 + jitter(wobble);
    const length = Math.hypot(x2 - x1, y2 - y1);
    const bow = Math.min(4, length / 60);
    const cx = (sx + ex) / 2 + jitter(bow);
    const cy = (sy + ey) / 2 + jitter(bow);
    return `M${sx.toFixed(1)} ${sy.toFixed(1)} Q${cx.toFixed(1)} ${cy.toFixed(1)} ${ex.toFixed(1)} ${ey.toFixed(1)}`;
  });
  return `<path d="${strokes.join(" ")}" fill="none" stroke="${color}" stroke-width="${width}" stroke-linecap="round"/>`;
}

function sketchRect(x, y, w, h, options) {
  return [
    sketchLine(x, y, x + w, y, options),
    sketchLine(x + w, y, x + w, y + h, options),
    sketchLine(x + w, y + h, x, y + h, options),
    sketchLine(x, y + h, x, y, options),
  ].join("");
}

/** Diagonal hatching clipped to a rectangle, as rough.js fills shapes. */
function hachure(x, y, w, h, color) {
  const gap = 7;
  const lines = [];
  for (let offset = gap; offset < w + h; offset += gap) {
    const start = { x: x + Math.min(offset, w), y: y + h - Math.max(0, offset - w) };
    const end = { x: x + Math.max(0, offset - h), y: y + h - Math.min(offset, h) };
    lines.push(sketchLine(start.x, start.y, end.x, end.y, { color, width: 2, wobble: 0.8 }));
  }
  return lines.join("");
}

function bar(x, baseline, width, height, color) {
  const top = baseline - height;
  return hachure(x, top, width, height, color) + sketchRect(x, top, width, height, { color, width: 2.4, wobble: 1 });
}

function text(x, y, content, { size = 22, color = INK, anchor = "middle", rotate = 0, heavy = false } = {}) {
  const transform = rotate ? ` transform="rotate(${rotate} ${x} ${y})"` : "";
  const outline = heavy ? ` stroke="${color}" stroke-width="1.2" stroke-linejoin="round"` : "";
  return `<text x="${x}" y="${y}" font-size="${size}" fill="${color}" text-anchor="${anchor}"${outline}${transform}>${content}</text>`;
}

/** Neucha has no glyphs for "." and ",", so the decimal separator is taken from a system font. */
function decimal(formatted) {
  return formatted.replace(/[.,]/, (separator) => `<tspan font-family="Arial,sans-serif" font-weight="700">${separator}</tspan>`);
}

/** One small bar chart with hand-drawn axes, ticks and value labels. */
function chart(left, label, data, t) {
  const top = 190;
  const baseline = 470;
  const plotHeight = baseline - top;
  const axisX = left + 70;
  const right = axisX + 290;
  const parts = [text(left + 200, top - 22, label, { size: 26 })];

  parts.push(sketchLine(axisX, top - 4, axisX, baseline + 2, { width: 3 }));
  parts.push(sketchLine(axisX - 2, baseline, right, baseline, { width: 3 }));
  for (let tick = 0; tick <= data.max; tick += data.step) {
    const y = baseline - (tick / data.max) * plotHeight;
    parts.push(sketchLine(axisX - 8, y, axisX, y, { width: 2.4 }));
    parts.push(text(axisX - 14, y + 7, t.number(tick), { size: 20, anchor: "end" }));
  }

  const bars = [
    { name: "SCARPIFY", value: data.scarpify, color: ACCENT },
    { name: "SCARPIFY", caption: t.lite, value: data.scarpifyLite, color: ACCENT_LIGHT },
    { name: "Spotify", value: data.spotify, color: MUTED },
  ].filter((item) => item.value !== undefined);
  const slot = (right - axisX) / bars.length;
  const barWidth = bars.length > 2 ? 64 : 76;
  bars.forEach((item, index) => {
    const x = axisX + slot * index + (slot - barWidth) / 2;
    const center = x + barWidth / 2;
    const height = (item.value / data.max) * plotHeight;
    parts.push(bar(x, baseline, barWidth, height, item.color));
    parts.push(text(center, baseline - height - 14, decimal(t.number(item.value)), { size: 30, color: item.color, heavy: true }));
    parts.push(text(center, baseline + 32, item.name, { size: 21 }));
    if (item.caption) parts.push(text(center, baseline + 54, item.caption, { size: 17, color: MUTED }));
  });
  return parts.join("");
}

/** The SCARPIFY mark (three arcs), sketched. */
function mark(cx, cy) {
  const arcs = [
    { dx: 15, dy: -6, sag: 8 },
    { dx: 11, dy: 3, sag: 6 },
    { dx: 7, dy: 11, sag: 4 },
  ];
  const strokes = arcs.map(({ dx, dy, sag }) => {
    const sx = cx - dx + jitter(0.6);
    const ex = cx + dx + jitter(0.6);
    return `<path d="M${sx.toFixed(1)} ${cy + dy + sag / 2} Q${cx} ${cy + dy - sag} ${ex.toFixed(1)} ${cy + dy + sag / 2}" fill="none" stroke="${ACCENT}" stroke-width="4" stroke-linecap="round"/>`;
  });
  const ring = Array.from({ length: 2 }, () => {
    const r = 24 + jitter(0.8);
    return `<circle cx="${cx + jitter(0.8)}" cy="${cy + jitter(0.8)}" r="${r.toFixed(1)}" fill="none" stroke="${INK}" stroke-width="2.6"/>`;
  });
  return ring.join("") + strokes.join("");
}

const LEGEND_WIDTH = 430;

function legend(x, y, t) {
  const entries = [
    { label: "SCARPIFY", color: ACCENT, offset: 18 },
    { label: t.lite, color: ACCENT_LIGHT, offset: 154 },
    { label: "Spotify", color: MUTED, offset: 318 },
  ];
  return [
    sketchRect(x, y, LEGEND_WIDTH, 48, { width: 2.6 }),
    ...entries.map(({ label, color, offset }) =>
      `<rect x="${x + offset}" y="${y + 17}" width="14" height="14" rx="3" fill="${color}"/>` +
      text(x + offset + 24, y + 31, label, { size: 21, anchor: "start" }),
    ),
  ].join("");
}

async function fontFaces() {
  const faces = await Promise.all(
    FONT_FILES.map(async (url) => {
      const data = Buffer.from(await (await fetch(url)).arrayBuffer()).toString("base64");
      return `@font-face{font-family:Sketch;src:url(data:font/woff2;base64,${data}) format("woff2");}`;
    }),
  );
  return faces.join("");
}

const fonts = await fontFaces();

for (const t of Object.values(TEXT)) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}" role="img" aria-label="${t.title}: CPU ${t.number(MEASUREMENTS.cpu.scarpify)}% vs ${t.number(MEASUREMENTS.cpu.spotify)}%, ${t.number(MEASUREMENTS.memory.scarpify)} (${t.lite}: ${t.number(MEASUREMENTS.memory.scarpifyLite)}) vs ${t.number(MEASUREMENTS.memory.spotify)} MB">
<style>${fonts}text{font-family:Sketch,"Comic Sans MS",cursive}</style>
<rect width="${WIDTH}" height="${HEIGHT}" fill="#ffffff"/>
${mark(WIDTH / 2 - 150, 62)}
${text(WIDTH / 2 + 30, 74, t.title, { size: 36 })}
${legend((WIDTH - LEGEND_WIDTH) / 2, 98, t)}
${chart(20, t.cpu, MEASUREMENTS.cpu, t)}
${chart(460, t.memory, MEASUREMENTS.memory, t)}
${text(WIDTH - 24, HEIGHT - 18, t.note, { size: 17, color: MUTED, anchor: "end" })}
</svg>
`;
  writeFileSync(new URL(`../${t.file}`, import.meta.url), svg);
  console.log("wrote", t.file);
}
