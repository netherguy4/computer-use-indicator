# computer-use-indicator

On-screen indicator for agents driving COSMIC through `computer-use-linux`, styled after the Codex software cursor: glowing arrow that glides to each target, click ripple, edge glow in the agent's colour, status pill. Click-through layer-shell surfaces exist only while actions arrive (fade after 8 s idle).

- `computer-use-indicator` (Rust/libcosmic) listens on `$XDG_RUNTIME_DIR/computer-use-indicator.sock` (JSON datagrams). Installed to `~/.local/bin`, started by `~/.config/autostart/io.github.netherguy4.ComputerUseIndicator.desktop`.
- `computer-use-indicator-proxy AGENT -- SERVER mcp` (Python stdlib) relays MCP stdio unchanged and mirrors tool calls. Pointer actions wait 0.45 s so the cursor lands before the real click; `screenshot`/`get_app_state` hide the overlay first so captures stay clean. Wired into Claude Code, Codex, OpenCode and Antigravity configs (backups: `~/.local/state/computer-use-linux/backups/20261002-indicator/`).

Build: `./local-cargo build --release` (Fedora 44 container `localhost/cosmic-comp-build:44`, recipe in `~/Projects/Rust/cosmic-comp/Containerfile.local`; target dir `~/.cache/computer-use-indicator-target`). Auto-blur is disabled: frosted themes would blur every screen behind the overlay. Rendering uses wgpu: tiny-skia copied full 4K buffers every frame (36 % of a core while active vs 12.5 % on wgpu). Idle motion settles 2 s after an action, so a resting overlay stops redrawing; hidden it costs nothing but ~67 MB RSS once wgpu has initialised.
