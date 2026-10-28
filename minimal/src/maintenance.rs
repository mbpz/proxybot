//! Self uninstall and self-update commands.
//!
//! Uninstall removes the installed binary, installed license/share files,
//! optional user data, and delegates Gateway/launchd cleanup when root. Update
//! downloads the matching release archive, verifies its checksum, and replaces
//! the current executable.

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(clap::Args)]
pub struct UninstallArgs {
    /// Also remove the user data directory (database, CA, rules).
    #[arg(long)]
    pub purge: bool,
    /// Do not prompt before --purge.
    #[arg(long)]
    pub yes: bool,
}

#[derive(clap::Args)]
pub struct UpdateArgs {
    /// Version to install, without the v prefix (default: latest release).
    #[arg(long)]
    pub version: Option<String>,
}

fn root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "0")
        .unwrap_or(false)
}

fn user_data_dir() -> PathBuf {
    if let Ok(user) = std::env::var("SUDO_USER") {
        let home = PathBuf::from("/Users").join(user);
        if home.is_dir() {
            return home.join("Library/Application Support/ProxyBot");
        }
    }
    crate::default_data_dir()
}

fn port_open(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().expect("addr"),
        std::time::Duration::from_millis(400),
    )
    .is_ok()
}

fn gateway_state_running() -> bool {
    let path = PathBuf::from("/tmp/proxybot-gateway/state.json");
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("state").and_then(|s| s.as_str()).map(str::to_string))
        .map(|state| state == "running")
        .unwrap_or(false)
}

pub fn uninstall(args: UninstallArgs) -> Result<()> {
    let exe = std::env::current_exe().context("resolve current executable")?;
    let prefix = exe.parent().context("executable has no parent")?.to_path_buf();
    let share = prefix.join("share/proxybot");

    if crate::launchd::service_installed() {
        if !root() {
            bail!(
                "Gateway launchd service is installed; run: sudo {} gateway uninstall-service",
                exe.display()
            );
        }
        crate::launchd::uninstall()?;
    }
    if gateway_state_running() {
        bail!("Gateway is running; stop it first with: sudo {} gateway stop", exe.display());
    }
    if port_open(8088) {
        bail!("ProxyBot is still listening on 127.0.0.1:8088; stop it (Ctrl+C) before uninstalling");
    }

    let data = user_data_dir();
    if args.purge {
        if !args.yes {
            print!(
                "Remove user data including database and CA at\n  {}\n? [y/N] ",
                data.display()
            );
            io::stdout().flush().ok();
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                bail!("aborted; binary and installed files were not removed");
            }
        }
        if data.exists() {
            fs::remove_dir_all(&data)
                .with_context(|| format!("remove data directory {}", data.display()))?;
            println!("removed user data {}", data.display());
        }
    } else {
        println!("kept user data {}", data.display());
    }

    if share.exists() {
        fs::remove_dir_all(&share)?;
        println!("removed {}", share.display());
    }
    fs::remove_file(&exe).with_context(|| format!("remove {}", exe.display()))?;
    println!("removed {}", exe.display());
    println!();
    println!("CA trust is not removed automatically. Remove 'ProxyBot Local CA' from");
    println!("Keychain Access, or delete the iOS profile/trusted root separately.");
    Ok(())
}

fn target_asset() -> Result<&'static str> {
    match Command::new("uname").arg("-m").output()?.stdout.as_slice() {
        b"arm64\n" => Ok("aarch64-apple-darwin"),
        b"x86_64\n" => Ok("x86_64-apple-darwin"),
        other => bail!("unsupported architecture: {}", String::from_utf8_lossy(other).trim()),
    }
}

pub fn update(args: UpdateArgs) -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!("self-update currently supports macOS only");
    }
    let exe = std::env::current_exe().context("resolve current executable")?;
    let prefix = exe.parent().context("executable has no parent")?.to_path_buf();
    let target = target_asset()?;
    let version = args.version.as_deref().map(|value| {
        value.trim_start_matches("minimal-v").trim_start_matches('v').to_string()
    });
    let base = match &version {
        Some(version) => format!("https://github.com/mbpz/proxybot/releases/download/minimal-v{version}"),
        None => "https://github.com/mbpz/proxybot/releases/latest/download".to_string(),
    };
    let asset = format!("proxybot-min-{target}.tar.gz");
    let tmp = std::env::temp_dir().join(format!("proxybot-update-{}", std::process::id()));
    fs::create_dir_all(&tmp)?;

    println!("downloading {base}/{asset}");
    let status = Command::new("curl")
        .args(["--http1.1", "--fail", "--location", "--silent", "--show-error", "--retry", "3"])
        .arg(format!("{base}/{asset}"))
        .arg(tmp.join(&asset))
        .status()?;
    if !status.success() {
        bail!("download failed");
    }
    let status = Command::new("curl")
        .args(["--http1.1", "--fail", "--location", "--silent", "--show-error", "--retry", "3"])
        .arg(format!("{base}/{asset}.sha256"))
        .arg(tmp.join(format!("{asset}.sha256")))
        .status()?;
    if !status.success() {
        bail!("checksum download failed");
    }
    let sha_text = fs::read_to_string(tmp.join(format!("{asset}.sha256")))?;
    let expected = sha_text.split_whitespace().next().unwrap_or_default();
    let actual = Command::new("shasum")
        .args(["-a", "256"])
        .arg(tmp.join(&asset))
        .output()?;
    let actual = String::from_utf8_lossy(&actual.stdout);
    let actual = actual.split_whitespace().next().unwrap_or_default();
    if expected.is_empty() || expected != actual {
        bail!("checksum mismatch for {asset}");
    }
    println!("checksum verified");

    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(tmp.join(&asset))
        .arg("-C")
        .arg(&tmp)
        .status()?;
    if !status.success() {
        bail!("extract failed");
    }
    let source = tmp.join("proxybot");
    if !source.is_file() {
        bail!("archive does not contain proxybot");
    }
    let update_tmp = exe.with_extension("update-tmp");
    fs::copy(&source, &update_tmp)?;
    fs::set_permissions(&update_tmp, fs::Permissions::from_mode(0o755))?;
    fs::rename(&update_tmp, &exe).with_context(|| format!("replace {}", exe.display()))?;

    let share = prefix.join("share/proxybot");
    if share.is_dir() {
        for name in ["LICENSE", "THIRD_PARTY.md", "uninstall.sh"] {
            let from = tmp.join(name);
            if from.is_file() {
                let _ = fs::copy(from, share.join(name));
            }
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    let version = Command::new(&exe).arg("--version").output()?;
    println!("updated: {}", String::from_utf8_lossy(&version.stdout).trim());
    if crate::launchd::service_installed() {
        println!("Gateway launchd service is installed; restart it explicitly if needed.");
    }
    Ok(())
}
