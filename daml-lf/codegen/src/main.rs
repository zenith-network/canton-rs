use std::path::PathBuf;

use clap::Parser;
use daml_lf_codegen::{Config, generate};

#[derive(Debug, Parser)]
struct Cli {
    /// Path to .dar file
    dar: PathBuf,

    /// Output directory
    #[arg(short, long)]
    outdir: PathBuf,

    /// Disable sdk types
    #[arg(long)]
    no_sdk_types: bool,

    /// Enable debug output
    #[arg(long)]
    debug: bool,

    /// Enable tracing output
    #[arg(long)]
    trace: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init_tracing(&cli);

    let mut config = Config::default();
    config
        .outdir(&cli.outdir)
        .sdk_types(!cli.no_sdk_types);

    generate(&cli.dar, config)?;

    Ok(())
}

fn init_tracing(cli: &Cli) {
    if cli.trace {
        tracing_subscriber::fmt()
            .with_level(false)
            .with_max_level(tracing::Level::TRACE)
            .init();
    } else if cli.debug {
        tracing_subscriber::fmt()
            .with_level(false)
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }
}
