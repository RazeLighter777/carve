#!/bin/bash
set -e

if [ -z "$DOWNLOAD_IMAGE" ]; then
  echo "DOWNLOAD_IMAGE environment variable not set"
  exit 1
fi

IMAGE_PATH="/app/image.qcow2"
SHARED_ARG="${SHARED:-0}"

echo "Downloading image from $DOWNLOAD_IMAGE..."
curl -L "$DOWNLOAD_IMAGE" -o "$IMAGE_PATH"

echo "Serving image on port 10809..."
exec qemu-nbd --read-only --shared=$SHARED_ARG --persistent --cache=unsafe --discard=unmap --image-opts driver=qcow2,file.filename=$IMAGE_PATH --port=10809

