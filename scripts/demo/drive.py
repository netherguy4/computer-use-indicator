import json, os, socket, subprocess, time
s = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
P = os.environ["XDG_RUNTIME_DIR"] + "/computer-use-indicator.sock"
def ev(agent, tool, label, **kw):
    s.sendto(json.dumps({"agent": agent, "tool": tool, "label": label, **kw}).encode(), P)
def bg(name):
    subprocess.run(["imv-msg", os.environ["IMV_PID"], "goto", str("abc".index(name) + 1)])
T0 = time.monotonic()
def at(t): time.sleep(max(0, T0 + t - time.monotonic()))
at(0.8);  ev("Claude", "screenshot", "looking at the screen")
at(2.0);  ev("Claude", "click", "click", x=760, y=214)
at(3.3);  ev("Claude", "press_key", "key press", keys=["Ctrl", "F"])
at(4.6);  ev("Claude", "type_text", "typing", text="night")
at(5.1);  bg("b")
at(6.6);  ev("Claude", "click", "click", x=1514, y=320)
at(7.05); bg("c")
at(8.4);  ev("Claude", "scroll", "scroll", x=1100, y=560)
at(10.0); ev("Codex", "click", "click", x=420, y=407)
at(11.4); ev("Codex", "press_key", "key press", keys=["Enter"])
at(12.6); ev("Codex", "type_text", "typing", text="dark mode")
at(15.6)
