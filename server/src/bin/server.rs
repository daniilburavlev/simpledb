use clap::Parser;
use std::path::PathBuf;

fn default_path() -> String {
    "data".to_string()
}

#[derive(Parser)]
struct Cli {
    #[clap(long, default_value = "6543")]
    port: u16,
    #[clap(long, default_value = default_path())]
    path: PathBuf,
}

fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    if let Err(e) = server::run(cli.port, &cli.path) {
        tracing::error!("{}", e);
    }
}
