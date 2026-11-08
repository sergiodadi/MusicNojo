#!/bin/sh
# Refreshes testdata-rw/MyCollection.mucp from the real collection — a
# writable copy, used for testing mc_add's commit (plan/edit/delete).
# Never writes back to the real file; run this to discard whatever
# mc_add left in testdata-rw/ and start from a clean state again.
set -eu

SOURCE="$HOME/Documents/Collectorz/Music Collector/MyCollection.mucp"
DEST="$(dirname "$0")/testdata-rw/MyCollection.mucp"

rm -rf "$DEST"
mkdir -p "$(dirname "$DEST")"
cp -R "$SOURCE" "$DEST"

echo "testdata-rw/MyCollection.mucp rafraîchi (inscriptible) depuis $SOURCE"
