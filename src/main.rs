use clap::Parser;
mod app;

fn main() -> anyhow::Result<()> {
    let options = rex_linux::connection::Options::parse();
    if options.check_update {
        let code = rex_linux::update::run_cli_check(&rex_linux::update::CheckConfig::default());
        std::process::exit(code);
    }
    app::run(options)
}
