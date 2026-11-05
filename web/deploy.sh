#!/bin/sh
# Builds the app and uploads it to the Space in $KASHIKA_SPACE (e.g. owner/kashika) with $HF_TOKEN.
set -eu
cd "$(dirname "$0")"
: "${KASHIKA_SPACE:?set KASHIKA_SPACE to owner/name}"
: "${HF_TOKEN:?set HF_TOKEN}"
npm run build
cp space/README.md dist/README.md
hf upload "$KASHIKA_SPACE" dist . --repo-type space --delete '*' --commit-message "deploy"
echo "deployed to https://huggingface.co/spaces/$KASHIKA_SPACE"
