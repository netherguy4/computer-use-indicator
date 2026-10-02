# Demo recording

Renders a mock desktop and records the overlay in a headless sway session, so no real windows or data appear.

```sh
cargo build --release
cp target/release/computer-use-indicator scripts/demo/
cd scripts/demo
podman build -t cui-demo .
podman run --rm -v "$PWD:/demo:Z" cui-demo python3 /demo/scene.py   # a.png b.png c.png
podman run --rm -v "$PWD:/demo:Z" cui-demo /demo/record.sh          # raw.mp4
podman run --rm -v "$PWD:/demo:Z" cui-demo ffmpeg -ss 0.8 -i /demo/raw.mp4 \
  -vf "fps=24,scale=1280:-1:flags=lanczos" -c:v libwebp_anim -q:v 72 -loop 0 /demo/demo.webp
```

`drive.py` is the script: overlay datagrams plus backdrop switches through `imv-msg`.
