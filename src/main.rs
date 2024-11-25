use anyhow::anyhow;
use bogrep::{cmd, Args, Config, Logger, Subcommands};
use clap::Parser;
use std::fs;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    Logger::init(args.verbose);
    let config = Config::init()?;
    let target_bookmark_lock_file = config.target_bookmark_lock_file.clone();

    tokio::select! {
        _ = signal::ctrl_c() => {
            // Clean up lock file when aborting.
            if target_bookmark_lock_file.exists() {
                if let Err(err) = fs::remove_file(target_bookmark_lock_file) {
                    eprintln!("Can't remove lock file: {err:?}")
                }
            }

            println!("Aborting ...");
            Ok(())
        },
        res = run_app(args, config) => {
            res
        }
    }
}

async fn run_app(args: Args, config: Config) -> Result<(), anyhow::Error> {
    if let Some(subcommands) = args.subcommands {
        match subcommands {
            Subcommands::Config(args) => cmd::configure(config, args)?,
            Subcommands::Import(args) => cmd::import(config, args).await?,
            Subcommands::Sync(args) => cmd::sync(&config, &args).await?,
            Subcommands::Fetch(args) => cmd::fetch(&config, &args).await?,
            Subcommands::Clean(args) => cmd::clean(&config, &args).await?,
            Subcommands::Add(args) => cmd::add(config, args).await?,
            Subcommands::Remove(args) => cmd::remove(config, args).await?,
        }
    } else if let Some(pattern) = &args.pattern {
        cmd::search(pattern, &config, &args)?;
    } else {
        return Err(anyhow!("Missing search pattern: `bogrep <pattern>`"));
    }

    Ok(())
}
