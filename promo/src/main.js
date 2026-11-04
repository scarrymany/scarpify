import * as THREE from 'three';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';
import '@fontsource-variable/onest';
import '@fontsource/jetbrains-mono/400.css';
import '@fontsource/jetbrains-mono/700.css';

import homeUrl from '../../docs/screenshots/home.png';
import playlistUrl from '../../docs/screenshots/playlist.png';
import searchUrl from '../../docs/screenshots/search.png';
import * as art from './art.js';
import {
  clamp, lerp, range, easeInOutCubic, easeOutCubic, easeOutQuint, easeInOutSine, window4, seeded, track,
} from './anim.js';

export const DURATION = 30;
export const FPS = 60;
const W = 1920;
const H = 1080;
const FLOOR = -1.28;
const RENDER = new URLSearchParams(location.search).has('render');
if (RENDER) document.body.classList.add('render');

// ------------------------------------------------------------------ renderer and layers
//
// Three passes per frame: the 3D world through bloom, then crisp UI pieces placed in the
// same 3D space (no bloom, so text stays sharp), then screen-space captions.

const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
renderer.setPixelRatio(1);
renderer.setSize(W, H, RENDER);
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.NoToneMapping;
renderer.autoClear = false;
document.body.appendChild(renderer.domElement);
const maxAniso = renderer.capabilities.getMaxAnisotropy();

const camera = new THREE.PerspectiveCamera(35, W / H, 0.1, 100);
const world = new THREE.Scene();
world.background = new THREE.Color(art.C.bg);
const crisp = new THREE.Scene();
const overlay = new THREE.Scene();
const ortho = new THREE.OrthographicCamera(0, W, 0, -H, -10, 10); // y runs down: top = 0

const composer = new EffectComposer(
  renderer,
  new THREE.WebGLRenderTarget(W, H, { type: THREE.HalfFloatType, samples: 4 }),
);
composer.addPass(new RenderPass(world, camera));
const bloom = new UnrealBloomPass(new THREE.Vector2(W, H), 0.55, 0.65, 0.92);
composer.addPass(bloom);
composer.addPass(new OutputPass());

function texture(source) {
  const t = source instanceof HTMLCanvasElement ? new THREE.CanvasTexture(source) : new THREE.Texture(source);
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = maxAniso;
  t.minFilter = THREE.LinearMipmapLinearFilter;
  t.needsUpdate = true;
  return t;
}

// ------------------------------------------------------------------ materials

const accentHDR = new THREE.Color(art.C.accent);

const screenVertex = /* glsl */ `
  varying vec2 vUv;
  varying vec3 vWorld;
  void main() {
    vUv = uv;
    vec4 w = modelMatrix * vec4(position, 1.0);
    vWorld = w.xyz;
    gl_Position = projectionMatrix * viewMatrix * w;
  }
`;

/** An app window: rounded corners, a thin rim of light, a passing sheen, dimming. */
function screenMaterial(map, size, mirrored) {
  return new THREE.ShaderMaterial({
    transparent: true,
    side: THREE.DoubleSide,
    depthWrite: !mirrored,
    depthTest: !mirrored,
    uniforms: {
      map: { value: map },
      uSize: { value: new THREE.Vector2(size.x, size.y) },
      uRadius: { value: 0.07 },
      uOpacity: { value: 1 },
      uBright: { value: 1 },
      uSheen: { value: -2 },
      uFloor: { value: FLOOR },
      uMirror: { value: mirrored ? 1 : 0 },
    },
    vertexShader: screenVertex,
    fragmentShader: /* glsl */ `
      uniform sampler2D map;
      uniform vec2 uSize;
      uniform float uRadius, uOpacity, uBright, uSheen, uFloor, uMirror;
      varying vec2 vUv;
      varying vec3 vWorld;
      void main() {
        vec2 p = (vUv - 0.5) * uSize;
        vec2 q = abs(p) - (uSize * 0.5 - uRadius);
        float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - uRadius;
        float aa = fwidth(d) * 1.2;
        float alpha = 1.0 - smoothstep(-aa, aa, d);
        vec3 col = texture2D(map, vUv).rgb * uBright;
        // Light from above: the top of the glass is a touch brighter.
        col += vec3(0.010, 0.009, 0.008) * smoothstep(0.3, 1.0, vUv.y);
        float band = vUv.x * 0.8 + vUv.y * 0.45 - uSheen;
        col += vec3(1.0, 0.93, 0.88) * exp(-band * band * 90.0) * 0.045;
        col += vec3(1.0, 0.94, 0.9) * exp(-band * band * 6.0) * 0.012;
        float rim = 1.0 - smoothstep(0.0, aa * 2.5, abs(d + aa * 1.5));
        col += vec3(1.0) * rim * 0.10 * (0.4 + 0.6 * vUv.y);
        float a = alpha * uOpacity;
        if (uMirror > 0.5) {
          float depth = uFloor - vWorld.y;
          a *= 0.16 * exp(-depth * 2.6) * smoothstep(0.0, 0.05, depth);
          col *= 0.9;
        }
        if (a < 0.002) discard;
        gl_FragColor = vec4(col, a);
        #include <colorspace_fragment>
      }
    `,
  });
}

/** Text and UI textures: a left-to-right reveal where each column rises into place. */
function textMaterial(map) {
  return new THREE.ShaderMaterial({
    transparent: true,
    depthWrite: false,
    depthTest: false,
    uniforms: {
      map: { value: map },
      uOpacity: { value: 0 },
      uReveal: { value: 1 },
      uRise: { value: 0.18 },
    },
    vertexShader: /* glsl */ `
      varying vec2 vUv;
      void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }
    `,
    fragmentShader: /* glsl */ `
      uniform sampler2D map;
      uniform float uOpacity, uReveal, uRise;
      varying vec2 vUv;
      void main() {
        const float S = 0.35;
        float k = clamp((uReveal * (1.0 + S) - vUv.x) / S, 0.0, 1.0);
        k = k * k * (3.0 - 2.0 * k);
        vec2 uv = vec2(vUv.x, vUv.y + (1.0 - k) * uRise);
        vec4 c = texture2D(map, uv);
        if (uv.y > 1.0) c.a = 0.0;
        float a = c.a * k * uOpacity;
        if (a < 0.002) discard;
        gl_FragColor = vec4(c.rgb, a);
        #include <colorspace_fragment>
      }
    `,
  });
}

function textMesh(source, width) {
  const tex = texture(source);
  const height = (width * source.height) / source.width;
  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(width, height), textMaterial(tex));
  mesh.userData.source = source;
  mesh.userData.tex = tex;
  return mesh;
}

/** A screen-space plane placed by its centre in 1920×1080 pixels. */
function overlayText(source, cx, cy, width) {
  const m = textMesh(source, width);
  m.position.set(cx, -cy, 0);
  overlay.add(m);
  return m;
}

function setText(m, opacity, reveal = 1) {
  m.material.uniforms.uOpacity.value = opacity;
  m.material.uniforms.uReveal.value = reveal;
  m.visible = opacity > 0.001 && reveal > 0;
}

// ------------------------------------------------------------------ environment

const floor = new THREE.Mesh(
  new THREE.PlaneGeometry(80, 80),
  new THREE.ShaderMaterial({
    transparent: true,
    depthWrite: false,
    uniforms: { uBg: { value: new THREE.Color(art.C.bg) }, uLift: { value: new THREE.Color('#141418') } },
    vertexShader: screenVertex,
    fragmentShader: /* glsl */ `
      uniform vec3 uBg, uLift;
      varying vec3 vWorld;
      void main() {
        float d = length(vWorld.xz - vec2(0.0, 0.5));
        vec3 col = mix(uLift, uBg, smoothstep(1.0, 11.0, d));
        gl_FragColor = vec4(col, 1.0 - smoothstep(5.0, 15.0, d));
        #include <colorspace_fragment>
      }
    `,
  }),
);
floor.rotation.x = -Math.PI / 2;
floor.position.y = FLOOR;
world.add(floor);

// A warm haze far behind everything.
const haze = new THREE.Mesh(
  new THREE.PlaneGeometry(60, 26),
  new THREE.ShaderMaterial({
    transparent: true,
    depthWrite: false,
    uniforms: { uColor: { value: new THREE.Color(art.C.accent) }, uAmount: { value: 0.03 } },
    vertexShader: screenVertex,
    fragmentShader: /* glsl */ `
      uniform vec3 uColor;
      uniform float uAmount;
      varying vec2 vUv;
      void main() {
        vec2 p = (vUv - vec2(0.5, 0.42)) * vec2(2.3, 1.0);
        float g = exp(-dot(p, p) * 6.0);
        gl_FragColor = vec4(uColor, g * uAmount);
        #include <colorspace_fragment>
      }
    `,
  }),
);
haze.position.set(0, 1.0, -14);
haze.renderOrder = -2;
floor.renderOrder = -1;
world.add(haze);

// Dust: drifting motes, positions computed in the shader from time alone.
const DUST = 900;
const dust = (() => {
  const rnd = seeded(7);
  const pos = new Float32Array(DUST * 3);
  const seed = new Float32Array(DUST * 4);
  for (let i = 0; i < DUST; i++) {
    pos.set([(rnd() - 0.5) * 22, rnd() * 6.5, -12 + rnd() * 20], i * 3);
    seed.set([rnd(), rnd(), rnd(), rnd()], i * 4);
  }
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  geo.setAttribute('seed', new THREE.BufferAttribute(seed, 4));
  const mat = new THREE.ShaderMaterial({
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    uniforms: {
      uTime: { value: 0 },
      uAmount: { value: 1 },
      uWarm: { value: new THREE.Color(art.C.accent) },
      uScale: { value: H * 0.5 },
    },
    vertexShader: /* glsl */ `
      attribute vec4 seed;
      uniform float uTime, uScale;
      varying float vFade;
      varying vec4 vSeed;
      void main() {
        vec3 p = position;
        float speed = 0.05 + seed.x * 0.10;
        p.y = mod(p.y + uTime * speed, 6.5) - 1.3;
        p.x += sin(uTime * (0.2 + seed.y * 0.3) + seed.z * 6.28) * 0.35;
        p.z += cos(uTime * (0.15 + seed.w * 0.25) + seed.y * 6.28) * 0.35;
        vec4 mv = modelViewMatrix * vec4(p, 1.0);
        gl_Position = projectionMatrix * mv;
        gl_PointSize = (0.012 + seed.w * 0.026) * uScale / -mv.z;
        float yk = (p.y + 1.3) / 6.5;
        vFade = smoothstep(0.0, 0.12, yk) * smoothstep(1.0, 0.75, yk) * smoothstep(0.4, 2.5, -mv.z);
        vSeed = seed;
      }
    `,
    fragmentShader: /* glsl */ `
      uniform float uAmount, uTime;
      uniform vec3 uWarm;
      varying float vFade;
      varying vec4 vSeed;
      void main() {
        vec2 c = gl_PointCoord - 0.5;
        float r = length(c);
        float soft = smoothstep(0.5, 0.0, r);
        float twinkle = 0.65 + 0.35 * sin(uTime * (0.8 + vSeed.x * 1.5) + vSeed.z * 20.0);
        vec3 col = mix(vec3(1.0, 0.92, 0.85), uWarm, vSeed.y * 0.8);
        float bright = vSeed.x > 0.93 ? 1.7 : 0.55;
        gl_FragColor = vec4(col * bright, soft * soft * vFade * twinkle * uAmount * 0.75);
        #include <colorspace_fragment>
      }
    `,
  });
  const points = new THREE.Points(geo, mat);
  points.frustumCulled = false;
  world.add(points);
  return mat;
})();

// ------------------------------------------------------------------ the logo in 3D

const LOGO_SCALE = 1 / 400;
const logo = new THREE.Group();
world.add(logo);
const logoDisc = new THREE.Mesh(
  new THREE.CircleGeometry(464 * LOGO_SCALE, 128),
  new THREE.MeshBasicMaterial({ color: art.C.surface, transparent: true }),
);
logo.add(logoDisc);
const logoRim = new THREE.Mesh(
  new THREE.RingGeometry(464 * LOGO_SCALE, 472 * LOGO_SCALE, 128),
  new THREE.MeshBasicMaterial({ color: '#34343c', transparent: true }),
);
logo.add(logoRim);

const icon = (x, y) => new THREE.Vector3((x - 512) * LOGO_SCALE, -(y - 512) * LOGO_SCALE, 0.03);
const arcs = [
  [76, [262, 382], [420, 322], [612, 330], [772, 410]],
  [64, [298, 532], [432, 486], [592, 494], [726, 560]],
  [54, [334, 670], [444, 636], [572, 642], [680, 694]],
].map(([w, a, b, c, d]) => {
  const curve = new THREE.CubicBezierCurve3(icon(...a), icon(...b), icon(...c), icon(...d));
  const r = (w / 2) * LOGO_SCALE;
  const segments = 160;
  const mat = new THREE.MeshBasicMaterial({ color: accentHDR, transparent: true });
  const tube = new THREE.Mesh(new THREE.TubeGeometry(curve, segments, r, 24, false), mat);
  const capGeo = new THREE.SphereGeometry(r, 24, 16);
  const start = new THREE.Mesh(capGeo, mat);
  const end = new THREE.Mesh(capGeo, mat);
  start.position.copy(curve.getPoint(0));
  logo.add(tube, start, end);
  return { curve, tube, start, end, mat, segments };
});

const halo = new THREE.Mesh(
  new THREE.PlaneGeometry(6, 6),
  new THREE.ShaderMaterial({
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    uniforms: { uColor: { value: accentHDR }, uAmount: { value: 0 } },
    vertexShader: screenVertex,
    fragmentShader: /* glsl */ `
      uniform vec3 uColor;
      uniform float uAmount;
      varying vec2 vUv;
      void main() {
        vec2 p = vUv - 0.5;
        float r = length(p);
        float g = exp(-r * r * 26.0) * 0.5 + exp(-r * r * 90.0) * 0.5;
        gl_FragColor = vec4(uColor * g * uAmount, 1.0);
        #include <colorspace_fragment>
      }
    `,
  }),
);
halo.position.z = -0.05;
logo.add(halo);

function setLogo(progresses, opacity) {
  halo.material.uniforms.uAmount.value = 0.22 * opacity * (0.3 + 0.7 * Math.min(...progresses.map((p) => clamp(p))));
  arcs.forEach((arc, i) => {
    const p = clamp(progresses[i]);
    const shown = Math.floor(p * arc.segments);
    arc.tube.geometry.setDrawRange(0, shown * 24 * 6);
    arc.start.visible = p > 0.001;
    arc.end.visible = p > 0.001;
    arc.end.position.copy(arc.curve.getPoint(p));
    arc.mat.opacity = opacity;
  });
  logoDisc.material.opacity = opacity;
  logoRim.material.opacity = opacity;
  logo.visible = opacity > 0.001;
}

// ------------------------------------------------------------------ app windows

const SCREEN = new THREE.Vector2(3.2, 2.0);
const reflect = new THREE.Matrix4()
  .makeTranslation(0, FLOOR, 0)
  .multiply(new THREE.Matrix4().makeScale(1, -1, 1))
  .multiply(new THREE.Matrix4().makeTranslation(0, -FLOOR, 0));

const shadowTex = texture(
  art.canvas(256, 128, (g, w, h) => {
    const grad = g.createRadialGradient(w / 2, h / 2, 0, w / 2, h / 2, w / 2);
    grad.addColorStop(0, 'rgba(0,0,0,0.85)');
    grad.addColorStop(1, 'rgba(0,0,0,0)');
    g.setTransform(1, 0, 0, 0.5, 0, h / 4);
    g.fillStyle = grad;
    g.fillRect(0, 0, w, h * 2);
  }),
);

const screens = [];
function makeScreen(source, size = SCREEN) {
  const tex = texture(source);
  const geo = new THREE.PlaneGeometry(size.x, size.y);
  const mat = screenMaterial(tex, size, false);
  const mesh = new THREE.Mesh(geo, mat);
  mesh.renderOrder = 3;
  const mirrorMat = screenMaterial(tex, size, true);
  const mirror = new THREE.Mesh(geo, mirrorMat);
  mirror.matrixAutoUpdate = false;
  mirror.renderOrder = 2;
  const shadow = new THREE.Mesh(
    new THREE.PlaneGeometry(size.x * 1.25, 1.1),
    new THREE.MeshBasicMaterial({ map: shadowTex, transparent: true, depthWrite: false }),
  );
  shadow.rotation.x = -Math.PI / 2;
  shadow.renderOrder = 1;
  world.add(mesh, mirror, shadow);
  const s = { mesh, mat, mirror, mirrorMat, shadow, size };
  screens.push(s);
  return s;
}

const tmpQ = new THREE.Quaternion();
const tmpE = new THREE.Euler();
function placeScreen(s, { x, y, z, ry = 0, rx = 0, scale = 1, opacity = 1, bright = 1, sheen = -2 }) {
  s.mesh.position.set(x, y, z);
  s.mesh.rotation.set(rx, ry, 0, 'YXZ');
  s.mesh.scale.setScalar(scale);
  s.mesh.updateMatrixWorld(true);
  s.mirror.matrix.multiplyMatrices(reflect, s.mesh.matrixWorld);
  s.mirror.matrixWorldNeedsUpdate = true;
  for (const m of [s.mat, s.mirrorMat]) {
    m.uniforms.uOpacity.value = opacity;
    m.uniforms.uBright.value = bright;
    m.uniforms.uSheen.value = sheen;
  }
  const lift = y - (s.size.y / 2) * scale - FLOOR;
  s.shadow.position.set(x, FLOOR + 0.002, z + 0.05);
  s.shadow.rotation.set(-Math.PI / 2, 0, ry);
  s.shadow.scale.setScalar(scale);
  s.shadow.material.opacity = opacity * clamp(0.75 - lift * 0.9, 0.15, 0.75);
  const vis = opacity > 0.002;
  s.mesh.visible = s.mirror.visible = s.shadow.visible = vis;
}

/** Puts a crisp-layer object at a point in a screen's own space, so it floats with it. */
const tmpM = new THREE.Matrix4();
function pin(obj, s, lx, ly, lz, ry = 0, extraScale = 1) {
  obj.matrixAutoUpdate = false;
  tmpE.set(0, ry, 0);
  tmpQ.setFromEuler(tmpE);
  tmpM.compose(new THREE.Vector3(lx, ly, lz), tmpQ, new THREE.Vector3(extraScale, extraScale, extraScale));
  obj.matrix.multiplyMatrices(s.mesh.matrixWorld, tmpM);
  obj.matrixWorldNeedsUpdate = true;
}

// ------------------------------------------------------------------ overlay helpers

const shade = new THREE.Mesh(
  new THREE.PlaneGeometry(W, H),
  new THREE.ShaderMaterial({
    transparent: true,
    depthTest: false,
    depthWrite: false,
    uniforms: { uCaption: { value: 0 }, uBlack: { value: 1 }, uFrame: { value: 0 } },
    vertexShader: /* glsl */ `
      varying vec2 vUv;
      void main() { vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }
    `,
    fragmentShader: /* glsl */ `
      uniform float uCaption, uBlack, uFrame;
      varying vec2 vUv;
      float hash(vec2 p) { return fract(sin(dot(p, vec2(12.9898, 78.233)) + uFrame * 0.618) * 43758.5453); }
      void main() {
        vec2 p = (vUv - 0.5) * vec2(1.6, 1.0);
        float vignette = smoothstep(0.45, 1.05, length(p)) * 0.55;
        // A soft floor of shade behind captions, bottom-left.
        vec2 c = (vUv - vec2(0.12, -0.05)) * vec2(1.0, 1.9);
        float cap = exp(-dot(c, c) * 3.2) * 0.62 * uCaption;
        float a = clamp(1.0 - (1.0 - vignette) * (1.0 - cap), 0.0, 1.0);
        a = mix(a, 1.0, uBlack);
        // Fine noise keeps the dark gradients from banding after encoding.
        float n = (hash(gl_FragCoord.xy) - 0.5) / 255.0 * 1.6;
        gl_FragColor = vec4(vec3(12.0 / 255.0, 12.0 / 255.0, 14.0 / 255.0) * (1.0 - uBlack) + n, a);
      }
    `,
  }),
);
shade.position.set(W / 2, -H / 2, -1);
shade.renderOrder = -1;
overlay.add(shade);

// ------------------------------------------------------------------ build everything

let S = null;

async function build() {
  await Promise.all([
    document.fonts.load(`700 120px "Onest Variable"`),
    document.fonts.load(`400 40px "Onest Variable"`),
    document.fonts.load(`400 20px "JetBrains Mono"`),
    document.fonts.load(`700 20px "JetBrains Mono"`),
  ]);
  await document.fonts.ready;
  const [home, playlist, search] = await Promise.all([homeUrl, playlistUrl, searchUrl].map(art.loadImage));

  const scr = {
    home: makeScreen(home),
    playlist: makeScreen(playlist),
    search: makeScreen(search),
    term: makeScreen(art.terminal(home)),
  };

  const t3 = (source, width) => {
    const m = textMesh(source, width);
    crisp.add(m);
    return m;
  };

  const ui = {
    yt: t3(art.chip('YouTube Music', art.C.youtube), 1.5),
    sc: t3(art.chip('SoundCloud', art.C.soundcloud), 1.5),
    noAds: t3(art.chip('No ads', null, { accent: true }), 1.5),
    link: t3(art.linkCard(), 2.1),
    imported: t3(art.importedChip(), 1.6),
    presence: t3(art.canvas(1200, 470), 1.8),
    memWin: t3(art.canvas(1100, 210), 2.35),
    memTerm: t3(art.canvas(1100, 210), 2.35),
  };

  const ov = {
    word: overlayText(art.wordmark(), W / 2, 652, 800),
    tagline: overlayText(art.line('No ads, no accounts, one library.', { size: 46, color: art.C.muted }), W / 2, 742, 900),
    word2: overlayText(art.wordmark(), W / 2, 652, 800),
    sub2: overlayText(
      art.line('Free and open source · Windows 10 and 11', { size: 40, color: art.C.faint }),
      W / 2,
      730,
      900,
    ),
    github: overlayText(art.githubPill(), W / 2, 834, 750),
    captions: [
      ['The app', 'Your music, one calm window'],
      ['01 · No ads', 'Ad-free player for YouTube Music and SoundCloud'],
      ['02 · No login', 'Spotify playlists imported without an account'],
      ['03 · Discord', 'What you play shows in your Discord status'],
      ['04 · Light', '~200 MB as a window, ~30 MB in the terminal'],
    ].map(([k, text]) => overlayText(art.caption(k, text), 0, 0, 1600)),
  };
  ov.captions.forEach((m) => {
    m.geometry.translate(800, 0, 0); // anchor at the left edge
    m.material.uniforms.uRise.value = 0.12;
  });

  S = { scr, ui, ov, cover: home };
}

// ------------------------------------------------------------------ timeline

// Where each window rests during the tour.
const LAYOUT = {
  playlist: { x: 0, y: 0, z: 0, ry: 0 },
  home: { x: -3.15, y: 0.02, z: -1.35, ry: 0.5 },
  search: { x: 3.15, y: 0.02, z: -1.35, ry: -0.5 },
  homePair: { x: -1.74, y: 0.12, z: 0.9, ry: 0.16 },
  termPair: { x: 1.74, y: 0.12, z: 0.9, ry: -0.16 },
};

/** A camera pose that frames a window, nudged so the window sits right of centre. */
function frame(l, dist, shift, up = 0.25, lift = 0) {
  const n = [Math.sin(l.ry), 0, Math.cos(l.ry)];
  const r = [Math.cos(l.ry), 0, -Math.sin(l.ry)];
  const tx = l.x - r[0] * shift;
  const ty = l.y + lift;
  const tz = l.z - r[2] * shift;
  return [tx + n[0] * dist, ty + up, tz + n[2] * dist, tx, ty, tz];
}

const TITLE_POSE = [0, 0.3, 8.0, 0, 0.3, 2.5];
const f1 = frame({ ...LAYOUT.search, ry: -0.12 }, 5.4, 1.05, 0.3, -0.12);
const f2 = frame(LAYOUT.playlist, 4.9, 0.85, 0.3, -0.15);
const f3 = frame({ ...LAYOUT.playlist, ry: -0.18 }, 4.6, 0.55, 0.22, -0.2);
const camTrack = track([
  { t: 0, v: [0, 0.3, 8.5, 0, 0.3, 2.5] },
  { t: 3.3, v: TITLE_POSE },
  { t: 5.9, v: [-2.3, 0.95, 7.7, 0, -0.1, 0] },
  { t: 9.0, v: [0.3, 0.5, 6.6, 0, -0.1, 0] },
  { t: 11.6, v: [1.6, 0.5, 6.0, 1.1, -0.05, -0.45] },
  { t: 13.4, v: f1 },
  { t: 14.6, v: f1.map((v, i) => (i === 2 ? v - 0.25 : i === 0 ? v + 0.12 : v)) },
  { t: 16.1, v: f2 },
  { t: 17.3, v: f2.map((v, i) => (i === 2 ? v - 0.2 : v)) },
  { t: 18.9, v: f3 },
  { t: 20.1, v: f3.map((v, i) => (i === 2 ? v - 0.2 : v)) },
  { t: 22.0, v: [0, 0.42, 7.9, 0, 0.18, 0.9] },
  { t: 24.4, v: [0, 0.4, 7.5, 0, 0.18, 0.9] },
  { t: 26.6, v: TITLE_POSE },
  { t: 30, v: [0, 0.3, 7.75, 0, 0.3, 2.5] },
]);

function sheenAt(t, starts) {
  for (const s of starts) {
    if (t >= s && t <= s + 2.0) return lerp(-0.6, 1.9, easeInOutSine(range(t, s, s + 2.0)));
  }
  return -2;
}

const lerpL = (a, b, k) => ({ x: lerp(a.x, b.x, k), y: lerp(a.y, b.y, k), z: lerp(a.z, b.z, k), ry: lerp(a.ry, b.ry, k) });

function update(t) {
  const { scr, ui, ov } = S;

  // Camera, with a slow breath on top of the path.
  const c = camTrack(t);
  camera.position.set(c[0] + Math.sin(t * 0.37) * 0.04, c[1] + Math.sin(t * 0.29 + 1) * 0.03, c[2]);
  camera.lookAt(c[3], c[4], c[5]);
  camera.updateMatrixWorld();

  // Environment.
  dust.uniforms.uTime.value = t + 40;
  dust.uniforms.uAmount.value = 0.7 + 0.3 * window4(t, 0, 1, 3.4, 5) + 0.3 * window4(t, 24.8, 26.5, 30, 31);
  haze.material.uniforms.uAmount.value = 0.022 + 0.018 * window4(t, 0, 1.5, 3.5, 5.5) + 0.018 * range(t, 25, 27);

  // Logo: draws itself, leans gently, leaves upward, returns for the close.
  const LOGO_AT = new THREE.Vector3(0, 0.82, 2.5);
  const intro = [0, 1, 2].map((i) => easeInOutCubic(range(t, 0.35 + i * 0.2, 1.45 + i * 0.2)));
  const outro = [0, 1, 2].map((i) => easeInOutCubic(range(t, 25.5 + i * 0.2, 26.6 + i * 0.2)));
  const leave = easeInOutCubic(range(t, 3.3, 4.5));
  const back = easeOutCubic(range(t, 25.0, 26.4));
  const isOutro = t > 20;
  const discIn = isOutro ? back : easeOutCubic(range(t, 0.05, 1.0));
  setLogo(isOutro ? outro : intro, isOutro ? back : discIn * (1 - leave));
  const s = 0.55 * (isOutro ? lerp(0.86, 1, back) : lerp(0.86, 1, discIn) * lerp(1, 0.9, leave));
  logo.scale.setScalar(s);
  const away = isOutro ? 0 : leave;
  logo.position.set(LOGO_AT.x, LOGO_AT.y + away * 0.5 + (isOutro ? (1 - back) * -0.15 : 0), LOGO_AT.z - away * 0.6);
  logo.rotation.set(Math.sin(t * 0.5) * 0.06, Math.sin(t * 0.42 + 0.6) * 0.16, 0);

  // Titles.
  const wordIn = easeOutCubic(range(t, 1.25, 2.6));
  const wordOut = easeInOutCubic(range(t, 3.1, 3.9));
  setText(ov.word, 1 - wordOut, wordIn);
  setText(ov.tagline, easeOutCubic(range(t, 2.0, 2.9)) * (1 - wordOut), 1);
  ov.tagline.position.y = -742 + (1 - easeOutCubic(range(t, 2.0, 2.9))) * -10;
  setText(ov.word2, 1, easeOutCubic(range(t, 26.0, 27.3)));
  const sub = easeOutCubic(range(t, 26.8, 27.7));
  setText(ov.sub2, sub, 1);
  ov.sub2.position.y = -730 - (1 - sub) * 10;
  const gh = easeOutCubic(range(t, 27.2, 28.2));
  setText(ov.github, gh, 1);
  ov.github.position.y = -834 - (1 - gh) * 14;

  // Captions: [start, end] for the tour and each feature.
  const CAPS = [
    [6.1, 11.6],
    [12.9, 15.0],
    [15.75, 17.85],
    [18.55, 20.65],
    [21.45, 24.5],
  ];
  let capShade = 0;
  CAPS.forEach(([a, b], i) => {
    const m = ov.captions[i];
    const inK = easeOutCubic(range(t, a, a + 0.9));
    const outK = easeInOutCubic(range(t, b - 0.4, b));
    setText(m, 1 - outK, inK);
    m.position.set(118 - outK * 0, -(926 + outK * -12), 0);
    capShade = Math.max(capShade, window4(t, a - 0.4, a + 0.3, b - 0.2, b + 0.4));
  });
  shade.material.uniforms.uCaption.value = capShade;
  shade.material.uniforms.uBlack.value = Math.max(1 - easeInOutSine(range(t, 0, 0.7)), easeInOutSine(range(t, 29.35, 30)));
  shade.material.uniforms.uFrame.value = Math.round(t * FPS) % 97;

  // Windows rise from below, staggered.
  const rise = (d) => easeOutQuint(range(t, d, d + 1.9));
  const pair = easeInOutCubic(range(t, 20.6, 22.2));
  const termIn = easeInOutCubic(range(t, 20.9, 22.4));
  const gone = easeInOutCubic(range(t, 24.5, 26.2));
  const focus = (who) => {
    // Windows not being talked about step back into the dark a little.
    const f1w = window4(t, 12.4, 13.2, 14.9, 15.5);
    const f23 = window4(t, 15.2, 16.0, 20.4, 21.0);
    const f3 = window4(t, 18.2, 18.9, 20.3, 20.9);
    let b = 1;
    if (who !== 'search') b -= 0.45 * f1w;
    if (who !== 'playlist') b -= 0.45 * f23;
    if (who === 'playlist') b -= 0.25 * f3;
    return b;
  };
  const float = (ph, base) => ({
    ...base,
    y: base.y + Math.sin(t * 0.8 + ph) * 0.035,
    rx: Math.sin(t * 0.55 + ph) * 0.018 - 0.02,
    ry: base.ry + Math.sin(t * 0.45 + ph * 2) * 0.025,
  });
  const entrance = (l, k) => ({ ...l, y: l.y - (1 - k) * 2.4, rx: (1 - k) * 0.5 });
  const sheens = [4.9, 12.5, 15.3, 21.6];

  {
    const k = rise(3.7);
    const l = float(0.0, LAYOUT.playlist);
    // It steps aside while the camera looks past it at the search window.
    const aside = window4(t, 12.0, 13.3, 14.9, 16.0);
    const p = { ...l, x: l.x - aside * 1.4, z: l.z - pair * 2.8 - aside * 1.0, y: l.y + pair * 0.25 };
    const e = entrance(p, k);
    placeScreen(scr.playlist, {
      ...e, rx: e.rx + l.rx, opacity: k * (1 - pair * 0.97) * (1 - gone) * (1 - aside * 0.75),
      bright: focus('playlist') * lerp(1, 0.5, pair), sheen: sheenAt(t, sheens.map((x) => x + 0.15)),
    });
  }
  {
    const k = rise(3.95);
    const l = float(1.7, lerpL(LAYOUT.home, LAYOUT.homePair, pair));
    const e = entrance(l, k);
    placeScreen(scr.home, {
      ...e, rx: e.rx + l.rx, z: e.z - gone * 1.4, opacity: k * (1 - gone), bright: focus('home'),
      sheen: sheenAt(t, sheens),
    });
  }
  {
    const k = rise(4.2);
    const l = float(3.1, LAYOUT.search);
    const p = { ...l, x: l.x + pair * 1.2, z: l.z - pair * 2.4 };
    const e = entrance(p, k);
    placeScreen(scr.search, {
      ...e, rx: e.rx + l.rx, opacity: k * (1 - pair * 0.95) * (1 - gone), bright: focus('search'),
      sheen: sheenAt(t, sheens.map((x) => x + 0.3)),
    });
  }
  {
    const l = float(4.4, LAYOUT.termPair);
    const e = entrance({ ...l, x: l.x + (1 - termIn) * 0.6 }, termIn);
    placeScreen(scr.term, {
      ...e, rx: e.rx + l.rx, z: e.z - gone * 1.4, opacity: termIn * (1 - gone), sheen: sheenAt(t, [21.8]),
    });
  }

  // Feature 1: the services come forward out of the search window.
  const pop = (a, b, c, d) => window4(t, a, b, c, d, easeOutCubic);
  const chips = [
    [ui.yt, -0.75, 0.62, 0.55, 12.9],
    [ui.sc, -0.05, 0.3, 0.75, 13.15],
    [ui.noAds, 0.95, -0.55, 0.6, 13.45],
  ];
  for (const [m, x, y, z, a] of chips) {
    const k = pop(a, a + 0.8, 14.8, 15.3);
    pin(m, scr.search, x, y + (1 - k) * -0.15 + Math.sin(t * 1.1 + x * 3) * 0.02, z * (0.4 + 0.6 * k), 0.06, lerp(0.85, 1, k));
    setText(m, k, 1);
  }

  // Feature 2: a Spotify link becomes a playlist.
  {
    const k = pop(15.8, 16.7, 17.6, 18.05);
    pin(ui.link, scr.playlist, -0.05, -0.42 + (1 - k) * -0.12 + Math.sin(t * 0.9) * 0.015, 0.6, -0.05, lerp(0.9, 1, k));
    setText(ui.link, k, 1);
    const k2 = pop(16.55, 17.25, 17.6, 18.05);
    pin(ui.imported, scr.playlist, 0.35, -0.82 + (1 - k2) * -0.12 + Math.sin(t * 0.9 + 1) * 0.015, 0.75, -0.05, lerp(0.85, 1, k2));
    setText(ui.imported, k2, 1);
  }

  // Feature 3: the Discord presence.
  {
    const k = pop(18.45, 19.35, 20.45, 20.95);
    if (k > 0) {
      const g = ui.presence.userData.source.getContext('2d');
      art.drawPresence(g, S.cover, 70 + (t - 18.4) * 1.0);
      ui.presence.userData.tex.needsUpdate = true;
    }
    pin(ui.presence, scr.playlist, 0.38, -0.28 + (1 - k) * -0.14 + Math.sin(t * 0.8) * 0.02, 0.8, -0.14, lerp(0.88, 1, k));
    setText(ui.presence, k, 1);
  }

  // Feature 4: memory, counted up above each window.
  {
    const k = pop(21.7, 22.5, 24.3, 24.9);
    const count = easeOutCubic(range(t, 21.9, 23.6));
    if (k > 0) {
      art.drawMemory(ui.memWin.userData.source.getContext('2d'), 'Window', 'Tauri + WebView2', 200 * count, 220, false);
      art.drawMemory(ui.memTerm.userData.source.getContext('2d'), 'Terminal mode', 'Windows Terminal, real covers', 30 * count, 220, true);
      ui.memWin.userData.tex.needsUpdate = true;
      ui.memTerm.userData.tex.needsUpdate = true;
    }
    pin(ui.memWin, scr.home, 0, 1.36 + (1 - k) * -0.08, 0.05);
    pin(ui.memTerm, scr.term, 0, 1.36 + (1 - k) * -0.08, 0.05);
    setText(ui.memWin, k, 1);
    setText(ui.memTerm, k, 1);
  }
}

function draw(t) {
  update(t);
  renderer.clear();
  composer.render();
  renderer.clearDepth();
  renderer.render(crisp, camera);
  renderer.clearDepth();
  renderer.render(overlay, ortho);
}

// ------------------------------------------------------------------ run

const ready = build();

if (RENDER) {
  const gl = renderer.getContext();
  const pixels = new Uint8Array(W * H * 4);
  window.__promo = {
    ready,
    fps: FPS,
    duration: DURATION,
    width: W,
    height: H,
    /** Draws frame `i` and returns its RGBA pixels (bottom-up) as base64. */
    frame(i) {
      draw(i / FPS);
      gl.readPixels(0, 0, W, H, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
      let out = '';
      const chunk = 0x8000;
      for (let o = 0; o < pixels.length; o += chunk) out += String.fromCharCode.apply(null, pixels.subarray(o, o + chunk));
      return btoa(out);
    },
    /** Draws a moment and leaves it on the canvas, for still frames. */
    still(t) {
      draw(t);
    },
  };
} else {
  ready.then(() => {
    const at = parseFloat(new URLSearchParams(location.search).get('t'));
    let playing = Number.isNaN(at);
    let time = Number.isNaN(at) ? 0 : at;
    let last = performance.now();
    addEventListener('keydown', (e) => {
      if (e.code === 'Space') playing = !playing;
      if (e.code === 'ArrowRight') time = Math.min(DURATION, time + 1);
      if (e.code === 'ArrowLeft') time = Math.max(0, time - 1);
    });
    const loop = (now) => {
      if (playing) time = (time + (now - last) / 1000) % DURATION;
      last = now;
      draw(time);
      requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
  });
}
