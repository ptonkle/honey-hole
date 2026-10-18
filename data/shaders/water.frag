#version 300 es
precision highp float;
// The lake's surface: each wave facet lit flat (its normal from the screen
// derivatives of its position), a glint of the sun, and an opacity that is
// lower looking down (so what lies just under shows through, tinted) and
// higher at a grazing angle. Fog takes it like the rest of the world.
in vec3 vWorld;
in vec3 vColour;
uniform vec3 uEye;
uniform vec2 uFog;
uniform vec3 uFogColour;
uniform vec3 uZenith;
uniform vec2 uSkyBand;
uniform vec3 uSunDir;
uniform vec3 uSun;
uniform vec3 uSky;
uniform vec3 uGround;
uniform vec2 uHemi;
uniform vec4 uWater;
uniform vec4 uSpec;
out vec4 outColour;
void main() {
  vec3 n = normalize(cross(dFdx(vWorld), dFdy(vWorld)));
  if (n.y < 0.0) n = -n;
  vec3 ray = vWorld - uEye;
  vec3 view = normalize(-ray);
  float sun = max(dot(n, uSunDir), 0.0);
  vec3 fill = mix(uGround, uSky, clamp(n.y * uHemi.x + uHemi.y, 0.0, 1.0));
  vec3 lit = vColour * (fill + uSun * sun);
  lit += uSpec.rgb * pow(max(dot(n, normalize(view + uSunDir)), 0.0), uSpec.w);
  float graze = pow(1.0 - abs(view.y), uWater.z);
  float alpha = mix(uWater.x, uWater.y, graze);
  vec3 sky = mix(uFogColour, uZenith, smoothstep(uSkyBand.x, uSkyBand.y, -view.y));
  float f = smoothstep(uFog.x, uFog.y, length(ray));
  outColour = vec4(mix(lit, sky, f), mix(alpha, 1.0, f));
}
