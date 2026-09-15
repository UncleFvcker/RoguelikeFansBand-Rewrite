// SPDX-License-Identifier: MPL-2.0
import type { Application, Filter, Ticker } from "pixi.js";

// Static artwork for GPU failure; independent of the player's maps, saves, and RNG.
const ruins = String.raw`
                 ___________
             ___/___________\___
          __/___/           \___\__
         /___/    _________    \___\
        /___/   /         \   \___\
       |___|   /   _____   \   |___|
       |___|  |   /     \   |  |___|
    *  |___|  |  |       |  |  |___|  *
    |  |___|  |  |_______|  |  |___|  |
   [_] |___|  | /_________\ |  |___| [_]
       |___|  |/___________\|  |___|
       |___|  /_____________\  |___|
       |___| /_______________\ |___|
      /_____/_________________\_____\
     /_______________________________\
`;

const vertex = `
in vec2 aPosition;
out vec2 vTextureCoord;
uniform vec4 uInputSize;
uniform vec4 uOutputFrame;
uniform vec4 uOutputTexture;
void main() {
  vec2 p = aPosition * uOutputFrame.zw + uOutputFrame.xy;
  gl_Position = vec4(p.x * 2.0 / uOutputTexture.x - 1.0,
    p.y * 2.0 * uOutputTexture.z / uOutputTexture.y - uOutputTexture.z, 0.0, 1.0);
  vTextureCoord = aPosition * uOutputFrame.zw * uInputSize.zw;
}`;

const fragment = `
precision highp float;
in vec2 vTextureCoord;
out vec4 finalColor;
uniform vec4 uInputSize;
uniform vec4 uOutputFrame;
uniform vec4 uMotion;
uniform vec4 uFogA;
uniform vec4 uFogB;
uniform vec4 uFogC;
float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float noise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x),
    mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0)), f.x), f.y);
}
float box(vec3 p, vec3 size) {
  vec3 q = abs(p) - size;
  return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0);
}
// Vertical jambs below the spring line, semicircle above.
float arch(vec2 p) { return length(vec2(p.x, max(p.y - 0.5, 0.0))) - 1.25; }
float scene(vec3 p) {
  float opening = arch(p.xy);
  float wall = max(box(p - vec3(0.0, 0.2, 0.25), vec3(3.7, 2.8, 0.6)), -opening);
  float tunnel = max(box(p - vec3(0.0, -0.2, 4.7), vec3(1.7, 3.2, 4.0)), -opening);
  float ribZ = p.z - 2.6 * clamp(floor(p.z / 2.6 + 0.5), 0.0, 3.0);
  float ribs = max(abs(opening - 0.14) - 0.19, abs(ribZ) - 0.22);
  float pillars = box(vec3(abs(p.x) - 2.25, p.y + 0.15, p.z + 0.35), vec3(0.28, 2.0, 0.45));
  float ground = max(p.y + 1.65, p.z - 0.55);
  float stone = min(min(wall, tunnel), min(ribs, min(pillars, ground)));
  // Closed boxes give descending treads AND risers, with a continuous distance field.
  for (int i = 0; i < 6; i++) {
    float step = float(i);
    stone = min(stone, box(p - vec3(0.0, -2.15 - step * 0.19, 1.025 + step * 0.95), vec3(1.3, 0.5, 0.475)));
  }
  float brackets = box(vec3(abs(p.x) - 1.87, p.y + 0.15, p.z + 0.75), vec3(0.09, 0.27, 0.12));
  return min(stone, brackets);
}
vec3 normalAt(vec3 p) {
  vec2 e = vec2(0.003, -0.003);
  return normalize(e.xyy * scene(p + e.xyy) + e.yyx * scene(p + e.yyx)
    + e.yxy * scene(p + e.yxy) + e.xxx * scene(p + e.xxx));
}
float flicker(float phase) { return 0.9 + 0.07 * sin(uMotion.x * 2.7 + phase) + 0.03 * sin(uMotion.x * 6.1 + phase); }
vec3 torchLight(vec3 p, vec3 n, vec3 lamp, float phase) {
  vec3 delta = lamp - p;
  float distanceSquared = dot(delta, delta);
  return uFogC.rgb * max(dot(n, normalize(delta)), 0.0) * 3.2 * flicker(phase) / (1.0 + distanceSquared);
}
vec3 stoneColor(vec3 p, vec3 n) {
  vec2 plane = p.xy;
  if (abs(n.y) > 0.65) plane = p.xz;
  else if (abs(n.x) > 0.65) plane = p.zy;
  vec2 tile = plane * vec2(1.8, 3.3);
  tile.x += 0.5 * mod(floor(tile.y), 2.0);
  // Radial masonry around the mouth of the arch.
  if (p.y > 0.5 && abs(p.z) < 0.3 && abs(arch(p.xy) - 0.14) < 0.23) {
    tile = vec2(atan(p.y - 0.5, p.x) * 5.0, length(p.xy - vec2(0.0, 0.5)) * 3.0);
  }
  vec2 edge = min(fract(tile), 1.0 - fract(tile));
  float mortar = smoothstep(0.015, 0.055, min(edge.x, edge.y));
  float grain = noise(plane * 36.0);
  vec3 base = mix(vec3(0.24, 0.28, 0.3), vec3(0.48, 0.5, 0.48), hash(floor(tile)));
  base = mix(base, uFogB.rgb * 0.55, 0.22 * noise(plane * 2.1));
  return base * (0.45 + 0.55 * mortar) * (0.84 + 0.16 * grain);
}
void main() {
  vec2 uv = vTextureCoord * uInputSize.xy / uOutputFrame.zw;
  float center = mix(0.53, 0.73, smoothstep(0.7, 1.45, uMotion.y));
  vec2 screen = vec2((uv.x - center) * uMotion.y, 0.53 - uv.y) * 2.0;
  vec3 eye = vec3(0.65, 0.5, -6.7);
  vec3 forward = normalize(vec3(-0.65, -0.65, 8.0));
  vec3 right = normalize(cross(vec3(0.0, 1.0, 0.0), forward));
  vec3 up = cross(forward, right);
  vec3 ray = normalize(forward * 1.85 + right * screen.x + up * screen.y);
  float travel = 0.0;
  bool hit = false;
  for (int i = 0; i < 72; i++) {
    float distance = scene(eye + ray * travel);
    if (distance < 0.003) { hit = true; break; }
    travel += distance * 0.85;
    if (travel > 24.0) break;
  }
  travel = min(travel, 24.0);
  vec3 color = uFogA.rgb * 0.035;
  if (hit) {
    vec3 p = eye + ray * travel;
    vec3 n = normalAt(p);
    float ao = clamp(scene(p + n * 0.18) / 0.18, 0.3, 1.0);
    vec3 moon = mix(uFogA.rgb, vec3(0.7, 0.8, 0.88), 0.5);
    vec3 light = moon * (0.12 + 0.65 * max(dot(n, normalize(vec3(-0.5, 0.9, -0.8))), 0.0));
    light *= exp(-max(p.z, 0.0) * 0.24);
    light += torchLight(p, n, vec3(-1.87, 0.25, -0.85), 0.0);
    light += torchLight(p, n, vec3(1.87, 0.25, -0.85), 2.0);
    light += torchLight(p, n, vec3(0.8, -0.15, 4.8), 4.0) * 0.5;
    color = stoneColor(p, n) * light * ao;
  }
  // Clip ray-to-flame distance at the first stone hit, keeping fire behind walls occluded.
  for (int i = 0; i < 2; i++) {
    float side = float(i) * 2.0 - 1.0;
    vec3 lamp = vec3(side * 1.87 + 0.025 * sin(uMotion.x * 2.3 + side), 0.25, -0.85);
    float along = clamp(dot(lamp - eye, ray), 0.0, travel);
    vec3 offset = eye + ray * along - lamp;
    float flame = length(offset * vec3(1.0, 0.42, 1.0));
    color += uFogC.rgb * (exp(-flame * 38.0) * 2.4 + 0.065 * exp(-flame * 4.0)) * flicker(side);
  }
  float fog = noise(screen * 3.0 + vec2(uMotion.x * 0.026, -uMotion.x * 0.013));
  float haze = (1.0 - exp(-travel * 0.028)) * (0.12 + 0.25 * fog);
  color = mix(color, uFogA.rgb * 0.22, haze);
  color *= 1.0 - 0.3 * smoothstep(0.45, 1.5, length(screen));
  finalColor = vec4(pow(max(color, 0.0), vec3(0.82)), 1.0);

}`;

// Same bounded 3D distance field in both backends; animation changes light and fog only.
const gpuSource = `
struct Globals {
  inputSize: vec4<f32>, inputPixel: vec4<f32>, inputClamp: vec4<f32>,
  outputFrame: vec4<f32>, globalFrame: vec4<f32>, outputTexture: vec4<f32>,
};
struct Atmosphere { uMotion: vec4<f32>, uFogA: vec4<f32>, uFogB: vec4<f32>, uFogC: vec4<f32>, };
@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var uTexture: texture_2d<f32>;
@group(0) @binding(2) var uSampler: sampler;
@group(1) @binding(0) var<uniform> atmosphere: Atmosphere;
struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, };
@vertex fn mainVertex(@location(0) position: vec2<f32>) -> VertexOutput {
  let p = position * g.outputFrame.zw + g.outputFrame.xy;
  return VertexOutput(vec4<f32>(p.x * 2.0 / g.outputTexture.x - 1.0,
    p.y * 2.0 * g.outputTexture.z / g.outputTexture.y - g.outputTexture.z, 0.0, 1.0),
    position * g.outputFrame.zw * g.inputSize.zw);
}
fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let fraction = fract(p);
  let f = fraction * fraction * (3.0 - 2.0 * fraction);
  return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), f.x),
    mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0)), f.x), f.y);
}
fn box(p: vec3<f32>, size: vec3<f32>) -> f32 {
  let q = abs(p) - size;
  return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
}
fn arch(p: vec2<f32>) -> f32 { return length(vec2<f32>(p.x, max(p.y - 0.5, 0.0))) - 1.25; }
fn scene(p: vec3<f32>) -> f32 {
  let opening = arch(p.xy);
  let wall = max(box(p - vec3<f32>(0.0, 0.2, 0.25), vec3<f32>(3.7, 2.8, 0.6)), -opening);
  let tunnel = max(box(p - vec3<f32>(0.0, -0.2, 4.7), vec3<f32>(1.7, 3.2, 4.0)), -opening);
  let ribZ = p.z - 2.6 * clamp(floor(p.z / 2.6 + 0.5), 0.0, 3.0);
  let ribs = max(abs(opening - 0.14) - 0.19, abs(ribZ) - 0.22);
  let pillars = box(vec3<f32>(abs(p.x) - 2.25, p.y + 0.15, p.z + 0.35), vec3<f32>(0.28, 2.0, 0.45));
  let ground = max(p.y + 1.65, p.z - 0.55);
  var stone = min(min(wall, tunnel), min(ribs, min(pillars, ground)));
  for (var i = 0; i < 6; i++) {
    let step = f32(i);
    stone = min(stone, box(p - vec3<f32>(0.0, -2.15 - step * 0.19, 1.025 + step * 0.95), vec3<f32>(1.3, 0.5, 0.475)));
  }
  let brackets = box(vec3<f32>(abs(p.x) - 1.87, p.y + 0.15, p.z + 0.75), vec3<f32>(0.09, 0.27, 0.12));
  return min(stone, brackets);
}
fn normalAt(p: vec3<f32>) -> vec3<f32> {
  let e = vec2<f32>(0.003, -0.003);
  return normalize(e.xyy * scene(p + e.xyy) + e.yyx * scene(p + e.yyx)
    + e.yxy * scene(p + e.yxy) + e.xxx * scene(p + e.xxx));
}
fn flicker(phase: f32) -> f32 {
  return 0.9 + 0.07 * sin(atmosphere.uMotion.x * 2.7 + phase) + 0.03 * sin(atmosphere.uMotion.x * 6.1 + phase);
}
fn torchLight(p: vec3<f32>, n: vec3<f32>, lamp: vec3<f32>, phase: f32) -> vec3<f32> {
  let delta = lamp - p;
  let distanceSquared = dot(delta, delta);
  return atmosphere.uFogC.rgb * max(dot(n, normalize(delta)), 0.0) * 3.2 * flicker(phase) / (1.0 + distanceSquared);
}
fn stoneColor(p: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
  var plane = p.xy;
  if (abs(n.y) > 0.65) { plane = p.xz; }
  else if (abs(n.x) > 0.65) { plane = p.zy; }
  var tile = plane * vec2<f32>(1.8, 3.3);
  tile.x += 0.5 * (floor(tile.y) - 2.0 * floor(floor(tile.y) / 2.0));
  if (p.y > 0.5 && abs(p.z) < 0.3 && abs(arch(p.xy) - 0.14) < 0.23) {
    tile = vec2<f32>(atan2(p.y - 0.5, p.x) * 5.0, length(p.xy - vec2<f32>(0.0, 0.5)) * 3.0);
  }
  let edge = min(fract(tile), vec2<f32>(1.0) - fract(tile));
  let mortar = smoothstep(0.015, 0.055, min(edge.x, edge.y));
  let grain = noise(plane * 36.0);
  var base = mix(vec3<f32>(0.24, 0.28, 0.3), vec3<f32>(0.48, 0.5, 0.48), hash(floor(tile)));
  base = mix(base, atmosphere.uFogB.rgb * 0.55, 0.22 * noise(plane * 2.1));
  return base * (0.45 + 0.55 * mortar) * (0.84 + 0.16 * grain);
}
@fragment fn mainFragment(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
  let uv = coord * g.inputSize.xy / g.outputFrame.zw;
  let center = mix(0.53, 0.73, smoothstep(0.7, 1.45, atmosphere.uMotion.y));
  let screen = vec2<f32>((uv.x - center) * atmosphere.uMotion.y, 0.53 - uv.y) * 2.0;
  let eye = vec3<f32>(0.65, 0.5, -6.7);
  let forward = normalize(vec3<f32>(-0.65, -0.65, 8.0));
  let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), forward));
  let up = cross(forward, right);
  let ray = normalize(forward * 1.85 + right * screen.x + up * screen.y);
  var travel = 0.0;
  var hit = false;
  for (var i = 0; i < 72; i++) {
    let distance = scene(eye + ray * travel);
    if (distance < 0.003) { hit = true; break; }
    travel += distance * 0.85;
    if (travel > 24.0) { break; }
  }
  travel = min(travel, 24.0);
  var color = atmosphere.uFogA.rgb * 0.035;
  if (hit) {
    let p = eye + ray * travel;
    let n = normalAt(p);
    let ao = clamp(scene(p + n * 0.18) / 0.18, 0.3, 1.0);
    let moon = mix(atmosphere.uFogA.rgb, vec3<f32>(0.7, 0.8, 0.88), 0.5);
    var light = moon * (0.12 + 0.65 * max(dot(n, normalize(vec3<f32>(-0.5, 0.9, -0.8))), 0.0));
    light *= exp(-max(p.z, 0.0) * 0.24);
    light += torchLight(p, n, vec3<f32>(-1.87, 0.25, -0.85), 0.0);
    light += torchLight(p, n, vec3<f32>(1.87, 0.25, -0.85), 2.0);
    light += torchLight(p, n, vec3<f32>(0.8, -0.15, 4.8), 4.0) * 0.5;
    color = stoneColor(p, n) * light * ao;
  }
  for (var i = 0; i < 2; i++) {
    let side = f32(i) * 2.0 - 1.0;
    let lamp = vec3<f32>(side * 1.87 + 0.025 * sin(atmosphere.uMotion.x * 2.3 + side), 0.25, -0.85);
    let along = clamp(dot(lamp - eye, ray), 0.0, travel);
    let offset = eye + ray * along - lamp;
    let flame = length(offset * vec3<f32>(1.0, 0.42, 1.0));
    color += atmosphere.uFogC.rgb * (exp(-flame * 38.0) * 2.4 + 0.065 * exp(-flame * 4.0)) * flicker(side);
  }
  let fog = noise(screen * 3.0 + vec2<f32>(atmosphere.uMotion.x * 0.026, -atmosphere.uMotion.x * 0.013));
  let haze = (1.0 - exp(-travel * 0.028)) * (0.12 + 0.25 * fog);
  color = mix(color, atmosphere.uFogA.rgb * 0.22, haze);
  color *= 1.0 - 0.3 * smoothstep(0.45, 1.5, length(screen));
  return vec4<f32>(pow(max(color, vec3<f32>(0.0)), vec3<f32>(0.82)), 1.0);

}`;

export class TitleBackground {
  readonly #root: HTMLElement;
  readonly #host: HTMLElement;
  readonly #motion = window.matchMedia("(prefers-reduced-motion: reduce)");
  readonly #stateObserver = new MutationObserver(() => this.#sync());
  readonly #resizeObserver = new ResizeObserver(() => this.#resize?.());
  readonly #themeObserver = new MutationObserver(() => this.#resize?.());
  #app: Application | undefined;
  #filter: Filter | undefined;
  #resize: (() => void) | undefined;
  #started = false;
  #disposed = false;
  #failed = false;
  #time = 0;

  constructor(root: HTMLElement, host: HTMLElement) {
    this.#root = root;
    this.#host = host;
  }

  install(): void {
    this.#host.querySelector("pre")!.textContent = ruins;
    this.#stateObserver.observe(this.#root, { attributes: true, attributeFilter: ["hidden", "data-view"] });
    this.#themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["style", "class"] });
    this.#resizeObserver.observe(this.#host);
    this.#motion.addEventListener("change", this.#sync);
    document.addEventListener("visibilitychange", this.#sync);
    window.addEventListener("focus", this.#sync);
    window.addEventListener("blur", this.#sync);
    this.#sync();
  }

  get #visible(): boolean {
    return !this.#disposed && !this.#failed && !this.#root.hidden &&
      this.#root.dataset.view === "title" && document.visibilityState === "visible";
  }

  readonly #sync = (): void => {
    if (!this.#started && this.#visible) {
      this.#started = true;
      void this.#initialize();
      return;
    }
    if (!this.#app) return;
    this.#app.stop();
    if (!this.#visible) return;
    this.#resize?.();
    if (!this.#motion.matches && document.hasFocus()) this.#app?.start();
  };

  async #initialize(): Promise<void> {
    let app: Application | undefined;
    try {
      const { Application, Container, Sprite, Texture, Filter, GlProgram, GpuProgram, Color, RendererType } = await import("pixi.js");
      if (this.#disposed) return;
      app = new Application();
      await app.init({ width: 1, height: 1, resolution: 1, antialias: false, autoStart: false,
        sharedTicker: false, backgroundAlpha: 0, preference: "webgl", powerPreference: "low-power" });
      if (this.#disposed) { app.destroy(true, { children: true }); return; }
      this.#app = app;
      if (app.renderer.type === RendererType.CANVAS) throw new Error("Title shader requires a GPU renderer");
      const filter = new Filter({
        glProgram: GlProgram.from({ name: "title-dungeon-entrance", vertex, fragment }),
        gpuProgram: GpuProgram.from({ vertex: { source: gpuSource, entryPoint: "mainVertex" },
          fragment: { source: gpuSource, entryPoint: "mainFragment" } }),
        resources: { atmosphere: {
          uMotion: { value: new Float32Array([0, 1, 0, 0]), type: "vec4<f32>" },
          uFogA: { value: new Float32Array(4), type: "vec4<f32>" },
          uFogB: { value: new Float32Array(4), type: "vec4<f32>" },
          uFogC: { value: new Float32Array(4), type: "vec4<f32>" },
        } },
        resolution: 1,
      });
      this.#filter = filter;
      const scene = new Container();
      const backing = new Sprite({ texture: Texture.WHITE, tint: 0x000000 });
      scene.addChild(backing);
      scene.filters = [filter];
      app.stage.addChild(scene);
      // Own the render callback so a shader error cannot strand the session menu.
      app.ticker.remove(app.render, app);
      app.ticker.maxFPS = 30;
      app.ticker.add(this.#tick);
      app.canvas.addEventListener("webglcontextlost", this.#contextLost);
      if ("gpu" in app.renderer) {
        void app.renderer.gpu.device.lost.then(info => {
          if (this.#app === app && !this.#disposed) this.#fail(new Error(`Title GPU device lost: ${info.message}`));
        });
      }
      this.#host.append(app.canvas);
      const application = app;
      this.#resize = () => {
        if (!this.#visible) return;
        try {
          // Bound fragment work independently of large windows and HiDPI displays.
          const scale = Math.min(0.6, 960 / Math.max(1, this.#host.clientWidth), 600 / Math.max(1, this.#host.clientHeight));
          const width = Math.max(1, Math.round(this.#host.clientWidth * scale));
          const height = Math.max(1, Math.round(this.#host.clientHeight * scale));
          if (application.screen.width !== width || application.screen.height !== height) {
            application.renderer.resize(width, height);
            backing.width = width;
            backing.height = height;
          }
          filter.resources.atmosphere.uniforms.uMotion[1] = width / height;
          const style = getComputedStyle(this.#host);
          for (const [name, property] of [["uFogA", "--title-fog-a"], ["uFogB", "--title-fog-b"], ["uFogC", "--title-fog-c"]] as const) {
            filter.resources.atmosphere.uniforms[name].set(new Color(style.getPropertyValue(property).trim()).toArray());
          }
          this.#paint();
        } catch (error) { this.#fail(error); }
      };
      this.#sync();
    } catch (error) {
      // If init failed before owning the app, only its already-created stage exists.
      if (app && !this.#app) {
        if (app.renderer) app.destroy(true, { children: true });
        else app.stage.destroy({ children: true });
      }
      this.#fail(error);
    }
  }

  readonly #tick = (ticker: Ticker): void => {
    this.#time += Math.min(ticker.deltaMS, 100) / 1000;
    this.#paint();
  };

  #paint(): void {
    if (!this.#visible || !this.#app || !this.#filter) return;
    try {
      this.#filter.resources.atmosphere.uniforms.uMotion[0] = this.#motion.matches ? 0 : this.#time;
      this.#app.render();
      if (this.#host.dataset.rendered !== "true") this.#host.dataset.rendered = "true";
    } catch (error) { this.#fail(error); }
  }

  readonly #contextLost = (): void => { this.#fail(new Error("Title WebGL context lost")); };

  #fail(error: unknown): void {
    if (this.#failed || this.#disposed) return;
    this.#failed = true;
    console.error("Title background unavailable; keeping the static artwork", error);
    this.#release();
  }

  #release(): void {
    const app = this.#app;
    this.#app = undefined;
    this.#resize = undefined;
    delete this.#host.dataset.rendered;
    if (app) {
      app.stop();
      app.canvas.removeEventListener("webglcontextlost", this.#contextLost);
      app.destroy(true, { children: true });
    }
    this.#filter?.destroy();
    this.#filter = undefined;
  }

  dispose(): void {
    this.#disposed = true;
    this.#stateObserver.disconnect();
    this.#resizeObserver.disconnect();
    this.#themeObserver.disconnect();
    this.#motion.removeEventListener("change", this.#sync);
    document.removeEventListener("visibilitychange", this.#sync);
    window.removeEventListener("focus", this.#sync);
    window.removeEventListener("blur", this.#sync);
    this.#release();
  }
}
