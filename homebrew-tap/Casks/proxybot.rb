cask "proxybot" do
  arch arm: "arm64", intel: "x64"

  version "1.2.0"
  sha256 arm:   "bedff7e059d1bebc04316b117639e3bbfbaefaa5dc9f30fecbc53c4044d0a4b2",
         intel: "TODO: compute after first release - run on Intel Mac"

  url "https://github.com/mbpz/proxybot/releases/download/v#{version}/ProxyBot-#{version}-mac-#{arch}.zip"
  name "ProxyBot"
  desc "HTTPS MITM proxy tool for developers — capture and decrypt mobile traffic"
  homepage "https://github.com/mbpz/proxybot"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates true
  depends_on macos: ">= :big_sur"

  artifact "ProxyBot.app", target: "/Applications/ProxyBot.app"

  uninstall quit: "com.proxybot.app",
            delete: "/Applications/ProxyBot.app"

  zap trash: [
    "~/.proxybot",
    "~/Library/Application Support/com.proxybot.app",
    "~/Library/Preferences/com.proxybot.app.plist",
    "~/Library/Saved Application State/com.proxybot.app.savedState",
  ]
end