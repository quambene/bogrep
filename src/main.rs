use anyhow::anyhow;
use bogrep::{cmd, Args, Config, Logger, Subcommands};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    Logger::init(args.verbose);
    let config = Config::init()?;

    if let Some(subcommands) = args.subcommands {
        match subcommands {
            Subcommands::Config(args) => cmd::configure(config, args)?,
            Subcommands::Import(args) => cmd::import(&config, args)?,
            Subcommands::Init(args) => cmd::init(&config, &args).await?,
            Subcommands::Update(args) => cmd::update(&config, &args).await?,
            Subcommands::Fetch(args) => {
                if !args.diff.is_empty() {
                    cmd::fetch_diff(&config, args).await?;
                } else {
                    cmd::fetch(&config, &args).await?;
                }
            }
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
