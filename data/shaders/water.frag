#version 300 es
precision highp float;
// The lake's surface: each wave facet lit flat (its normal from the screen
// derivatives of its position), a glint of the sun, and an opacity that is
// lower looking down (so what lies just under shows through, tinted) and
// higher at a grazing angle. Fog takes it like the rest of the world.
// Seen from under the water (the eye below 0) it is the surface's
// underside: a bright window looking up (uWindow: where it turns from the
// underside's colour to the zenith's, and the opacity in it and outside),
// with the ripples' sheen playing over it (uCaustic.w), fogged by the murk.
in vec3 vWorld;
in vec3 vColour;
uniform vec3 uEye;
uniform vec2 uFog;
uniform vec3 uFogColour;
uniform vec3 uZenith;
uniform vec2 uSkyBand;
uniform vec3 uDeep;
uniform vec2 uDeepBand;
uniform vec4 uShaft;
uniform vec4 uShaftLook;
uniform vec3 uShaftDir;
uniform vec3 uSheen;
uniform vec4 uRipple1;
uniform vec4 uRipple2;
uniform vec4 uRipple3;
uniform vec4 uCaustic;
uniform vec3 uUnderside;
uniform vec4 uWindow;
uniform vec3 uSunDir;
uniform vec3 uSun;
uniform vec3 uSky;
uniform vec3 uGround;
uniform vec2 uHemi;
uniform vec4 uWater;
uniform vec4 uSpec;
out vec4 outColour;
vec3 murk(vec3 d) {
  vec3 c = mix(uFogColour, uZenith, smoothstep(uSkyBand.x, uSkyBand.y, d.y));
  c = mix(c, uDeep, smoothstep(uDeepBand.x, uDeepBand.y, -d.y));
  if (uShaftLook.x <= 0.0) return c;
  vec3 a = normalize(cross(uShaftDir, vec3(1.0, 0.0, 0.0)));
  vec3 b = cross(uShaftDir, a);
  float turn = atan(dot(d, b), dot(d, a));
  float rays = max(sin(turn * uShaft.x + uShaft.z) + sin(turn * uShaft.y + uShaft.w) - 1.0, 0.0);
  float fade = smoothstep(uShaftLook.z, uShaftLook.w, dot(d, uShaftDir));
  return c + uSheen * pow(rays, uShaftLook.y) * uShaftLook.x * fade;
}
float caustic(vec2 p) {
  float s = sin(dot(p, uRipple1.xy) + uRipple1.z) + sin(dot(p, uRipple2.xy) + uRipple2.z) + sin(dot(p, uRipple3.xy) + uRipple3.z);
  return pow(clamp(1.0 - abs(s) * uCaustic.x, 0.0, 1.0), uCaustic.y);
}
void main() {
  vec3 ray = vWorld - uEye;
  vec3 view = normalize(-ray);
  float f = smoothstep(uFog.x, uFog.y, length(ray));
  if (uEye.y < 0.0) {
    float up = clamp(-view.y, 0.0, 1.0);
    float window = smoothstep(uWindow.x, uWindow.y, up);
    vec3 under = mix(uUnderside, uZenith, window);
    under += uSheen * caustic(vWorld.xz) * uCaustic.w;
    float alpha = mix(uWindow.w, uWindow.z, window);
    // the murk only where the fog has begun: mixing in none of it is the same
    vec3 far = f > 0.0 ? murk(-view) : under;
    outColour = vec4(mix(under, far, f), mix(alpha, 1.0, f));
    return;
  }
  vec3 n = normalize(cross(dFdx(vWorld), dFdy(vWorld)));
  if (n.y < 0.0) n = -n;
  float sun = max(dot(n, uSunDir), 0.0);
  vec3 fill = mix(uGround, uSky, clamp(n.y * uHemi.x + uHemi.y, 0.0, 1.0));
  vec3 lit = vColour * (fill + uSun * sun);
  lit += uSpec.rgb * pow(max(dot(n, normalize(view + uSunDir)), 0.0), uSpec.w);
  float graze = pow(1.0 - abs(view.y), uWater.z);
  float alpha = mix(uWater.x, uWater.y, graze);
  vec3 sky = f > 0.0 ? murk(-view) : lit;
  outColour = vec4(mix(lit, sky, f), mix(alpha, 1.0, f));
}
