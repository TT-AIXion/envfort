mod cli;
#[allow(dead_code)]
mod crypto;
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod keychain;
#[allow(dead_code)]
mod storage;

use crate::cli::{Commands, ProfileCommands, parse_cli};
use crate::error::CliError;
use crate::keychain::get_backend;

fn main() {
    if let Err(err) = dispatch_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn dispatch_main() -> Result<(), CliError> {
    let cli = parse_cli();

    match cli.command {
        Commands::Init(args) => {
            let _backend = get_backend();
            println!("initialized profile={} db={}", args.profile, args.db);
        }
        Commands::Set(args) => {
            let _backend = get_backend();
            println!("set key={} profile={}", args.key, args.profile);
        }
        Commands::List(args) => {
            println!("list profile={}", args.profile);
        }
        Commands::Run(args) => {
            println!(
                "run profile={} command={}",
                args.profile,
                args.command.join(" ")
            );
        }
        Commands::Rm(args) => {
            let _backend = get_backend();
            println!("remove key={} profile={}", args.key, args.profile);
        }
        Commands::Profile(args) => match args.command {
            ProfileCommands::Use(use_args) => {
                println!("profile use {}", use_args.name);
            }
            ProfileCommands::Current => {
                println!("profile current");
            }
            ProfileCommands::List => {
                println!("profile list");
            }
        },
    }

    Ok(())
}
