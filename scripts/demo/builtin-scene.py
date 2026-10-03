# Renders the demo states of a mock Settings app and writes the coordinates of
# every control to coords.json, so the overlay script clicks exactly on them.
import json
import subprocess

W, H = 1920, 1080
WX, WY, WW, WH = 320, 150, 1280, 780
SIDEBAR = ["Display", "Sound", "Network", "Appearance", "Power", "Privacy"]
DISPLAY_ROWS = [
    ("Night light", "Warmer colours after sunset"),
    ("Do not disturb", "Silence notifications"),
    ("Auto brightness", "Adapt to ambient light"),
    ("Large text", "Scale interface text by 125%"),
]
FONT, FONTB = "Noto-Sans", "Noto-Sans-SemiBold"
ACCENT = "#4c9dff"
SEARCH = (WX + 340, WY + 40, WX + WW - 40, WY + 88)
TRACK = (WX + 700, WX + 1160)  # interface scale slider, 75 %..150 %


def sidebar_center(i):
    return (WX + 150, WY + 101 + i * 52)


def row_top(i):
    return WY + 130 + i * 96


def toggle_center(i):
    return (WX + WW - 86, row_top(i) + 40)


def slider_x(percent):
    return round(TRACK[0] + (percent - 75) / 75 * (TRACK[1] - TRACK[0]))


SLIDER_Y = row_top(2) + 40

coords = {
    "search": ((SEARCH[0] + SEARCH[2]) // 2 - 300, (SEARCH[1] + SEARCH[3]) // 2),
    "night_toggle": toggle_center(0),
    "appearance": sidebar_center(3),
    "dark_toggle": toggle_center(0),
    "slider_from": (slider_x(100), SLIDER_Y),
    "slider_to": (slider_x(125), SLIDER_Y),
}


def palette(dark):
    if dark:
        return dict(window="#121419", sidebar="#171a20", row="#1b1f27", field="#1e232c",
                    text="#eef1f6", dim="#8790a3", select="#262c38", border="#2c3340")
    return dict(window="#1c1f26", sidebar="#22262f", row="#232833", field="#2a2f3a",
                text="#e8ebf2", dim="#8a93a6", select="#2f3542", border="#3d4452")


def toggle(cmd, center, on, p):
    x, y = center[0] - 24, center[1] - 13
    bg = ACCENT if on else "#3a3f4b"
    knob = x + 35 if on else x + 13
    cmd += ["-fill", bg, "-draw", f"roundrectangle {x},{y} {x + 48},{y + 26} 13,13",
            "-fill", "#f4f6fa", "-draw", f"circle {knob},{y + 13} {knob},{y + 3}"]


def text(cmd, font, size, color, x, y, value):
    cmd += ["-font", font, "-pointsize", str(size), "-fill", color, "-draw", f"text {x},{y} '{value}'"]


def render(name, page="display", query="", focused=False, night=False, dark=False, scale=100, toast=False):
    p = palette(dark)
    cmd = ["magick", "-size", f"{W}x{H}", "gradient:#0e1626-#1c1433"]
    # window shadow
    cmd += ["(", "-size", f"{W}x{H}", "xc:none", "-fill", "#00000090",
            "-draw", f"roundrectangle {WX},{WY + 18} {WX + WW},{WY + WH + 18} 16,16", "-blur", "0x26", ")", "-composite"]
    cmd += ["-fill", p["window"], "-draw", f"roundrectangle {WX},{WY} {WX + WW},{WY + WH} 16,16",
            "-fill", p["sidebar"], "-draw", f"roundrectangle {WX},{WY} {WX + 300},{WY + WH} 16,16",
            "-draw", f"rectangle {WX + 284},{WY} {WX + 300},{WY + WH}"]
    for i, c in enumerate(["#ff5f57", "#febc2e", "#28c840"]):
        cmd += ["-fill", c, "-draw", f"circle {WX + 28 + i * 22},{WY + 28} {WX + 34 + i * 22},{WY + 28}"]
    text(cmd, FONTB, 15, p["text"], WX + 104, WY + 33, "Settings")
    selected = "Appearance" if page == "appearance" else "Display"
    for i, item in enumerate(SIDEBAR):
        cx, cy = sidebar_center(i)
        if item == selected:
            cmd += ["-fill", p["select"], "-draw", f"roundrectangle {WX + 16},{cy - 21} {WX + 284},{cy + 21} 10,10"]
        cmd += ["-fill", ACCENT if item == selected else "#6b7385",
                "-draw", f"roundrectangle {WX + 34},{cy - 11} {WX + 56},{cy + 11} 6,6"]
        text(cmd, FONTB if item == selected else FONT, 17, p["text"] if item == selected else "#9aa3b5", WX + 70, cy + 6, item)

    if page == "display":
        stroke = ACCENT if focused else p["border"]
        cmd += ["-fill", p["field"], "-stroke", stroke, "-strokewidth", "2" if focused else "1",
                "-draw", f"roundrectangle {SEARCH[0]},{SEARCH[1]} {SEARCH[2]},{SEARCH[3]} 12,12", "-stroke", "none"]
        cmd += ["-fill", "none", "-stroke", "#8a93a6", "-strokewidth", "2",
                "-draw", f"circle {SEARCH[0] + 24},{SEARCH[1] + 22} {SEARCH[0] + 30},{SEARCH[1] + 22}",
                "-draw", f"line {SEARCH[0] + 29},{SEARCH[1] + 27} {SEARCH[0] + 34},{SEARCH[1] + 32}", "-stroke", "none"]
        if query:
            text(cmd, FONT, 17, p["text"], SEARCH[0] + 48, SEARCH[1] + 31, query)
        elif not focused:
            text(cmd, FONT, 17, "#6f7789", SEARCH[0] + 48, SEARCH[1] + 31, "Search settings")
        if focused:
            caret_x = SEARCH[0] + 48 + int(len(query) * 9.6)
            cmd += ["-fill", ACCENT, "-draw", f"rectangle {caret_x},{SEARCH[1] + 14} {caret_x + 1},{SEARCH[1] + 36}"]
        rows = [r for r in DISPLAY_ROWS if r[0].lower().startswith(query.lower())]
        for i, (title, sub) in enumerate(rows):
            y = row_top(i)
            cmd += ["-fill", p["row"], "-draw", f"roundrectangle {WX + 340},{y} {WX + WW - 40},{y + 80} 12,12"]
            text(cmd, FONTB, 18, p["text"], WX + 370, y + 35, title)
            text(cmd, FONT, 15, p["dim"], WX + 370, y + 60, sub)
            toggle(cmd, toggle_center(i), night and title == "Night light", p)
    else:
        text(cmd, FONTB, 24, p["text"], WX + 340, WY + 74, "Appearance")
        rows = [("Dark mode", "Use dark colours everywhere"), ("Accent colour", "Highlights and selections"),
                ("Interface scale", "Size of text and controls")]
        for i, (title, sub) in enumerate(rows):
            y = row_top(i)
            cmd += ["-fill", p["row"], "-draw", f"roundrectangle {WX + 340},{y} {WX + WW - 40},{y + 80} 12,12"]
            text(cmd, FONTB, 18, p["text"], WX + 370, y + 35, title)
            text(cmd, FONT, 15, p["dim"], WX + 370, y + 60, sub)
        toggle(cmd, toggle_center(0), dark, p)
        for k, c in enumerate(["#4c9dff", "#8bd47e", "#e08a67", "#c08af0", "#f2c14e"]):
            cx = WX + WW - 230 + k * 36
            cy = row_top(1) + 40
            if k == 0:
                cmd += ["-fill", "none", "-stroke", "#f4f6fa", "-strokewidth", "2", "-draw", f"circle {cx},{cy} {cx + 15},{cy}", "-stroke", "none"]
            cmd += ["-fill", c, "-draw", f"circle {cx},{cy} {cx + 11},{cy}"]
        knob = slider_x(scale)
        cmd += ["-fill", "#3a3f4b", "-draw", f"roundrectangle {TRACK[0]},{SLIDER_Y - 3} {TRACK[1]},{SLIDER_Y + 3} 3,3",
                "-fill", ACCENT, "-draw", f"roundrectangle {TRACK[0]},{SLIDER_Y - 3} {knob},{SLIDER_Y + 3} 3,3",
                "-fill", "#f4f6fa", "-draw", f"circle {knob},{SLIDER_Y} {knob + 11},{SLIDER_Y}"]
        text(cmd, FONTB, 17, p["text"], TRACK[1] + 30, SLIDER_Y + 6, f"{scale}%")
    if toast:
        tw, th = 300, 52
        tx, ty = WX + (WW - tw) // 2, WY + WH - 90
        cmd += ["-fill", "#2a303c", "-stroke", "#3d4452", "-strokewidth", "1",
                "-draw", f"roundrectangle {tx},{ty} {tx + tw},{ty + th} 26,26", "-stroke", "none",
                "-fill", "#3ccf7a", "-draw", f"circle {tx + 32},{ty + 26} {tx + 43},{ty + 26}",
                "-fill", "none", "-stroke", "#0f1a12", "-strokewidth", "3",
                "-draw", f"polyline {tx + 26},{ty + 26} {tx + 31},{ty + 31} {tx + 39},{ty + 21}", "-stroke", "none"]
        text(cmd, FONTB, 17, "#eef1f6", tx + 60, ty + 32, "Settings saved")
    if night:
        cmd += ["(", "-size", f"{W}x{H}", "xc:#ff9a3c", "-alpha", "set", "-channel", "A", "-evaluate", "set", "8%", "+channel", ")", "-composite"]
    cmd += [f"/demo/s_{name}.png"]
    subprocess.run(cmd, check=True)


render("start")
render("focus", focused=True)
for n in range(1, 6):
    render(f"type{n}", focused=True, query="night"[:n])
render("night", query="night", focused=True, night=True)
render("appearance", page="appearance", night=True)
render("dark", page="appearance", night=True, dark=True)
for k, pct in enumerate([105, 110, 115, 120, 125], 1):
    render(f"scale{k}", page="appearance", night=True, dark=True, scale=pct)
render("saved", page="appearance", night=True, dark=True, scale=125, toast=True)
json.dump(coords, open("/demo/coords.json", "w"))
