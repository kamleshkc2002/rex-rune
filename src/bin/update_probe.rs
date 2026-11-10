use clap::Parser;
use rex_linux::update::{CheckConfig, run_cli_check};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    about = "Verify signed Rex Linux release metadata against an endpoint or loopback fixture"
)]
struct Args {
    #[arg(long, default_value = env!("CARGO_PKG_VERSION"))]
    current_version: String,
    #[arg(long, default_value = rex_linux::update::DEFAULT_MANIFEST_URL)]
    manifest_url: String,
    #[arg(long, default_value = rex_linux::update::DEFAULT_SIGNATURE_URL)]
    signature_url: String,
    #[arg(long)]
    public_key_file: Vec<PathBuf>,
    #[arg(long)]
    state_path: Option<PathBuf>,
    #[arg(long)]
    allow_loopback_http: bool,
}

fn main() {
    let args = Args::parse();
    let ephemeral_state = args.state_path.is_none();
    let state_path = args.state_path.unwrap_or_else(|| {
        std::env::temp_dir().join(format!("rex-update-probe-{}.json", std::process::id()))
    });
    let mut config = CheckConfig {
        current_version: args.current_version,
        manifest_url: args.manifest_url,
        signature_url: args.signature_url,
        state_path: state_path.clone(),
        allow_loopback_http: args.allow_loopback_http,
        ..CheckConfig::default()
    };
    if !args.public_key_file.is_empty() {
        let mut keys = Vec::new();
        for path in &args.public_key_file {
            match std::fs::read_to_string(path) {
                Ok(text) => keys.push(text),
                Err(error) => {
                    eprintln!("rex-update-probe: cannot read {}: {error}", path.display());
                    std::process::exit(1);
                }
            }
        }
        config.public_keys = keys;
    }
    let code = run_cli_check(&config);
    if ephemeral_state {
        let _ = std::fs::remove_file(&state_path);
        let _ = std::fs::remove_file(state_path.with_extension("lock"));
    }
    std::process::exit(code);
}
