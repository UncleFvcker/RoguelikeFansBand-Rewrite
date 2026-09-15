// SPDX-License-Identifier: MPL-2.0
import { Filter, GlProgram, GpuProgram } from "pixi.js";

// One shared filter per map. The existing glyph texture supplies alpha; only phase changes.
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
in vec2 vTextureCoord;
out vec4 finalColor;
uniform sampler2D uTexture;
uniform vec4 uInputSize;
uniform vec4 uInputPixel;
uniform vec4 uInputClamp;
uniform vec4 uOutputFrame;
uniform float uPhase;
float alphaAt(vec2 uv) { return texture(uTexture, clamp(uv, uInputClamp.xy, uInputClamp.zw)).a; }
void main() {
  vec2 uv = vTextureCoord;
  vec2 local = uv * uInputSize.xy / uOutputFrame.zw;
  float hue = (local.x + local.y) * 0.75 - uPhase;
  vec3 color = 0.2 + 0.8 * clamp(abs(fract(hue + vec3(0.0, 2.0/3.0, 1.0/3.0)) * 6.0 - 3.0) - 1.0, 0.0, 1.0);
  float alpha = alphaAt(uv);
  float edge = max(max(alphaAt(uv + vec2(uInputPixel.z, 0.0)), alphaAt(uv - vec2(uInputPixel.z, 0.0))),
    max(alphaAt(uv + vec2(0.0, uInputPixel.w)), alphaAt(uv - vec2(0.0, uInputPixel.w))));
  finalColor = vec4(color * alpha, max(alpha, edge));
}`;

// Pixi can select WebGPU when WebGL is unavailable; both programs use identical math.
const gpuSource = `
struct Globals {
  inputSize: vec4<f32>, inputPixel: vec4<f32>, inputClamp: vec4<f32>,
  outputFrame: vec4<f32>, globalFrame: vec4<f32>, outputTexture: vec4<f32>,
};
struct Rainbow { uPhase: f32, };
@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var uTexture: texture_2d<f32>;
@group(0) @binding(2) var uSampler: sampler;
@group(1) @binding(0) var<uniform> rainbow: Rainbow;
struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
};
@vertex fn mainVertex(@location(0) position: vec2<f32>) -> VertexOutput {
  let p = position * g.outputFrame.zw + g.outputFrame.xy;
  return VertexOutput(vec4<f32>(p.x * 2.0 / g.outputTexture.x - 1.0,
    p.y * 2.0 * g.outputTexture.z / g.outputTexture.y - g.outputTexture.z, 0.0, 1.0),
    position * g.outputFrame.zw * g.inputSize.zw);
}
fn alphaAt(uv: vec2<f32>) -> f32 {
  return textureSample(uTexture, uSampler, clamp(uv, g.inputClamp.xy, g.inputClamp.zw)).a;
}
@fragment fn mainFragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
  let local = uv * g.inputSize.xy / g.outputFrame.zw;
  let hue = (local.x + local.y) * 0.75 - rainbow.uPhase;
  let color = vec3<f32>(0.2) + 0.8 * clamp(abs(fract(vec3<f32>(hue) + vec3<f32>(0.0, 2.0/3.0, 1.0/3.0)) * 6.0 - vec3<f32>(3.0)) - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0));
  let alpha = alphaAt(uv);
  let edge = max(max(alphaAt(uv + vec2<f32>(g.inputPixel.z, 0.0)), alphaAt(uv - vec2<f32>(g.inputPixel.z, 0.0))),
    max(alphaAt(uv + vec2<f32>(0.0, g.inputPixel.w)), alphaAt(uv - vec2<f32>(0.0, g.inputPixel.w))));
  return vec4<f32>(color * alpha, max(alpha, edge));
}`;

export function createUniqueRainbowFilter(): Filter {
  return new Filter({
    glProgram: GlProgram.from({ name: "unique-rainbow", vertex, fragment }),
    gpuProgram: GpuProgram.from({
      vertex: { source: gpuSource, entryPoint: "mainVertex" },
      fragment: { source: gpuSource, entryPoint: "mainFragment" },
    }),
    resources: { rainbow: { uPhase: { value: 0, type: "f32" } } },
    padding: 1,
    resolution: "inherit",
  });
}
