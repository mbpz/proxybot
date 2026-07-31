use proxybot_lib::desktop_contract::render_typescript;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("generated")
        .join("desktop-contract.ts");
    let generated = render_typescript();

    if std::env::args().any(|argument| argument == "--check") {
        let current = std::fs::read_to_string(&output_path).map_err(|error| {
            format!(
                "desktop contract is missing at {}: {error}; run `pnpm contract:generate`",
                output_path.display()
            )
        })?;
        if current != generated {
            return Err(format!(
                "desktop contract is stale at {}; run `pnpm contract:generate`",
                output_path.display()
            )
            .into());
        }
        println!("desktop contract is up to date");
        return Ok(());
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, generated)?;
    println!("generated {}", output_path.display());
    Ok(())
}
