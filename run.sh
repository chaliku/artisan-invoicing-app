#!/bin/bash
xhost +local:podman
podman run -it --rm --network=host \
  --security-opt label=disable \
  --device /dev/dri \
  -e DISPLAY=$DISPLAY \
  -e WAYLAND_DISPLAY=$WAYLAND_DISPLAY \
  -e XDG_RUNTIME_DIR=/tmp \
  -e GDK_BACKEND=x11 \
  -e WEBKIT_DISABLE_COMPOSITING_MODE=1 \
  -e WEBKIT_DISABLE_DMABUF_RENDERER=1 \
  -e LIBGL_ALWAYS_SOFTWARE=1 \
  -v /tmp/.X11-unix:/tmp/.X11-unix:ro \
  -v $XDG_RUNTIME_DIR/$WAYLAND_DISPLAY:/tmp/$WAYLAND_DISPLAY \
  -v ~/plumber-app:/workspace:z \
  -w /workspace \
  localhost/plumber-env bash
