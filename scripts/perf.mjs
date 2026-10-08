/**
 * UI smoothness check. Drives the browser preview (`npm run dev`) in headless Chrome,
 * records frame times during the interactions users reported as janky and fails on
 * long frames or on layout that snaps instead of animating.
 *
 * Usage: `npm run dev` in one terminal, then `npm run test:perf`.
 * Set CHROME_PATH when Chrome is not installed in the default Windows location.
 */
import { fileURLToPath } from "node:url";
import puppeteer from "puppeteer-core";

const CHROME = process.env.CHROME_PATH ?? "C:/Program Files/Google/Chrome/Application/chrome.exe";
const URL = process.env.PREVIEW_URL ?? "http://localhost:1420/";
/** Three missed frames at 60 Hz; anything longer is a visible hitch. */
const MAX_FRAME_MS = 50;
/**
 * A theme change starts with one frame in which the browser snapshots the page for the
 * cross-fade; without a GPU (headless) that frame alone takes 35-55 ms.
 */
const THEME_SWITCH_MAX_FRAME_MS = 80;
/**
 * Opening the queue must reflow the main area once (a width animation re-lays out every card
 * grid per frame) while the panel itself glides through many positions.
 */
const MAX_QUEUE_REFLOWS = 2;
const MIN_QUEUE_PANEL_STEPS = 10;

const browser = await puppeteer.launch({
  executablePath: CHROME,
  headless: "new",
  args: ["--window-size=1440,900", "--force-device-scale-factor=1"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1440, height: 900 });
await page.goto(URL, { waitUntil: "networkidle2" });
await page.evaluate(() => localStorage.clear());
await page.reload({ waitUntil: "networkidle2" });

await page.evaluate(() => {
  window.__sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  window.__byLabel = (label) => [...document.querySelectorAll("button")].find((b) => b.getAttribute("aria-label") === label);
  window.__measure = async (action, ms = 900) => {
    const frames = [];
    const widths = [];
    const panelX = [];
    const main = document.querySelector(".main");
    const panel = document.querySelector(".queue-slot");
    let last = performance.now();
    let run = true;
    const loop = (t) => {
      frames.push(t - last);
      last = t;
      widths.push(Math.round(main.getBoundingClientRect().width));
      panelX.push(Math.round(panel.getBoundingClientRect().left));
      if (run) requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
    await window.__sleep(120);
    await action();
    await window.__sleep(ms);
    run = false;
    const s = frames.slice(3).sort((a, b) => a - b);
    const distinct = [...new Set(widths)];
    return {
      frames: s.length,
      medianMs: +s[Math.floor(s.length / 2)].toFixed(1),
      p95Ms: +s[Math.floor(s.length * 0.95)].toFixed(1),
      maxMs: +s[s.length - 1].toFixed(1),
      over34ms: s.filter((f) => f > 34).length,
      mainWidthSteps: distinct.length,
      panelSteps: new Set(panelX).size,
    };
  };
});

// Seed: import the 300-track demo playlist and play from it.
await page.evaluate(async () => {
  window.__byLabel("Создать плейлист").click();
  await window.__sleep(150);
  [...document.querySelectorAll("[role=menuitem]")].find((b) => b.textContent.includes("Импорт")).click();
  await window.__sleep(300);
  const input = document.querySelector("#import-link");
  input.value = "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M";
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await window.__sleep(50);
  document.querySelector(".dialog form").requestSubmit();
  await window.__sleep(1500);
  document.querySelectorAll("[role=listitem]")[0].dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
  await window.__sleep(800);
});
await new Promise((r) => setTimeout(r, 3000)); // let artwork load

const results = {};
results.queueOpen = await page.evaluate(() => window.__measure(() => window.__byLabel("Очередь").click()));
results.queueClose = await page.evaluate(() => window.__measure(() => window.__byLabel("Очередь").click()));

results.scroll300 = await page.evaluate(() =>
  window.__measure(async () => {
    const view = document.querySelector(".view");
    for (let i = 0; i < 60; i++) {
      view.scrollTop += 120;
      await new Promise((r) => requestAnimationFrame(r));
    }
  }, 300),
);

results.toHome = await page.evaluate(() =>
  window.__measure(() => [...document.querySelectorAll(".nav-item")].find((b) => b.textContent.includes("Главная")).click()),
);
results.cachedArtwork = await page.evaluate(async () => {
  await window.__sleep(200);
  const imgs = [...document.querySelectorAll(".card img")];
  return { cards: imgs.length, instant: imgs.filter((i) => i.classList.contains("instant")).length };
});
results.toPlaylist = await page.evaluate(() => window.__measure(() => document.querySelectorAll(".entry")[1].click()));

await page.evaluate(() => [...document.querySelectorAll(".nav-item")].find((b) => b.textContent.includes("Настройки")).click());
await new Promise((r) => setTimeout(r, 500));
results.themeToLight = await page.evaluate(() =>
  window.__measure(() => [...document.querySelectorAll("[role=radio]")].find((b) => b.textContent.trim() === "Светлая").click()),
);
results.themeToDark = await page.evaluate(() =>
  window.__measure(() => [...document.querySelectorAll("[role=radio]")].find((b) => b.textContent.trim() === "Тёмная").click()),
);
results.switchingFlagCleared = await page.evaluate(() => !("switching" in document.documentElement.dataset));

// Selection indicators must travel between options, not jump, and settle on the choice.
results.glide = await page.evaluate(async () => {
  const checks = {};
  for (const [name, index, selector] of [["language", 0, ".thumb"], ["theme", 1, ".thumb"], ["accent", 2, ".ring"]]) {
    const group = document.querySelectorAll("[role=radiogroup]")[index];
    const indicator = group.querySelector(selector);
    const x = () => new DOMMatrix(getComputedStyle(indicator).transform).m41;
    const options = [...group.querySelectorAll("[role=radio]")];
    const next = options.find((o) => o.getAttribute("aria-checked") !== "true" && o !== options[0]) ?? options[0];
    const from = x();
    next.click();
    await window.__sleep(120);
    const mid = x();
    await window.__sleep(700);
    const to = x();
    checks[name] = {
      slides: Math.min(from, to) < mid && mid < Math.max(from, to),
      settles: Math.abs(to - next.offsetLeft) < 1,
    };
  }
  return checks;
});

// Space must toggle playback even when a button kept focus after a click (it used to
// press that button instead, e.g. minimizing the window).
results.spaceOnFocusedButton = await (async () => {
  const minimize = await page.$(".window-controls button:first-child");
  await page.evaluate(() => {
    window.__minimizeClicks = 0;
    document.querySelector(".window-controls button:first-child").addEventListener("click", () => window.__minimizeClicks++);
  });
  await minimize.click();
  await page.evaluate(() => (window.__minimizeClicks = 0));
  const label = () => page.$eval(".main-button", (b) => b.getAttribute("aria-label"));
  const before = await label();
  await page.keyboard.press("Space");
  await new Promise((r) => setTimeout(r, 300));
  return { toggled: before !== (await label()), buttonPressed: await page.evaluate(() => window.__minimizeClicks > 0) };
})();

// Editing flows: drag to reorder (queue and sidebar) and a custom playlist cover.
async function drag(from, to) {
  const a = await from.boundingBox();
  const b = await to.boundingBox();
  // Aim past the target's midpoint, the point where rows swap places.
  const distance = b.y - a.y + Math.sign(b.y - a.y) * (b.height / 3);
  await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  await page.mouse.down();
  for (let step = 1; step <= 12; step++) {
    await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2 + (distance * step) / 12);
    await new Promise((r) => setTimeout(r, 16));
  }
  await page.mouse.up();
  await new Promise((r) => setTimeout(r, 300));
}

results.editing = {};
{
  await page.evaluate(() => window.__byLabel("Queue")?.click() ?? window.__byLabel("Очередь")?.click());
  await new Promise((r) => setTimeout(r, 500));
  const titles = () => page.$$eval(".upcoming .title", (els) => els.slice(0, 3).map((e) => e.textContent));
  const before = await titles();
  const items = await page.$$(".upcoming [data-sortable]");
  await drag(items[2], items[0]);
  const after = await titles();
  results.editing.queueDrag = after[0] === before[2] && after[1] === before[0];
}
{
  const create = () =>
    page.evaluate(async () => {
      const add = document.querySelector(".library-header .icon-button");
      add.click();
      await window.__sleep(150);
      document.querySelector("[role=menuitem]").click();
      await window.__sleep(400);
      document.activeElement?.blur();
      await window.__sleep(200);
    });
  await create();
  await create();
  const names = () => page.$$eval(".collections .entry-title", (els) => els.map((e) => e.textContent));
  const before = await names();
  const entries = await page.$$(".collections [data-sortable]");
  await drag(entries[entries.length - 1], entries[0]);
  const after = await names();
  results.editing.sidebarDrag = after[0] === before[before.length - 1];

  const [chooser] = await Promise.all([page.waitForFileChooser(), page.click(".hero .cover")]);
  await chooser.accept([fileURLToPath(import.meta.resolve("../src-tauri/icons/icon.png"))]);
  await new Promise((r) => setTimeout(r, 800));
  results.editing.customCover = await page.$eval(".hero .cover img", (img) => img.src.startsWith("data:image/jpeg"));
}

await browser.close();

const failures = [];
for (const [name, result] of Object.entries(results)) {
  const limit = name.startsWith("theme") ? THEME_SWITCH_MAX_FRAME_MS : MAX_FRAME_MS;
  if (result.maxMs > limit) failures.push(`${name}: ${result.maxMs} ms frame`);
}
for (const name of ["queueOpen", "queueClose"]) {
  if (results[name].mainWidthSteps > MAX_QUEUE_REFLOWS) failures.push(`${name}: content reflows every frame`);
  if (results[name].panelSteps < MIN_QUEUE_PANEL_STEPS) failures.push(`${name}: panel snapped instead of sliding`);
}
if (results.cachedArtwork.instant !== results.cachedArtwork.cards) failures.push("cached artwork faded in");
if (!results.switchingFlagCleared) failures.push("theme switch left transitions disabled");
if (!results.spaceOnFocusedButton.toggled || results.spaceOnFocusedButton.buttonPressed) {
  failures.push("space pressed the focused button instead of toggling playback");
}
for (const [name, passed] of Object.entries(results.editing)) {
  if (!passed) failures.push(`${name} did not work`);
}
for (const [name, check] of Object.entries(results.glide)) {
  if (!check.slides) failures.push(`${name} indicator jumps instead of sliding`);
  if (!check.settles) failures.push(`${name} indicator misses the selected option`);
}

console.table(
  Object.fromEntries(Object.entries(results).filter(([, r]) => typeof r === "object" && "maxMs" in r)),
);
if (failures.length > 0) {
  console.error(["Jank detected:", ...failures].join("\n  "));
  process.exit(1);
}
console.log("UI interactions are smooth.");
