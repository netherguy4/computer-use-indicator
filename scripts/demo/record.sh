#!/bin/sh
set -e
export XDG_RUNTIME_DIR=/tmp/rt LANG=C WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1
mkdir -m700 -p $XDG_RUNTIME_DIR
cp /usr/bin/sway /tmp/sway; /tmp/sway -c /demo/sway.conf >/demo/sway.log 2>&1 &
for i in $(seq 50); do [ -S $XDG_RUNTIME_DIR/wayland-1 ] && break; sleep 0.1; done
export WAYLAND_DISPLAY=wayland-1 SWAYSOCK=$(ls $XDG_RUNTIME_DIR/sway-ipc.*.sock)
imv -s none -u nearest_neighbour /demo/a.png /demo/b.png /demo/c.png >/demo/imv.log 2>&1 &
export IMV_PID=$!
sleep 1.5
imv-msg $IMV_PID goto 2; sleep 0.3; imv-msg $IMV_PID goto 1; sleep 0.5
/demo/computer-use-indicator >/demo/overlay.log 2>&1 &
sleep 2
wf-recorder -y -r 30 -c libopenh264 -p b=16M -f /demo/raw.mp4 >/demo/rec.log 2>&1 &
REC=$!
sleep 1
python3 /demo/drive.py
kill -INT $REC; wait $REC || true
