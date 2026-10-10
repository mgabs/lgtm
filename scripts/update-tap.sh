#!/usr/bin/env bash
set -euo pipefail

TAP_DIR="${1:-../homebrew-tap}"

if [ ! -d "$TAP_DIR" ]; then
    echo "Homebrew tap directory not found at $TAP_DIR"
    exit 1
fi

VERSION=$(grep '^version =' Cargo.toml | head -1 | cut -d '"' -f2)
echo "Resolving hashes for lgtm v${VERSION}..."

DARWIN_SHA=$(curl -sL "https://github.com/mgabs/lgtm/releases/download/v${VERSION}/lgtm-v${VERSION}-darwin-arm64.tar.gz.sha256" | awk '{print $1}' || true)
LINUX_SHA=$(curl -sL "https://github.com/mgabs/lgtm/releases/download/v${VERSION}/lgtm-v${VERSION}-linux-x86_64.tar.gz.sha256" | awk '{print $1}' || true)

if [ -z "$DARWIN_SHA" ] || [ -z "$LINUX_SHA" ]; then
    echo "Warning: could not fetch remote checksums, computing from local release..."
    DARWIN_SHA=$(curl -sL "https://github.com/mgabs/lgtm/releases/download/v${VERSION}/lgtm-v${VERSION}-darwin-arm64.tar.gz" | shasum -a 256 | awk '{print $1}')
    LINUX_SHA=$(curl -sL "https://github.com/mgabs/lgtm/releases/download/v${VERSION}/lgtm-v${VERSION}-linux-x86_64.tar.gz" | shasum -a 256 | awk '{print $1}')
fi

cat << EOF > "$TAP_DIR/Formula/lgtm.rb"
class Lgtm < Formula
  desc "Fast, real-time terminal diff reviewer & AI agent steering TUI built in Rust"
  homepage "https://github.com/mgabs/lgtm"
  version "${VERSION}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/mgabs/lgtm/releases/download/v${VERSION}/lgtm-v${VERSION}-darwin-arm64.tar.gz"
      sha256 "${DARWIN_SHA}"
    else
      url "https://github.com/mgabs/lgtm/releases/download/v${VERSION}/lgtm-v${VERSION}-darwin-arm64.tar.gz"
      sha256 "${DARWIN_SHA}"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/mgabs/lgtm/releases/download/v${VERSION}/lgtm-v${VERSION}-linux-x86_64.tar.gz"
      sha256 "${LINUX_SHA}"
    end
  end

  def install
    bin.install "lgtm"
  end

  test do
    assert_match "Real-Time Terminal Diff Reviewer", shell_output("#{bin}/lgtm --help")
  end
end
EOF

echo "Successfully updated $TAP_DIR/Formula/lgtm.rb for v${VERSION}!"
