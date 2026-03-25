mod app;
mod cli;
mod color;
mod hid_transport;
mod nuphy_protocol;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use hidapi::HidApi;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let api = HidApi::new().context("failed to initialize hidapi")?;
    app::run(cli, &api)
}
