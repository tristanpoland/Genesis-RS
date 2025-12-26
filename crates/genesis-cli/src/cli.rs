//! CLI structure and command definitions.

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "genesis")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "BOSH Deployment Lifecycle Manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Enable debug output
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Suppress output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Genesis repository
    Init {
        /// Repository directory
        #[arg(default_value = ".")]
        path: String,
    },

    /// Create a new environment
    New {
        /// Environment name
        name: String,

        /// Kit name
        #[arg(short, long)]
        kit: Option<String>,

        /// Kit version
        #[arg(short = 'v', long)]
        version: Option<String>,
    },

    /// Deploy an environment
    Deploy {
        /// Environment name
        env: String,

        /// Dry run (don't actually deploy)
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Skip secrets generation
        #[arg(long)]
        no_secrets: bool,

        /// Redeploy even if no changes
        #[arg(long)]
        force: bool,
    },

    /// Delete a deployment
    Delete {
        /// Environment name
        env: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Generate secrets for an environment
    #[command(name = "add-secrets")]
    AddSecrets {
        /// Environment name
        env: String,

        /// Force regeneration of existing secrets
        #[arg(short, long)]
        force: bool,
    },

    /// Remove secrets for an environment
    #[command(name = "remove-secrets")]
    RemoveSecrets {
        /// Environment name
        env: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Rotate secrets for an environment
    #[command(name = "rotate-secrets")]
    RotateSecrets {
        /// Environment name
        env: String,

        /// Secret paths to rotate (all if not specified)
        #[arg(short, long)]
        paths: Option<Vec<String>>,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Check secrets for an environment
    #[command(name = "check-secrets")]
    CheckSecrets {
        /// Environment name
        env: String,
    },

    /// Show manifest for an environment
    Manifest {
        /// Environment name
        env: String,

        /// Write manifest to file
        #[arg(short, long)]
        output: Option<String>,

        /// Show redacted manifest (secrets hidden)
        #[arg(short, long)]
        redacted: bool,
    },

    /// Download a Genesis kit
    Download {
        /// Kit name
        kit: String,

        /// Kit version (latest if not specified)
        #[arg(short, long)]
        version: Option<String>,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,
    },

    /// List available kits
    #[command(name = "list-kits")]
    ListKits {
        /// Show all versions
        #[arg(short, long)]
        all: bool,
    },

    /// List environments
    #[command(name = "list")]
    ListEnvs {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show information about an environment
    Info {
        /// Environment name
        env: String,
    },

    /// Edit environment files
    Edit {
        /// Environment name
        env: String,

        /// Specific file to edit
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Show differences between manifests
    Diff {
        /// First environment
        env1: String,

        /// Second environment
        env2: String,
    },

    /// Export exodus data
    #[command(name = "export")]
    ExportExodus {
        /// Environment name
        env: String,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Import exodus data
    #[command(name = "import")]
    ImportExodus {
        /// Source environment
        from: String,

        /// Target environment
        to: String,

        /// Specific keys to import
        #[arg(short, long)]
        keys: Option<Vec<String>>,
    },

    /// Run kit hooks
    Run {
        /// Hook type (addon, blueprint, check, info, new, secrets, subkit)
        hook: String,

        /// Environment name
        env: Option<String>,

        /// Hook arguments
        args: Vec<String>,
    },

    /// Validate Vault connectivity
    #[command(name = "vault")]
    VaultCheck {
        /// Show Vault status
        #[arg(short, long)]
        status: bool,
    },

    /// Validate BOSH connectivity
    #[command(name = "bosh")]
    BoshCheck {
        /// Show BOSH status
        #[arg(short, long)]
        status: bool,
    },

    /// Update Genesis to latest version
    Update {
        /// Check for updates without installing
        #[arg(short, long)]
        check: bool,
    },

    /// Show version information
    Version {
        /// Show detailed version info
        #[arg(short, long)]
        verbose: bool,
    },
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        use crate::commands::*;

        match &self.command {
            Commands::Init { path } => {
                init::execute(path).await
            }
            Commands::New { name, kit, version } => {
                new::execute(name, kit.as_deref(), version.as_deref()).await
            }
            Commands::Deploy { env, dry_run, no_secrets, force } => {
                deploy::execute(env, *dry_run, *no_secrets, *force).await
            }
            Commands::Delete { env, yes } => {
                delete::execute(env, *yes).await
            }
            Commands::AddSecrets { env, force } => {
                secrets::add(env, *force).await
            }
            Commands::RemoveSecrets { env, yes } => {
                secrets::remove(env, *yes).await
            }
            Commands::RotateSecrets { env, paths, yes } => {
                secrets::rotate(env, paths.as_ref(), *yes).await
            }
            Commands::CheckSecrets { env } => {
                secrets::check(env).await
            }
            Commands::Manifest { env, output, redacted } => {
                manifest::execute(env, output.as_deref(), *redacted).await
            }
            Commands::Download { kit, version, output } => {
                download::execute(kit, version.as_deref(), output).await
            }
            Commands::ListKits { all } => {
                list::kits(*all).await
            }
            Commands::ListEnvs { detailed } => {
                list::envs(*detailed).await
            }
            Commands::Info { env } => {
                info::execute(env).await
            }
            Commands::Edit { env, file } => {
                edit::execute(env, file.as_deref()).await
            }
            Commands::Diff { env1, env2 } => {
                diff::execute(env1, env2).await
            }
            Commands::ExportExodus { env, output } => {
                exodus::export(env, output.as_deref()).await
            }
            Commands::ImportExodus { from, to, keys } => {
                exodus::import(from, to, keys.as_ref()).await
            }
            Commands::Run { hook, env, args } => {
                run::execute(hook, env.as_deref(), args).await
            }
            Commands::VaultCheck { status } => {
                vault::check(*status).await
            }
            Commands::BoshCheck { status } => {
                bosh::check(*status).await
            }
            Commands::Update { check } => {
                update::execute(*check).await
            }
            Commands::Version { verbose } => {
                version::execute(*verbose).await
            }
        }
    }
}
