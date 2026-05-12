class ProxybotTui < Formula
  desc "ProxyBot TUI — HTTPS MITM proxy for developers"
  homepage "https://github.com/mbpz/proxybot"
  license "MIT"
  version "1.2.0"

  on_macos do
    if Hardware::CPU.arm64?
      url "https://github.com/mbpz/proxybot/releases/download/v1.2.0/proxybot-tui-1.2.0-mac-arm64"
      sha256 "TODO: run 'brew install --cask --dry-run' to get actual sha256 after first release"
    else
      url "https://github.com/mbpz/proxybot/releases/download/v1.2.0/proxybot-tui-1.2.0-mac-x64"
      sha256 "TODO: run 'brew install --cask --dry-run' to get actual sha256 after first release"
    end
  end

  def install
    bin.install "proxybot-tui"
  end
end
