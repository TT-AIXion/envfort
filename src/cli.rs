use clap::{Args, Parser, Subcommand, ValueEnum};

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
    Allowlist(AllowlistArgs),
    Rm(RemoveArgs),
    #[command(name = "rotate-kek")]
    RotateKek(RotateKekArgs),
    Export(ExportArgs),
    Import(ImportArgs),
    Audit(AuditArgs),
    Kdf(KdfArgs),
    Profile(ProfileArgs),
    Ui(UiArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
    #[arg(long, default_value = "vault.db")]
    pub db: String,
}

#[derive(Debug, Args)]
pub struct SetArgs {
    pub key: String,
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
    #[arg(long, default_value_t = false)]
    pub ci: bool,
    #[arg(long, value_enum)]
    pub inject: Option<InjectMode>,
    #[arg(long, default_value_t = false)]
    pub llm_safe: bool,
}

#[derive(Debug, Args)]
pub struct AllowlistArgs {
    #[command(subcommand)]
    pub command: AllowlistCommands,
}

#[derive(Debug, Subcommand)]
pub enum AllowlistCommands {
    List,
    Add(AllowlistCommandArgs),
    Rm(AllowlistCommandArgs),
}

#[derive(Debug, Args)]
pub struct AllowlistCommandArgs {
    pub command: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InjectMode {
    Env,
    Stdin,
    Fd,
    Socket,
    Tmpfile,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    pub key: String,
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(Debug, Args)]
pub struct RotateKekArgs {
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[arg(long)]
    pub encrypted: bool,
    #[arg(long)]
    pub output: String,
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(Debug, Args)]
pub struct ImportArgs {
    #[arg(long)]
    pub encrypted: bool,
    pub path: String,
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
pub struct AuditArgs {
    #[arg(long, default_value_t = 20)]
    pub tail: usize,
}

#[derive(Debug, Args)]
pub struct KdfArgs {
    #[command(subcommand)]
    pub command: KdfCommands,
}

#[derive(Debug, Subcommand)]
pub enum KdfCommands {
    Calibrate(KdfCalibrateArgs),
}

#[derive(Debug, Args)]
pub struct KdfCalibrateArgs {
    #[arg(long)]
    pub target_ms: u64,
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommands,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCommands {
    Create(ProfileNameArgs),
    Delete(ProfileNameArgs),
    List,
}

#[derive(Debug, Args)]
pub struct ProfileNameArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct UiArgs {
    #[arg(long, default_value_t = false)]
    pub no_open: bool,
    #[arg(long, default_value_t = 30)]
    pub timeout: u64,
}

pub fn parse_cli() -> Cli {
    Cli::parse()
}
