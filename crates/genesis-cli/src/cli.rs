//! CLI structure and command definitions.

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "genesis")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "BOSH Deployment Lifecycle Manager", long_about = None)]
#[command(arg_required_else_help = true)]
#[command(color = clap::ColorChoice::Auto)]
#[command(help_template = "{before-help}{name} {version}\n{about-with-newline}\n{usage-heading}\n    {usage}\n\n{all-args}{after-help}")]
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
    // ─── Core ───────────────────────────────────────────────────────────────

    /// Show version information
    Version {
        /// Show detailed version info
        #[arg(short, long)]
        verbose: bool,
    },

    /// Simple connectivity / health check
    Ping,

    /// Update Genesis to latest version
    Update {
        /// Check for updates without installing
        #[arg(short, long)]
        check: bool,

        /// Pre-release versions
        #[arg(long)]
        pre: bool,

        /// Specific version to install
        #[arg(short = 'v', long)]
        version: Option<String>,
    },

    // ─── Repository ─────────────────────────────────────────────────────────

    /// Initialize a new Genesis deployment repository
    Init {
        /// Repository directory
        #[arg(default_value = ".")]
        path: String,

        /// Kit to use for initial setup
        #[arg(short, long)]
        kit: Option<String>,
    },

    /// Configure the Vault/secrets provider for this repository
    #[command(name = "secrets-provider")]
    SecretsProvider {
        /// Vault target URL or name
        target: Option<String>,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,

        /// Clear the provider configuration
        #[arg(long)]
        clear: bool,
    },

    /// Configure the kit provider for this repository
    #[command(name = "kit-provider")]
    KitProvider {
        /// Provider URL or name
        provider: Option<String>,

        /// Set as default provider
        #[arg(long)]
        default: bool,

        /// Export current provider config as YAML
        #[arg(long)]
        export_config: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    // ─── Environment ────────────────────────────────────────────────────────

    /// Create a new deployment environment
    #[command(alias = "create")]
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

    /// Edit an environment file
    #[command(alias = "e")]
    Edit {
        /// Environment name
        env: String,

        /// Specific file to edit
        #[arg(short, long)]
        file: Option<String>,
    },

    /// Validate an environment's manifest (without deploying)
    Check {
        /// Environment name
        env: String,

        /// Skip BOSH config checks
        #[arg(long)]
        no_config: bool,

        /// Check secrets are present and valid
        #[arg(long)]
        secrets: bool,

        /// Validate manifest YAML
        #[arg(long)]
        manifest: bool,

        /// Check stemcell availability
        #[arg(long)]
        stemcells: bool,
    },

    /// Generate and show a manifest
    #[command(alias = "m")]
    Manifest {
        /// Environment name
        env: String,

        /// Write manifest to file
        #[arg(short, long)]
        output: Option<String>,

        /// Show redacted manifest (secrets hidden)
        #[arg(short, long)]
        redacted: bool,

        /// Manifest type (unredacted, redacted, partial, vaultified)
        #[arg(short = 't', long)]
        manifest_type: Option<String>,

        /// Cherry-pick a subset of keys
        #[arg(long)]
        subset: Option<String>,

        /// List available manifest types
        #[arg(short, long)]
        list: bool,
    },

    /// Deploy an environment to BOSH
    #[command(alias = "d")]
    Deploy {
        /// Environment name
        env: String,

        /// Dry run (show what would happen)
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Skip secrets generation
        #[arg(long)]
        no_secrets: bool,

        /// Redeploy even if no changes
        #[arg(long)]
        force: bool,

        /// Recreate all VMs
        #[arg(long)]
        recreate: bool,

        /// Fix stemcells
        #[arg(long)]
        fix_stemcells: bool,

        /// Skip draining instances
        #[arg(long)]
        skip_drain: bool,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Number of canary instances
        #[arg(long)]
        canaries: Option<u32>,

        /// Max instances to update in parallel
        #[arg(long)]
        max_in_flight: Option<u32>,
    },

    /// Delete a BOSH deployment (without cleaning secrets)
    Delete {
        /// Environment name
        env: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Terminate a deployment and optionally clean up all associated resources
    #[command(aliases = &["destroy", "implode", "kill"])]
    Terminate {
        /// Environment name
        env: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,

        /// Dry run mode
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Force deletion even if already deleted
        #[arg(long)]
        force: bool,

        /// Also remove secrets from Vault
        #[arg(long)]
        secrets: bool,

        /// Remove all associated resources (secrets, exodus, networking)
        #[arg(long)]
        all: bool,
    },

    /// Run a kit addon script
    #[command(name = "do", aliases = &["addon", "apply"])]
    Addon {
        /// Environment name
        env: String,

        /// Addon script name
        script: String,

        /// Arguments to pass to the addon script
        args: Vec<String>,
    },

    // ─── Secrets ────────────────────────────────────────────────────────────

    /// Generate missing secrets for an environment
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

        /// Remove all secrets (including user-provided)
        #[arg(long)]
        all: bool,

        /// Remove unused secrets only
        #[arg(long)]
        unused: bool,
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

        /// Rotate only problematic/invalid secrets
        #[arg(long)]
        problematic: bool,
    },

    /// Check/validate secrets for an environment
    #[command(name = "check-secrets")]
    CheckSecrets {
        /// Environment name
        env: String,

        /// Only check existence (don't validate values)
        #[arg(long)]
        exists: bool,
    },

    // ─── BOSH / Infrastructure ───────────────────────────────────────────────

    /// Run BOSH commands for an environment (or check BOSH connectivity)
    #[command(alias = "b")]
    Bosh {
        /// Environment name (optional; omit to check global connectivity)
        env: Option<String>,

        /// Just set up the BOSH connection and print env vars
        #[arg(long)]
        connect: bool,

        /// Show BOSH director status
        #[arg(short, long)]
        status: bool,

        /// Use this environment as its own BOSH director
        #[arg(long)]
        self_: bool,

        /// BOSH arguments to pass through
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Run CredHub commands for an environment
    Credhub {
        /// Environment name
        env: String,

        /// Skip path manipulation
        #[arg(long)]
        raw: bool,

        /// CredHub arguments to pass through
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Fetch BOSH deployment logs
    Logs {
        /// Environment name
        env: String,

        /// Instance group/id (e.g., web/0)
        instance: Option<String>,

        /// Follow (tail) logs
        #[arg(short, long)]
        follow: bool,

        /// Number of lines
        #[arg(short, long)]
        num: Option<u32>,

        /// Specific job
        #[arg(short, long)]
        job: Option<String>,
    },

    /// Run a command across all BOSH deployment instances
    Broadcast {
        /// Environment name
        env: String,

        /// Target only specific instance group
        #[arg(long)]
        on: Option<String>,

        /// Command to run
        command: String,
    },

    /// Manage BOSH configs (cloud-config, runtime-config, cpi-config)
    #[command(name = "bosh-configs")]
    BoshConfigs {
        /// Environment name
        env: String,

        /// Action: upload, list, view, compare, delete, summary
        #[arg(default_value = "list")]
        action: String,

        /// Config type (cloud, runtime, cpi)
        #[arg(short = 't', long)]
        config_type: Option<String>,

        /// Config file to upload
        #[arg(short, long)]
        file: Option<String>,

        /// Dry run
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    // ─── Information ────────────────────────────────────────────────────────

    /// Show detailed information about an environment
    #[command(alias = "information")]
    Info {
        /// Environment name
        env: String,

        /// Show deployment history
        #[arg(long)]
        history: bool,
    },

    /// Look up a value in an environment's configuration or manifest
    Lookup {
        /// Environment name
        env: String,

        /// Key to look up (dot-separated for nested keys)
        key: String,

        /// Search in merged manifest (default)
        #[arg(long)]
        merged: bool,

        /// Search in partial (unevaluated) manifest
        #[arg(long)]
        partial: bool,

        /// Search in the last deployed manifest
        #[arg(long)]
        deployed: bool,

        /// Search in exodus data
        #[arg(long)]
        exodus: bool,

        /// Search in another environment's exodus data
        #[arg(long)]
        exodus_for: Option<String>,

        /// Exit 1 (silently) if key not defined instead of erroring
        #[arg(long)]
        defined: bool,

        /// Output as shell variable assignment
        #[arg(long)]
        env_var: bool,
    },

    /// List YAML files that make up an environment's manifest
    Yamls {
        /// Environment name
        env: String,

        /// Include kit YAML files
        #[arg(long)]
        include_kit: bool,

        /// View file contents inline
        #[arg(long)]
        view: bool,
    },

    /// Show vault secret paths for an environment
    #[command(name = "vault-paths")]
    VaultPaths {
        /// Environment name
        env: String,

        /// Show vault operator reference strings
        #[arg(long)]
        references: bool,
    },

    /// Display kit documentation
    #[command(name = "kit-manual")]
    KitManual {
        /// Environment name
        env: String,

        /// Output raw text (no formatting)
        #[arg(long)]
        raw: bool,

        /// Display in pager
        #[arg(long)]
        pager: bool,
    },

    /// List all environments in the repository
    #[command(name = "list", alias = "environments")]
    ListEnvs {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Group by environment type
        #[arg(long)]
        group_by_type: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Filter by pattern
        filter: Option<String>,
    },

    /// Show differences between two environments' manifests
    Diff {
        /// First environment
        env1: String,

        /// Second environment
        env2: String,
    },

    // ─── Exodus ─────────────────────────────────────────────────────────────

    /// Export exodus data for an environment
    #[command(name = "export")]
    ExportExodus {
        /// Environment name
        env: String,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Import exodus data from one environment to another
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

    // ─── Kit Management ─────────────────────────────────────────────────────

    /// List available kits
    #[command(name = "list-kits")]
    ListKits {
        /// Show all versions
        #[arg(short, long)]
        all: bool,

        /// Show only latest versions
        #[arg(long)]
        latest: bool,

        /// Filter by name
        #[arg(long)]
        filter: Option<String>,

        /// Show details (description, authors)
        #[arg(short, long)]
        details: bool,
    },

    /// Download a kit from Genesis community or GitHub
    #[command(name = "fetch-kit", alias = "download")]
    FetchKit {
        /// Kit name
        kit: String,

        /// Kit version (latest if not specified)
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,

        /// Extract into dev/ directory
        #[arg(long)]
        as_dev: bool,

        /// Force overwrite
        #[arg(short, long)]
        force: bool,
    },

    /// Create a new kit scaffold
    #[command(name = "create-kit")]
    CreateKit {
        /// Kit name
        name: String,

        /// Create as dev kit
        #[arg(long)]
        dev: bool,

        /// Target directory
        #[arg(short, long)]
        directory: Option<String>,
    },

    /// Compile/build a kit tarball from source
    #[command(name = "build-kit", aliases = &["compile-kit"])]
    BuildKit {
        /// Kit source directory
        #[arg(default_value = ".")]
        directory: String,

        /// Override version
        #[arg(short = 'v', long)]
        version: Option<String>,

        /// Output directory
        #[arg(short, long)]
        target: Option<String>,

        /// Force overwrite
        #[arg(short, long)]
        force: bool,
    },

    /// Extract a compiled kit into a dev/ directory
    #[command(name = "decompile-kit")]
    DecompileKit {
        /// Kit tarball, version, or directory (default: latest in current dir)
        source: Option<String>,

        /// Target directory (default: dev/)
        #[arg(short, long)]
        directory: Option<String>,

        /// Force overwrite
        #[arg(short, long)]
        force: bool,
    },

    /// Compare two kit versions
    #[command(name = "compare-kits")]
    CompareKits {
        /// Kit name
        kit: String,

        /// First version (default: second-latest)
        version1: Option<String>,

        /// Second version to compare to (default: latest)
        #[arg(long)]
        compare_to: Option<String>,

        /// Show unchanged jobs too
        #[arg(long)]
        show_unchanged_jobs: bool,
    },

    // ─── Hooks ──────────────────────────────────────────────────────────────

    /// Run a kit hook directly
    Run {
        /// Hook type (addon, blueprint, check, info, new, pre-deploy, post-deploy)
        hook: String,

        /// Environment name
        env: Option<String>,

        /// Hook arguments
        args: Vec<String>,
    },

    // ─── Vault ──────────────────────────────────────────────────────────────

    /// Check Vault connectivity
    #[command(name = "vault")]
    VaultCheck {
        /// Show Vault status details
        #[arg(short, long)]
        status: bool,
    },

    // ─── Pipelines ──────────────────────────────────────────────────────────

    /// Embed this Genesis binary into the repository
    Embed,

    /// Generate and upload a Concourse pipeline
    Repipe {
        /// Pipeline config file
        #[arg(short, long)]
        config: Option<String>,

        /// Vault target for pipeline secrets
        #[arg(long)]
        vault: Option<String>,

        /// Pipeline layout
        #[arg(long)]
        layout: Option<String>,

        /// Concourse target
        #[arg(short, long)]
        target: Option<String>,

        /// Dry run - show pipeline without uploading
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,

        /// Create pipeline in paused state
        #[arg(long)]
        paused: bool,
    },

    /// Generate a GraphViz pipeline dependency graph
    Graph {
        /// Pipeline config file
        #[arg(short, long)]
        config: Option<String>,

        /// Pipeline layout
        #[arg(long)]
        layout: Option<String>,
    },

    /// Describe the pipeline in human-readable format
    Describe {
        /// Pipeline config file
        #[arg(short, long)]
        config: Option<String>,

        /// Pipeline layout
        #[arg(long)]
        layout: Option<String>,
    },

    /// CI task: deploy an environment (for use in Concourse pipeline)
    #[command(name = "ci-pipeline-deploy")]
    CiPipelineDeploy,

    /// CI task: show manifest changes since last deployment
    #[command(name = "ci-show-changes")]
    CiShowChanges,

    /// CI task: generate cache from previous environment
    #[command(name = "ci-generate-cache")]
    CiGenerateCache,

    /// CI task: run a BOSH errand in pipeline context
    #[command(name = "ci-pipeline-run-errand")]
    CiPipelineRunErrand,
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        use crate::commands::*;

        match &self.command {
            // ── Core ──────────────────────────────────────────────────────
            Commands::Version { verbose } => {
                version::execute(*verbose).await
            }
            Commands::Ping => {
                println!("PING!");
                Ok(())
            }
            Commands::Update { check, pre: _, version: _ } => {
                update::execute(*check).await
            }

            // ── Repository ────────────────────────────────────────────────
            Commands::Init { path, kit: _ } => {
                init::execute(path).await
            }
            Commands::SecretsProvider { target, interactive, clear } => {
                repo::secrets_provider(target.as_deref(), *interactive, *clear).await
            }
            Commands::KitProvider { provider, default, export_config, verbose } => {
                repo::kit_provider(provider.as_deref(), *default, *export_config, *verbose).await
            }

            // ── Environment ───────────────────────────────────────────────
            Commands::New { name, kit, version } => {
                new::execute(name, kit.as_deref(), version.as_deref()).await
            }
            Commands::Edit { env, file } => {
                edit::execute(env, file.as_deref()).await
            }
            Commands::Check { env, no_config, secrets, manifest, stemcells } => {
                check::execute(env, *no_config, *secrets, *manifest, *stemcells).await
            }
            Commands::Manifest { env, output, redacted, manifest_type: _, subset: _, list: _ } => {
                manifest::execute(env, output.as_deref(), *redacted).await
            }
            Commands::Deploy { env, dry_run, no_secrets, force, recreate, fix_stemcells, skip_drain, yes, canaries, max_in_flight } => {
                deploy::execute(env, *dry_run, *no_secrets, *force, *yes, *recreate, *fix_stemcells, *skip_drain, *canaries, *max_in_flight).await
            }
            Commands::Delete { env, yes } => {
                delete::execute(env, *yes).await
            }
            Commands::Terminate { env, yes, dry_run, force, secrets, all } => {
                terminate::execute(env, *yes, *dry_run, *force, *secrets, *all).await
            }
            Commands::Addon { env, script, args } => {
                addon::execute(env, script, args).await
            }

            // ── Secrets ───────────────────────────────────────────────────
            Commands::AddSecrets { env, force } => {
                secrets::add(env, *force).await
            }
            Commands::RemoveSecrets { env, yes, all: _, unused: _ } => {
                secrets::remove(env, *yes).await
            }
            Commands::RotateSecrets { env, paths, yes, problematic: _ } => {
                secrets::rotate(env, paths.as_ref(), *yes).await
            }
            Commands::CheckSecrets { env, exists: _ } => {
                secrets::check(env).await
            }

            // ── BOSH / Infrastructure ─────────────────────────────────────
            Commands::Bosh { env, connect, status, self_, args } => {
                bosh::execute(env.as_deref(), *connect, *status, *self_, args).await
            }
            Commands::Credhub { env, raw, args } => {
                bosh::credhub(env, *raw, args).await
            }
            Commands::Logs { env, instance, follow, num, job } => {
                bosh::logs(env, instance.as_deref(), *follow, *num, job.as_deref()).await
            }
            Commands::Broadcast { env, on, command } => {
                bosh::broadcast(env, on.as_deref(), command).await
            }
            Commands::BoshConfigs { env, action, config_type, file, dry_run } => {
                bosh::configs(env, action, config_type.as_deref(), file.as_deref(), *dry_run).await
            }

            // ── Information ───────────────────────────────────────────────
            Commands::Info { env, history: _ } => {
                info::execute(env).await
            }
            Commands::Lookup { env, key, merged, partial, deployed, exodus, exodus_for, defined, env_var } => {
                lookup::execute(env, key, *merged, *partial, *deployed, *exodus, exodus_for.as_deref(), *defined, *env_var).await
            }
            Commands::Yamls { env, include_kit, view } => {
                yamls::execute(env, *include_kit, *view).await
            }
            Commands::VaultPaths { env, references } => {
                vault_paths::execute(env, *references).await
            }
            Commands::KitManual { env, raw, pager } => {
                kit_manual::execute(env, *raw, *pager).await
            }
            Commands::ListEnvs { detailed, group_by_type: _, json: _, filter: _ } => {
                list::envs(*detailed).await
            }
            Commands::Diff { env1, env2 } => {
                diff::execute(env1, env2).await
            }

            // ── Exodus ────────────────────────────────────────────────────
            Commands::ExportExodus { env, output } => {
                exodus::export(env, output.as_deref()).await
            }
            Commands::ImportExodus { from, to, keys } => {
                exodus::import(from, to, keys.as_ref()).await
            }

            // ── Kit Management ────────────────────────────────────────────
            Commands::ListKits { all, latest: _, filter: _, details: _ } => {
                list::kits(*all).await
            }
            Commands::FetchKit { kit, version, output, as_dev, force } => {
                kit_cmds::fetch(kit, version.as_deref(), output, *as_dev, *force).await
            }
            Commands::CreateKit { name, dev, directory } => {
                kit_cmds::create(name, *dev, directory.as_deref()).await
            }
            Commands::BuildKit { directory, version, target, force } => {
                kit_cmds::build(Some(directory), version.as_deref(), target.as_deref(), *force).await
            }
            Commands::DecompileKit { source, directory, force } => {
                kit_cmds::decompile(source.as_deref(), directory.as_deref(), *force).await
            }
            Commands::CompareKits { kit, version1, compare_to, show_unchanged_jobs } => {
                kit_cmds::compare(kit, version1.as_deref(), compare_to.as_deref(), *show_unchanged_jobs).await
            }

            // ── Hooks ─────────────────────────────────────────────────────
            Commands::Run { hook, env, args } => {
                run::execute(hook, env.as_deref(), args).await
            }

            // ── Vault ─────────────────────────────────────────────────────
            Commands::VaultCheck { status } => {
                vault::check(*status).await
            }

            // ── Pipelines ─────────────────────────────────────────────────
            Commands::Embed => {
                pipeline::embed().await
            }
            Commands::Repipe { config, vault: _, layout, target, dry_run, yes, paused } => {
                pipeline::repipe(config.as_deref(), None, layout.as_deref(), target.as_deref(), *dry_run, *yes, *paused).await
            }
            Commands::Graph { config, layout } => {
                pipeline::graph(config.as_deref(), layout.as_deref()).await
            }
            Commands::Describe { config, layout } => {
                pipeline::describe(config.as_deref(), layout.as_deref()).await
            }
            Commands::CiPipelineDeploy => {
                pipeline::ci_pipeline_deploy().await
            }
            Commands::CiShowChanges => {
                pipeline::ci_show_changes().await
            }
            Commands::CiGenerateCache => {
                pipeline::ci_generate_cache().await
            }
            Commands::CiPipelineRunErrand => {
                pipeline::ci_pipeline_run_errand().await
            }
        }
    }
}
