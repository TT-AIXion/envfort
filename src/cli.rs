use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "envfort", version, about = "Secret manager CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init(InitArgs),
    Set(SetArgs),
    List(ListArgs),
    Run(RunArgs),
    Rm(RemoveArgs),
    Profile(ProfileArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
    #[arg(long, default_value = "envfort.db")]
    pub db: String,
}

#[derive(Debug, Args)]
pub struct SetArgs {
    pub key: String,
    pub value: String,
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    #[arg(required = true)]
    pub command: Vec<String>,
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    pub key: String,
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommands,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCommands {
    Use(ProfileUseArgs),
    Current,
    List,
}

#[derive(Debug, Args)]
pub struct ProfileUseArgs {
    pub name: String,
}

pub fn parse_cli() -> Cli {
    Cli::parse()
}
