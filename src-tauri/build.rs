// Build script for the ProxyBot Tauri app.
//
// Responsibilities:
//   * Run tauri-build (icon embedding, capabilities generation, etc.).
//   * Provide a manual override path for the frida-core devkit via the
//     `FRIDA_DEVKIT_DIR` environment variable.
//
// The `frida` crate is configured with the `auto-download` feature, which
// causes its own build script (`frida-sys`) to fetch the matching
// frida-core devkit (headers + static libs) from GitHub on first build
// and cache it. We deliberately do NOT download the devkit here to avoid
// racing with `frida-sys`.
//
// To use a pre-downloaded devkit:
//   1. Set `FRIDA_DEVKIT_DIR=/path/to/devkit/root` before `cargo build`.
//      The devkit root must contain `lib/libfrida-core.{a,dylib,lib}`
//      and `include/frida-core.h`.
//   2. Disable the auto-download feature by overriding the dep in
//      `.cargo/config.toml` or by using `--no-default-features`.
//
// If the devkit download fails (network, rate limit), the frida-sys
// build script fails loudly — that is intentional, because without the
// devkit the `frida` crate cannot generate bindings.

fn main() {
    tauri_build::build();

    // Optional override: if the user has pre-downloaded the devkit, expose
    // its lib path to rustc so the linker can find frida-core / frida-gum
    // even if auto-download is disabled. This is a no-op when
    // FRIDA_DEVKIT_DIR is unset.
    if let Ok(custom_dir) = std::env::var("FRIDA_DEVKIT_DIR") {
        let dir = std::path::PathBuf::from(custom_dir);
        println!("cargo:rerun-if-env-changed=FRIDA_DEVKIT_DIR");
        println!(
            "cargo:rustc-link-search=native={}",
            dir.join("lib").display()
        );
        println!("cargo:rustc-link-lib=static=frida-core");
        println!("cargo:rustc-link-lib=static=frida-gum");
    }
}
