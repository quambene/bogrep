use anyhow::anyhow;
use bogrep::{cmd, Args, Config, Logger, Subcommands, TargetReaderWriter};
use clap::Parser;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();
    Logger::init(args.verbose);
    let config = Config::init()?;
    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;

    tokio::select! {
        _ = signal::ctrl_c() => {
            // Clean up when aborting.
            if let Err(err) =  target_reader_writer.close() {
                eprintln!("Can't finish clean up: {err:?}");
            }


            println!("Aborting ...");
            Ok(())
        },
        res = run_app(args, config, &target_reader_writer) => {
            target_reader_writer.close()?;
            res
        }
    }
}

async fn run_app(
    args: Args,
    config: Config,
    target_reader_writer: &TargetReaderWriter,
) -> Result<(), anyhow::Error> {
    if let Some(subcommands) = args.subcommands {
        match subcommands {
            Subcommands::Config(args) => cmd::configure(config, args)?,
            Subcommands::Import(args) => cmd::import(config, args, target_reader_writer).await?,
            Subcommands::Sync(args) => cmd::sync(&config, &args, target_reader_writer).await?,
            Subcommands::Fetch(args) => cmd::fetch(&config, &args, target_reader_writer).await?,
            Subcommands::Clean(args) => cmd::clean(&config, &args, target_reader_writer).await?,
            Subcommands::Add(args) => cmd::add(config, args, target_reader_writer).await?,
            Subcommands::Remove(args) => cmd::remove(config, args, target_reader_writer).await?,
        }
    } else if let Some(pattern) = &args.pattern {
        cmd::search(pattern, &config, &args, target_reader_writer)?;
    } else {
        return Err(anyhow!("Missing search pattern: `bogrep <pattern>`"));
    }

    Ok(())
}
