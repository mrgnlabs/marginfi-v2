use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    marginfi_v2_indexer::entrypoint::entry(marginfi_v2_indexer::entrypoint::Opts::parse())
}
