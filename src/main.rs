use anyhow::anyhow;
use bogrep::{cmd, Args, Config, Logger, Subcommands};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    Logger::init(args.verbose);
    let config = Config::init()?;

    run_app(args, config).await?;

    Ok(())
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
