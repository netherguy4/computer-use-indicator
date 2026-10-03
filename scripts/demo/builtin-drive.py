import json, os, socket, subprocess, time
s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
P = os.environ["XDG_RUNTIME_DIR"] + "/computer-use-linux-indicator.sock"
C = json.load(open("/demo/coords.json"))
FRAMES = os.environ["FRAMES"].split()
def ev(agent, tool, at=None, **kw):
    if at:
        kw.update(x=C[at][0], y=C[at][1])
    s.sendto(json.dumps({"agent": agent, "tool": tool, **kw}).encode(), P)
def bg(name):
    subprocess.run(["imv-msg", os.environ["IMV_PID"], "goto", str(FRAMES.index(f"/demo/s_{name}.png") + 1)])
T0 = time.monotonic()
def at(t): time.sleep(max(0, T0 + t - time.monotonic()))
LAND = 0.36  # the overlay glide (GLIDE = 350 ms) plus a frame

at(0.8);  ev("Claude", "screenshot")
at(2.2);  ev("Claude", "click", "search")
at(2.2 + LAND); bg("focus")
at(3.4);  ev("Claude", "type_text", text="night")
for n in range(1, 6):
    at(3.5 + n * 0.13); bg(f"type{n}")
at(5.0);  ev("Claude", "click", "night_toggle")
at(5.0 + LAND); bg("night")
at(6.5);  ev("Claude", "screenshot")
at(8.3);  ev("Codex", "click", "appearance")
at(8.3 + LAND); bg("appearance")
at(9.7);  ev("Codex", "click", "dark_toggle")
at(9.7 + LAND); bg("dark")
at(11.2); ev("Codex", "drag", "slider_from")
at(11.2 + LAND); ev("Codex", "drag", "slider_to")
for k in range(1, 6):
    at(11.2 + LAND + k * 0.065); bg(f"scale{k}")
at(13.0); ev("Codex", "press_key", keys=["ctrl", "s"])
at(13.15); bg("saved")
at(14.6); ev("Codex", "screenshot")
at(16.8)
