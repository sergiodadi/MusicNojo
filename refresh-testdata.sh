#!/bin/sh
# Refreshes testdata/MyCollection.mucp from the real collection — a
# read-only copy (chmod 444/555), used by mc_export's test and for
# manual read-only testing. Never writes back to the real file.
set -eu

SOURCE="$HOME/Documents/Collectorz/Music Collector/MyCollection.mucp"
DEST="$(dirname "$0")/testdata/MyCollection.mucp"

rm -rf "$DEST"
mkdir -p "$(dirname "$DEST")"
cp -R "$SOURCE" "$DEST"
find "$DEST" -type d -exec chmod 555 {} +
find "$DEST" -type f -exec chmod 444 {} +

echo "testdata/MyCollection.mucp rafraîchi (lecture seule) depuis $SOURCE"
