"""How the browser checks launch each engine.

Headless Chromium draws WebGL and canvas in software (SwiftShader), which is
slow once a page animates. Under WSL a headless Chromium cannot reach the
GPU (the WebGL context is lost and screenshots fail), but a windowed one
can, through Mesa's d3d12 driver to the Windows GPU. Its window goes on an
invisible Xvfb display started here, so nothing shows on screen. Without
Xvfb or /dev/dxg (CI, a plain Linux machine) Chromium runs headless as
before; BUILDER_HEADLESS=1 forces that.

`webkit-iphone` is WebKit with Playwright's iPhone 15 profile (touch, a
phone viewport). Emulation shows layout and touch events; it cannot show
real multitouch, fullscreen or install, which stay a person's check.
"""

import atexit
import os
import shutil
import subprocess
import time

GPU_ARGS = ["--ignore-gpu-blocklist", "--enable-gpu", "--use-angle=gl", "--ozone-platform=x11"]

RENDERER = """() => { const g = document.createElement('canvas').getContext('webgl2');
  if (!g) return 'no WebGL 2';
  const e = g.getExtension('WEBGL_debug_renderer_info');
  return e ? g.getParameter(e.UNMASKED_RENDERER_WEBGL) : g.getParameter(g.RENDERER); }"""

_display = None


def _xvfb():
    """An invisible X display for this run, started once (":N"), or None.

    The display is named, not picked with -displayfd: WSL mounts
    /tmp/.X11-unix read-only, so Xvfb only listens on its abstract socket,
    and an X server started on a taken number exits at once."""
    global _display
    if _display is None:
        for n in range(99, 140):
            proc = subprocess.Popen(
                ["Xvfb", f":{n}", "-screen", "0", "1920x1080x24", "-nolisten", "tcp"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            time.sleep(0.5)
            if proc.poll() is None:
                atexit.register(proc.kill)
                _display = f":{n}"
                break
    return _display


def gpu_window():
    """Whether Chromium can draw on the GPU in an invisible window here."""
    forced = os.environ.get("BUILDER_HEADLESS") == "1"
    return os.path.exists("/dev/dxg") and shutil.which("Xvfb") is not None and not forced


def launch(p, engine):
    """(browser, context options) for an engine name: `chromium`, `firefox`,
    `webkit` or `webkit-iphone`."""
    launcher, _, device = engine.partition("-")
    profile = dict(p.devices["iPhone 15"]) if device == "iphone" else {}
    if launcher == "chromium" and gpu_window() and _xvfb():
        # the env goes to the browser itself (Playwright's driver started
        # before this); no Wayland, so the window can only go to Xvfb
        env = {k: v for k, v in os.environ.items() if k != "WAYLAND_DISPLAY"}
        env.update(DISPLAY=_xvfb(), GALLIUM_DRIVER="d3d12")
        return p.chromium.launch(channel="chromium", headless=False, args=GPU_ARGS + MUTE_ARGS, env=env), profile
    if launcher == "chromium":
        return p.chromium.launch(args=MUTE_ARGS), profile
    if launcher == "firefox":
        return p.firefox.launch(firefox_user_prefs=MUTE_PREFS), profile
    return getattr(p, launcher).launch(), profile


# The checks make no sound unless BUILDER_MUTE=0: the GPU Chromium runs in a
# real (hidden) window and would otherwise play through the speakers.
_MUTED = os.environ.get("BUILDER_MUTE") != "0"
MUTE_ARGS = ["--mute-audio"] if _MUTED else []
MUTE_PREFS = {"media.volume_scale": "0.0"} if _MUTED else {}


def renderer(page):
    """The WebGL renderer the page got, for the log (GPU or software)."""
    try:
        return page.evaluate(RENDERER)[:80]
    except Exception as e:  # a page that will not make a context
        return f"unknown ({e.__class__.__name__})"
