use anyhow::Result;
use clap::Parser;
use marginfi_client_v2::Opts;

fn main() -> Result<()> {
    marginfi_client_v2::entry(Opts::parse())
}
