use anyhow::Result;
use clap::Parser;
use marginfi_v2_cli::Opts;

fn main() -> Result<()> {
    marginfi_v2_cli::entry(Opts::parse())
}
