// Everything flat is drawn with the 2D canvas and used as a texture: text, chips, cards,
// and the terminal mode, which is recreated cell by cell from the TUI layout
// (src-tauri/src/tui/ui) with real covers cut from the screenshots.

export const FONT = '"Onest Variable", Inter, "Segoe UI", sans-serif';
export const MONO = '"JetBrains Mono", "DejaVu Sans Mono", "DejaVu Sans", "Noto Color Emoji", monospace';

export const C = {
  bg: '#0c0c0e',
  surface: '#151518',
  raised: '#1c1c20',
  hover: '#242429',
  active: '#2d2d33',
  text: '#ededef',
  muted: '#a3a3ab',
  faint: '#6e6e78',
  accent: '#ec6a45',
  onAccent: '#16110f',
  youtube: '#ff0033',
  soundcloud: '#ff5500',
  spotify: '#1ed760',
};

export function loadImage(url) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = url;
  });
}

export function canvas(w, h, draw) {
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const g = c.getContext('2d');
  if (draw) draw(g, w, h);
  return c;
}

export function rrect(g, x, y, w, h, r) {
  g.beginPath();
  g.roundRect(x, y, w, h, r);
}

function font(g, size, weight = 400, family = FONT) {
  g.font = `${weight} ${size}px ${family}`;
}

/** The three arcs of assets/icon.svg, drawn at `size` pixels. */
export function drawIcon(g, x, y, size, disc = true) {
  const s = size / 1024;
  g.save();
  g.translate(x, y);
  g.scale(s, s);
  if (disc) {
    g.fillStyle = C.surface;
    g.beginPath();
    g.arc(512, 512, 464, 0, Math.PI * 2);
    g.fill();
  }
  g.strokeStyle = C.accent;
  g.lineCap = 'round';
  const arcs = [
    [76, 262, 382, 420, 322, 612, 330, 772, 410],
    [64, 298, 532, 432, 486, 592, 494, 726, 560],
    [54, 334, 670, 444, 636, 572, 642, 680, 694],
  ];
  for (const [w, ...p] of arcs) {
    g.lineWidth = w;
    g.beginPath();
    g.moveTo(p[0], p[1]);
    g.bezierCurveTo(p[2], p[3], p[4], p[5], p[6], p[7]);
    g.stroke();
  }
  g.restore();
}

// ---------------------------------------------------------------- titles and captions

export function wordmark() {
  return canvas(1600, 260, (g, w, h) => {
    font(g, 150, 700);
    g.letterSpacing = '30px';
    g.fillStyle = C.text;
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    // letterSpacing adds space after the last letter too; shift by half of it.
    g.fillText('SCARPIFY', w / 2 + 15, h / 2 + 6);
  });
}

export function line(text, { size = 44, weight = 400, color = C.muted, width = 1800, height = 110, spacing = 0 } = {}) {
  return canvas(width, height, (g, w, h) => {
    font(g, size, weight);
    g.letterSpacing = `${spacing}px`;
    g.fillStyle = color;
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    g.fillText(text, w / 2 + spacing / 2, h / 2 + 2);
  });
}

/** A feature caption: a small accent kicker over one line, left aligned, at 2x. */
export function caption(kicker, text) {
  return canvas(3200, 300, (g) => {
    font(g, 34, 600);
    g.letterSpacing = '7px';
    g.fillStyle = C.accent;
    g.textBaseline = 'alphabetic';
    g.fillText(kicker.toUpperCase(), 8, 80);
    g.letterSpacing = '0px';
    font(g, 96, 600);
    g.fillStyle = C.text;
    g.fillText(text, 4, 220);
  });
}

export function githubPill() {
  return canvas(1500, 200, (g, w, h) => {
    const text = 'github.com/scarrymany/scarpify';
    font(g, 66, 500);
    const tw = g.measureText(text).width;
    const pw = tw + 130 + 90;
    const x = (w - pw) / 2;
    rrect(g, x + 3, 23, pw - 6, h - 46, (h - 46) / 2);
    g.fillStyle = 'rgba(236,106,69,0.10)';
    g.fill();
    g.lineWidth = 4;
    g.strokeStyle = C.accent;
    g.stroke();
    // GitHub's mark is a brand; a neutral "code" glyph stands in for it.
    g.save();
    g.translate(x + 80, h / 2);
    g.strokeStyle = C.accent;
    g.lineWidth = 7;
    g.lineCap = 'round';
    g.lineJoin = 'round';
    g.beginPath();
    g.moveTo(-10, -20); g.lineTo(-30, 0); g.lineTo(-10, 20);
    g.moveTo(10, -20); g.lineTo(30, 0); g.lineTo(10, 20);
    g.stroke();
    g.restore();
    g.fillStyle = C.text;
    g.textBaseline = 'middle';
    g.fillText(text, x + 140, h / 2 + 3);
  });
}

// ---------------------------------------------------------------- floating UI pieces

function dot(g, x, y, r, color) {
  g.fillStyle = color;
  g.beginPath();
  g.arc(x, y, r, 0, Math.PI * 2);
  g.fill();
}

function shadowed(g, draw) {
  g.save();
  g.shadowColor = 'rgba(0,0,0,0.55)';
  g.shadowBlur = 40;
  g.shadowOffsetY = 14;
  draw();
  g.restore();
}

/** A pill like the search filter chips, with a service dot. */
export function chip(label, color, { accent = false } = {}) {
  const pad = 60;
  return canvas(900, 200, (g, w, h) => {
    font(g, 62, 600);
    const tw = g.measureText(label).width;
    const pw = tw + (color ? 150 : 100);
    const x = (w - pw) / 2;
    shadowed(g, () => {
      rrect(g, x, pad / 2 + 10, pw, h - pad - 20, (h - pad - 20) / 2);
      g.fillStyle = accent ? C.accent : C.raised;
      g.fill();
    });
    g.lineWidth = 2;
    g.strokeStyle = accent ? 'rgba(255,255,255,0.12)' : 'rgba(255,255,255,0.09)';
    rrect(g, x, pad / 2 + 10, pw, h - pad - 20, (h - pad - 20) / 2);
    g.stroke();
    let tx = x + 50;
    if (color) {
      dot(g, x + 64, h / 2, 17, color);
      tx = x + 102;
    }
    g.fillStyle = accent ? C.onAccent : C.text;
    g.textBaseline = 'middle';
    g.fillText(label, tx, h / 2 + 3);
  });
}

export function linkCard() {
  return canvas(1500, 300, (g, w, h) => {
    shadowed(g, () => {
      rrect(g, 30, 40, w - 60, h - 80, 36);
      g.fillStyle = C.raised;
      g.fill();
    });
    g.strokeStyle = 'rgba(255,255,255,0.08)';
    g.lineWidth = 2;
    rrect(g, 30, 40, w - 60, h - 80, 36);
    g.stroke();
    font(g, 34, 600);
    g.letterSpacing = '4px';
    g.fillStyle = C.faint;
    g.fillText('IMPORT BY LINK', 90, 110);
    g.letterSpacing = '0px';
    dot(g, 108, 182, 16, C.spotify);
    font(g, 54, 500);
    g.fillStyle = C.text;
    g.textBaseline = 'middle';
    g.fillText('open.spotify.com/playlist/37i9dQZF1DXc…', 145, 184);
  });
}

export function importedChip() {
  return canvas(1100, 200, (g, w, h) => {
    const label = 'Imported · 50 tracks · no login';
    font(g, 58, 600);
    const tw = g.measureText(label).width;
    const pw = tw + 170;
    const x = (w - pw) / 2;
    shadowed(g, () => {
      rrect(g, x, 40, pw, h - 80, (h - 80) / 2);
      g.fillStyle = C.accent;
      g.fill();
    });
    g.strokeStyle = C.onAccent;
    g.lineWidth = 9;
    g.lineCap = 'round';
    g.lineJoin = 'round';
    g.beginPath();
    g.moveTo(x + 52, h / 2);
    g.lineTo(x + 70, h / 2 + 18);
    g.lineTo(x + 102, h / 2 - 16);
    g.stroke();
    g.fillStyle = C.onAccent;
    g.textBaseline = 'middle';
    g.fillText(label, x + 130, h / 2 + 3);
  });
}

const clock = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;

/** A rich presence card in the spirit of a Discord profile; `seconds` drives the bar. */
export function drawPresence(g, cover, seconds) {
  const w = g.canvas.width;
  const h = g.canvas.height;
  g.clearRect(0, 0, w, h);
  shadowed(g, () => {
    rrect(g, 30, 30, w - 60, h - 60, 40);
    g.fillStyle = '#1e1f24';
    g.fill();
  });
  g.strokeStyle = 'rgba(255,255,255,0.08)';
  g.lineWidth = 2;
  rrect(g, 30, 30, w - 60, h - 60, 40);
  g.stroke();

  font(g, 32, 700);
  g.letterSpacing = '4px';
  g.fillStyle = C.muted;
  g.textBaseline = 'alphabetic';
  g.fillText('LISTENING TO SCARPIFY', 90, 112);
  g.letterSpacing = '0px';

  const cs = 250;
  g.save();
  rrect(g, 90, 150, cs, cs, 22);
  g.clip();
  g.drawImage(cover, 928, 270, 214, 214, 90, 150, cs, cs);
  g.restore();
  // The app icon sits on the cover's corner, as small images do in presences.
  dot(g, 90 + cs - 6, 150 + cs - 6, 44, '#1e1f24');
  drawIcon(g, 90 + cs - 46, 150 + cs - 46, 80);

  const x = 390;
  font(g, 64, 700);
  g.fillStyle = C.text;
  g.fillText('Nicole Kidman', x, 215);
  font(g, 46, 400);
  g.fillStyle = C.muted;
  g.fillText('by ADÉLA', x, 280);

  const total = 181;
  const barX = x;
  const barW = w - x - 100;
  const k = Math.min(1, seconds / total);
  rrect(g, barX, 330, barW, 12, 6);
  g.fillStyle = '#3a3b42';
  g.fill();
  rrect(g, barX, 330, Math.max(12, barW * k), 12, 6);
  g.fillStyle = C.text;
  g.fill();
  font(g, 34, 500);
  g.fillStyle = C.muted;
  g.fillText(clock(seconds), barX, 395);
  g.textAlign = 'right';
  g.fillText(clock(total), barX + barW, 395);
  g.textAlign = 'left';
}

/** "Window · 200 MB" with a bar; `value` counts up, `full` is the bar's scale. */
export function drawMemory(g, title, sub, value, full, accent) {
  const w = g.canvas.width;
  const h = g.canvas.height;
  g.clearRect(0, 0, w, h);
  font(g, 50, 600);
  g.fillStyle = C.text;
  g.textBaseline = 'alphabetic';
  g.fillText(title, 20, 70);
  font(g, 38, 400);
  g.fillStyle = C.faint;
  g.fillText(sub, 20, 128);

  font(g, 120, 700);
  g.textAlign = 'right';
  g.fillStyle = accent ? C.accent : C.text;
  const unit = ' MB';
  font(g, 60, 600);
  const uw = g.measureText(unit).width;
  g.fillText(unit, w - 20, 128);
  font(g, 120, 700);
  g.fillText(`~${Math.round(value)}`, w - 20 - uw, 128);
  g.textAlign = 'left';

  rrect(g, 20, 172, w - 40, 22, 11);
  g.fillStyle = C.raised;
  g.fill();
  const k = Math.max(0, Math.min(1, value / full));
  if (k > 0) {
    rrect(g, 20, 172, Math.max(22, (w - 40) * k), 22, 11);
    g.fillStyle = accent ? C.accent : C.muted;
    g.fill();
  }
}

// ---------------------------------------------------------------- terminal mode

/**
 * The terminal interface inside a Windows Terminal window, laid out like the TUI:
 * 3-row titlebar, 32-column sidebar, home shelves, 5-row player bar. Covers are real
 * pictures (Sixel in the app), cut from the home screenshot.
 */
export function terminal(home) {
  return canvas(1920, 1200, (g, W, H) => {
    const tabH = 50;
    g.fillStyle = '#1f1f22';
    g.fillRect(0, 0, W, tabH);
    // Active tab.
    rrect(g, 10, 8, 300, tabH - 8 + 12, 10);
    g.fillStyle = C.bg;
    g.fill();
    drawIcon(g, 26, 16, 26);
    font(g, 19, 500);
    g.fillStyle = C.text;
    g.textBaseline = 'middle';
    g.fillText('SCARPIFY', 64, 30);
    g.fillStyle = C.faint;
    g.fillText('✕', 282, 30);
    g.fillText('+', 330, 30);
    g.fillText('⌄', 370, 27);
    font(g, 20, 300);
    g.fillStyle = C.muted;
    g.fillText('—', W - 160, 28);
    g.fillText('☐', W - 100, 30);
    g.fillText('✕', W - 42, 30);

    g.fillStyle = C.bg;
    g.fillRect(0, tabH, W, H - tabH);

    const size = 21;
    font(g, size, 400, MONO);
    const cw = g.measureText('M').width;
    const ch = 27;
    const ox = 8;
    const oy = tabH + 6;
    const cols = Math.floor((W - ox * 2) / cw);
    const rows = Math.floor((H - oy) / ch);
    const X = (c) => ox + c * cw;
    const Y = (r) => oy + r * ch;
    const fill = (c, r, w, h, color) => {
      g.fillStyle = color;
      g.fillRect(Math.round(X(c)), Math.round(Y(r)), Math.round(w * cw), h * ch);
    };
    const text = (c, r, s, color = C.text, weight = 400) => {
      font(g, size, weight, MONO);
      g.fillStyle = color;
      g.textBaseline = 'middle';
      g.fillText(s, X(c), Y(r) + ch / 2 + 1);
    };
    const art = (c, r, w, h, sx, sy, sw, sh) => {
      g.imageSmoothingQuality = 'high';
      g.drawImage(home, sx, sy, sw, sh, Math.round(X(c)), Math.round(Y(r)), Math.round(w * cw), h * ch);
    };
    const badge = (c, r, color, name) => {
      text(c, r, '●', color);
      text(c + 2, r, name, C.faint);
    };

    const playerRows = 5;
    const midTop = 3;
    const midBottom = rows - playerRows; // exclusive
    // Titlebar: back/forward, the search field, Telegram.
    text(2, 1, '‹  ›', C.faint);
    const sw = 64;
    const sx = Math.floor((cols - sw) / 2);
    fill(sx, 0, sw, 3, C.surface);
    text(sx + 2, 1, '🔍', C.faint);
    text(sx + 6, 1, 'What do you want to listen to?', C.faint);
    text(cols - 4, 1, '✈️', C.muted);

    // Sidebar.
    const sbw = 32;
    fill(1, midTop, sbw, 6, C.surface);
    text(3, midTop + 1, 'S C A R P I F Y', C.text, 700);
    text(3, midTop + 3, '🏠', C.text);
    text(7, midTop + 3, 'Home', C.text, 700);
    text(3, midTop + 4, '🔍', C.muted);
    text(7, midTop + 4, 'Search', C.muted);

    const libTop = midTop + 7;
    fill(1, libTop, sbw, midBottom - libTop, C.surface);
    text(3, libTop + 1, '📚', C.muted);
    text(7, libTop + 1, 'Your library', C.muted, 700);
    text(sbw - 2, libTop + 1, '➕', C.muted);
    const entries = [
      ['Liked tracks', '3 tracks', null, 375],
      ['Night Drive', '📌 8 tracks', null, 463],
      ['Deep Focus', '8 tracks', null, 551],
      ["Today's Top Hits", '50 tracks', C.spotify, 639],
      ['The Hit List', '57 tracks', C.youtube, 727],
    ];
    entries.forEach(([name, count, color, y], i) => {
      const r = libTop + 3 + i * 3;
      if (i === 3) fill(2, r - 0, sbw - 2, 2, C.hover);
      art(3, r, 4, 2, 32, y, 64, 64);
      text(8, r, name, i === 3 ? C.accent : C.text);
      if (color) {
        text(8, r + 1, '●', color);
        text(10, r + 1, count, C.muted);
      } else text(8, r + 1, count, C.muted);
    });
    text(3, midBottom - 2, '🔧', C.muted);
    text(7, midBottom - 2, 'Settings', C.muted);

    // Home page.
    const mx = sbw + 2;
    const mw = cols - mx - 1;
    fill(mx, midTop, mw, midBottom - midTop, C.surface);
    text(mx + 2, midTop + 1, 'Good night', C.text, 700);
    text(mx + 2, midTop + 3, 'Recently played', C.text, 700);
    const coverW = 14;
    const coverH = 6;
    const recent = [
      ['Patient Zero', 'Taylor Swift', C.spotify, 'Spotify'],
      ['the cure', 'Olivia Rodrigo', C.spotify, 'Spotify'],
      ['Nicole Kidman', 'ADÉLA', C.spotify, 'Spotify'],
      ['Earrings', 'Malcolm Todd', C.spotify, 'Spotify'],
      ['BbY WOW', 'KAROL G', C.spotify, 'Spotify'],
      ["Can't Feel My Face", 'The Weeknd', C.youtube, 'YouTube Music'],
    ];
    const xs = [437, 682, 928, 1173, 1418, 1664];
    const shelf = (r, items, sy) => {
      items.forEach(([name, artist, color, svc], i) => {
        const c = mx + 2 + i * (coverW + 3);
        if (c + coverW > mx + mw) return;
        art(c, r, coverW, coverH, xs[i], sy, 214, 214);
        text(c, r + coverH + 1, name.length > coverW + 2 ? name.slice(0, coverW) + '…' : name, C.text, 700);
        if (artist) text(c, r + coverH + 2, artist, C.muted);
        if (color) badge(c, r + coverH + 3, color, svc);
      });
    };
    shelf(midTop + 5, recent, 270);
    text(mx + 2, midTop + 16, 'Your playlists', C.text, 700);
    shelf(
      midTop + 18,
      [
        ['Liked tracks', '3 tracks'],
        ['Night Drive', '8 tracks'],
        ['Deep Focus', '8 tracks'],
        ["Today's Top Hits", '50 tracks', C.spotify, 'Spotify'],
        ['The Hit List', '57 tracks', C.youtube, 'YouTube Music'],
      ],
      706,
    );

    // Player bar.
    const pr = midBottom;
    art(1, pr + 1, 6, 3, 928, 270, 214, 214);
    text(8, pr + 1, 'Nicole Kidman', C.text, 700);
    text(8, pr + 2, 'ADÉLA', C.muted);
    badge(8, pr + 3, C.spotify, 'Spotify');
    const cx = Math.floor(cols / 2) - 11;
    text(cx, pr + 1, ' ⇄ ', C.muted);
    text(cx + 5, pr + 1, '|◀ ', C.muted);
    fill(cx + 9, pr + 1, 5, 1, C.text);
    text(cx + 10.5, pr + 1, '❚❚', C.bg, 700);
    text(cx + 15, pr + 1, ' ▶|', C.muted);
    text(cx + 20, pr + 1, ' ↻ ', C.muted);
    const barL = 44;
    const barR = cols - 44;
    text(barL - 5, pr + 3, '1:10', C.faint);
    const filled = Math.round((barR - barL) * 0.39);
    text(barL, pr + 3, '━'.repeat(filled), C.text);
    text(barL + filled, pr + 3, '━'.repeat(barR - barL - filled), C.active);
    text(barR + 1, pr + 3, '3:01', C.faint);
    text(cols - 26, pr + 2, '📜', C.muted);
    text(cols - 22, pr + 2, '🔊', C.muted);
    text(cols - 18, pr + 2, '━'.repeat(11), C.text);
    text(cols - 7, pr + 2, '━'.repeat(5), C.active);
  });
}
