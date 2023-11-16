use anyhow::anyhow;
use bogrep::{cmd, Args, Config, Subcommands};
use clap::Parser;
use env_logger::{Builder, Env};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Default to INFO level logs if RUST_LOG is not set.
    Builder::from_env(Env::default().default_filter_or("bogrep=info")).init();

    let args = Args::parse();
    let config = Config::init(args.verbose)?;

    if let Some(subcommands) = args.subcommands {
        match subcommands {
            Subcommands::Config(args) => cmd::configure(config, args)?,
            Subcommands::Import => cmd::import(&config)?,
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
        }
    } else if let Some(pattern) = &args.pattern {
        cmd::search(pattern, &config, &args)?;
    } else {
        return Err(anyhow!("Missing search pattern: `bogrep <pattern>`"));
    }

    Ok(())
}
