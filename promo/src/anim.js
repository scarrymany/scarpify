// Pure functions of time: every frame of the video is computed from `t` alone,
// so the browser preview and the offline render produce the same picture.

export const clamp = (x, a = 0, b = 1) => Math.min(b, Math.max(a, x));
export const lerp = (a, b, k) => a + (b - a) * k;
export const range = (t, a, b) => clamp((t - a) / (b - a));

export const easeInOutCubic = (x) => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
export const easeOutCubic = (x) => 1 - Math.pow(1 - x, 3);
export const easeOutQuint = (x) => 1 - Math.pow(1 - x, 5);
export const easeInOutSine = (x) => -(Math.cos(Math.PI * x) - 1) / 2;
export const easeInCubic = (x) => x * x * x;

/** 0 before `a`, eases up to 1 by `b`, holds, eases back to 0 between `c` and `d`. */
export function window4(t, a, b, c, d, ease = easeInOutCubic) {
  if (t <= a || t >= d) return 0;
  if (t < b) return ease(range(t, a, b));
  if (t <= c) return 1;
  return 1 - ease(range(t, c, d));
}

/** Deterministic pseudo-random numbers (mulberry32). */
export function seeded(seed) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let x = s;
    x = Math.imul(x ^ (x >>> 15), x | 1);
    x ^= x + Math.imul(x ^ (x >>> 7), x | 61);
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * A smooth path through timed keys of number arrays. Tangents follow Catmull-Rom over
 * non-uniform times, so speed never jumps at a key; the first and last keys start and end
 * at rest. A key with `hold: true` also comes to rest.
 */
export function track(keys) {
  const n = keys.length;
  const dim = keys[0].v.length;
  const tangents = keys.map((k, i) => {
    if (i === 0 || i === n - 1 || k.hold) return new Array(dim).fill(0);
    const a = keys[i - 1];
    const b = keys[i + 1];
    return k.v.map((_, d) => (b.v[d] - a.v[d]) / (b.t - a.t));
  });
  return (t) => {
    if (t <= keys[0].t) return keys[0].v.slice();
    if (t >= keys[n - 1].t) return keys[n - 1].v.slice();
    let i = 0;
    while (t > keys[i + 1].t) i++;
    const a = keys[i];
    const b = keys[i + 1];
    const h = b.t - a.t;
    const s = (t - a.t) / h;
    const s2 = s * s;
    const s3 = s2 * s;
    const h00 = 2 * s3 - 3 * s2 + 1;
    const h10 = s3 - 2 * s2 + s;
    const h01 = -2 * s3 + 3 * s2;
    const h11 = s3 - s2;
    return a.v.map((_, d) => h00 * a.v[d] + h10 * h * tangents[i][d] + h01 * b.v[d] + h11 * h * tangents[i + 1][d]);
  };
}
