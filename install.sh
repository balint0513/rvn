#!/usr/bin/env bash
set -e

echo "Downloading the latest release of rvn..."

# GitHub automatically redirects 'latest' to the newest uploaded binary
DOWNLOAD_URL="https://github.com/balint0513/rvn/releases/latest/download/rvn"

# Download the file to a temporary directory
curl -sSL -o /tmp/rvn "$DOWNLOAD_URL"

echo "Installing to /usr/local/bin (this requires sudo privileges)..."

# Move it to the universal path and make it executable
sudo mv /tmp/rvn /usr/local/bin/rvn
sudo chmod +x /usr/local/bin/rvn

echo ""
echo "Success! rvn is installed."
