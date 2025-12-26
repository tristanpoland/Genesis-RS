# Genesis Rust Rewrite - Complete Technical Specification

## Executive Summary

This document provides a comprehensive technical specification for rewriting the Genesis BOSH deployment tool from Perl to Rust. Genesis is a sophisticated deployment orchestration system with ~17,000 lines of Perl code across 94 modules. The Rust rewrite will maintain full semantic compatibility while leveraging Rust's type safety, performance, and modern ecosystem.

**Project Scope:**
- Complete rewrite of Genesis v3.x Perl codebase to Rust
- Multi-crate workspace architecture following Rust best practices
- Full functional compatibility with existing Genesis behavior
- Improved error handling, type safety, and performance
- Modern async I/O for network operations
- Comprehensive documentation and testing

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Crate Structure](#2-crate-structure)
3. [Core Modules Specification](#3-core-modules-specification)
4. [Type System Design](#4-type-system-design)
5. [Error Handling Strategy](#5-error-handling-strategy)
6. [External Dependencies](#6-external-dependencies)
7. [Implementation Roadmap](#7-implementation-roadmap)
8. [Testing Strategy](#8-testing-strategy)
9. [Migration from Perl](#9-migration-from-perl)
10. [Appendices](#10-appendices)

---

## 1. Architecture Overview

### 1.1 Design Principles

1. **Modular Design**: Separate crates for distinct functionality areas
2. **Type Safety**: Leverage Rust's type system to prevent runtime errors
3. **Error Propagation**: Use Result types throughout, anyhow for applications
4. **Trait-Based Abstractions**: Polymorphism through traits instead of inheritance
5. **Async Where Beneficial**: Network I/O, parallel operations
6. **Zero-Copy Where Possible**: Efficient data handling
7. **Semantic Compatibility**: Maintain behavior parity with Perl version

### 1.2 System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         genesis-cli                              │
│                   (Binary crate - CLI interface)                 │
└────────────────────────────┬────────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  genesis-env    │  │  genesis-kit    │  │ genesis-secrets │
│ (Environments)  │  │  (Kits/Hooks)   │  │   (Secrets)     │
└────────┬────────┘  └────────┬────────┘  └────────┬────────┘
         │                    │                     │
         └────────────────────┼─────────────────────┘
                              │
                  ┌───────────┴────────────┐
                  ▼                        ▼
         ┌─────────────────┐      ┌─────────────────┐
         │genesis-manifest │      │genesis-services │
         │   (Manifests)   │      │ (Vault/BOSH/etc)│
         └────────┬────────┘      └────────┬────────┘
                  │                        │
                  └────────────┬───────────┘
                               ▼
                   ┌─────────────────────┐
                   │   genesis-core      │
                   │ (Utils/Config/Log)  │
                   └─────────────────────┘
                               │
                               ▼
                   ┌─────────────────────┐
                   │   genesis-types     │
                   │ (Common Types/Traits)│
                   └─────────────────────┘
```

### 1.3 Data Flow

**Deployment Flow:**
```
User Command → CLI Parser → Command Handler
    ↓
Environment Loader → Kit Validator → Prerequisites Check
    ↓
Secret Plan → Secret Validation/Generation → Vault Storage
    ↓
Manifest Pipeline:
  - YAML Hierarchy Merge
  - Blueprint Evaluation
  - Feature Processing
  - Spruce Merge
  - Transformations (Redact/Vaultify/Entomb)
    ↓
Pre-Deploy Hook → BOSH Deployment → Post-Deploy Hook
    ↓
Exodus Data Storage → Deployment Audit
```

---

## 2. Crate Structure

### 2.1 Workspace Organization

**Cargo.toml (Workspace Root):**
```toml
[workspace]
members = [
    "crates/genesis-types",
    "crates/genesis-core",
    "crates/genesis-manifest",
    "crates/genesis-services",
    "crates/genesis-secrets",
    "crates/genesis-kit",
    "crates/genesis-env",
    "crates/genesis-cli",
]
resolver = "2"

[workspace.package]
version = "3.0.0"
edition = "2021"
rust-version = "1.75.0"
license = "MIT"
repository = "https://github.com/genesis-community/genesis-rs"
authors = ["Genesis Community"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# CLI
clap = { version = "4", features = ["derive", "color", "suggestions"] }
dialoguer = "0.11"

# Logging/Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"

# HTTP/Networking
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
url = "2.5"

# Crypto
sha2 = "0.10"
sha1 = "0.10"
hex = "0.4"

# File/Path handling
walkdir = "2"
tempfile = "3"
tar = "0.4"
flate2 = "1"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# Misc utilities
regex = "1"
lazy_static = "1.4"
once_cell = "1"
uuid = { version = "1", features = ["v4", "serde"] }
```

### 2.2 Individual Crate Specifications

#### 2.2.1 genesis-types

**Purpose:** Common types, traits, and interfaces shared across all crates.

**Key Components:**
- Result type aliases
- Common enums (LogLevel, HookType, ManifestType, SecretType)
- Core traits (Provider, Store, Validator)
- Error types
- Configuration primitives

**Dependencies:** Minimal (serde, thiserror only)

#### 2.2.2 genesis-core

**Purpose:** Core utilities, configuration management, logging, and common functions.

**Key Modules:**
- `config/` - Multi-layer configuration (global, repo, environment)
- `log/` - Structured logging system
- `term/` - Terminal utilities and colored output
- `util/` - File operations, YAML/JSON handling, process execution
- `state/` - Global application state
- `semver/` - Semantic version handling
- `time/` - Time utilities and formatting

**Dependencies:**
- genesis-types
- serde, serde_yaml, serde_json
- tracing, tracing-subscriber
- chrono
- colored, console
- tokio

#### 2.2.3 genesis-manifest

**Purpose:** Manifest generation pipeline and transformations.

**Key Modules:**
- `provider/` - Manifest factory and providers
- `types/` - Different manifest types (Redacted, Vaultified, etc.)
- `merge/` - YAML merging logic (spruce integration)
- `transform/` - Manifest transformations
- `cache/` - Manifest caching
- `subset/` - Subset extraction (cherry-pick, prune)

**Dependencies:**
- genesis-core
- genesis-types
- serde_yaml
- regex

#### 2.2.4 genesis-services

**Purpose:** External service integrations (Vault, BOSH, CredHub, GitHub).

**Key Modules:**
- `vault/` - Vault client (Remote, Local, None)
- `bosh/` - BOSH director client
- `credhub/` - CredHub client
- `github/` - GitHub API client

**Dependencies:**
- genesis-core
- genesis-types
- reqwest
- async-trait
- tokio

#### 2.2.5 genesis-secrets

**Purpose:** Secret management, generation, validation, and rotation.

**Key Modules:**
- `types/` - Secret type implementations (X509, SSH, RSA, etc.)
- `plan/` - Secret plan management
- `parser/` - Secret definition parsing (from kit, from manifest)
- `generator/` - Secret generation
- `validator/` - Secret validation
- `store/` - Secret store abstraction

**Dependencies:**
- genesis-core
- genesis-types
- genesis-services
- openssl (for crypto operations)
- uuid

#### 2.2.6 genesis-kit

**Purpose:** Kit handling, hook system, and blueprint processing.

**Key Modules:**
- `compiled/` - Compiled kit handling
- `dev/` - Development kit handling
- `provider/` - Kit providers (GitHub, custom)
- `hook/` - Hook execution system
- `metadata/` - Kit metadata handling

**Dependencies:**
- genesis-core
- genesis-types
- tar, flate2
- tokio

#### 2.2.7 genesis-env

**Purpose:** Environment and deployment management.

**Key Modules:**
- `environment/` - Environment loading and validation
- `deployment/` - Deployment orchestration
- `features/` - Feature management
- `exodus/` - Exodus data handling

**Dependencies:**
- genesis-core
- genesis-types
- genesis-kit
- genesis-manifest
- genesis-secrets
- genesis-services

#### 2.2.8 genesis-cli

**Purpose:** Command-line interface and command implementations.

**Binary crate with submodules:**
- `commands/` - Command implementations
  - `env.rs` - Environment commands (create, deploy, etc.)
  - `bosh.rs` - BOSH commands
  - `kit.rs` - Kit commands
  - `repo.rs` - Repository commands
  - `info.rs` - Informational commands
  - `pipeline.rs` - Pipeline commands
  - `core.rs` - Genesis management commands
- `ui/` - User interaction (prompts, progress bars)
- `main.rs` - CLI entry point

**Dependencies:**
- All other genesis crates
- clap
- dialoguer
- indicatif (for progress bars)

---

## 3. Core Modules Specification

### 3.1 Configuration System (genesis-core/config)

**Purpose:** Multi-layer configuration with validation and merging.

**Key Types:**

```rust
/// Configuration layer enumeration
pub enum ConfigLayer {
    Default,
    Loaded,      // From file
    Set,         // Programmatically set
    Environment, // From env vars
}

/// Main configuration structure
pub struct Config {
    layers: HashMap<ConfigLayer, Value>,
    file_path: Option<PathBuf>,
    auto_save: bool,
    schema: Option<Schema>,
}

impl Config {
    /// Create new config with schema validation
    pub fn new(path: impl AsRef<Path>) -> Result<Self>;

    /// Get value with priority: Environment > Set > Loaded > Default
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;

    /// Set value programmatically
    pub fn set(&mut self, key: &str, value: impl Serialize) -> Result<()>;

    /// Save to file
    pub fn save(&self) -> Result<()>;

    /// Validate against schema
    pub fn validate(&self) -> Result<()>;
}

/// Global configuration (~/.genesis/config)
pub struct GlobalConfig {
    config: Config,
    deployment_roots: Vec<DeploymentRoot>,
    kit_provider: ProviderConfig,
    secrets_provider: SecretsProviderConfig,
    logs: Vec<LogConfig>,
}

/// Repository configuration (.genesis/config)
pub struct RepoConfig {
    config: Config,
    deployment_type: String,
    version: u32,
    minimum_version: Option<String>,
    manifest_store: String,
    kits_path: Option<PathBuf>,
    secrets_provider: SecretsProviderConfig,
    kit_provider: Option<ProviderConfig>,
}
```

**Implementation Notes:**
- Use `serde` for YAML serialization
- Implement `priority_merge` function for merging configurations
- Schema validation using `jsonschema` crate
- Support for environment variable interpolation

### 3.2 Logging System (genesis-core/log)

**Purpose:** Structured logging with multiple outputs and stack traces.

**Key Types:**

```rust
use tracing::{Level, Subscriber};

/// Log level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    None,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Log configuration
pub struct LogConfig {
    pub path: String,      // Template-based path
    pub level: LogLevel,
    pub stack: bool,
    pub format: LogFormat,
}

#[derive(Debug, Clone)]
pub enum LogFormat {
    Pretty,
    Json,
    Compact,
}

/// Logger setup
pub struct Logger {
    configs: Vec<LogConfig>,
}

impl Logger {
    /// Initialize global logger from configs
    pub fn setup(configs: &[LogConfig]) -> Result<()>;

    /// Get current logger instance
    pub fn instance() -> &'static Logger;
}

/// Logging macros (wrapping tracing)
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => { tracing::error!($($arg)*) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { tracing::warn!($($arg)*) };
}

// ... similar for info, debug, trace
```

**Implementation Notes:**
- Use `tracing` for structured logging
- Support multiple simultaneous outputs (files, stderr)
- Template-based log file paths (with datetime substitution)
- Stack trace capture using `backtrace` crate
- Colored output using `console` or `colored` crate

### 3.3 Terminal Utilities (genesis-core/term)

**Purpose:** Colored output, terminal detection, and formatting.

**Key Types:**

```rust
use console::{Style, Term};

/// Terminal color formatting
pub struct TermFormatter {
    enabled: bool,
    styles: HashMap<char, Style>,
}

impl TermFormatter {
    /// Format string with color codes (e.g., "#R{red text}")
    pub fn format(&self, input: &str) -> String;

    /// Check if in controlling terminal
    pub fn in_controlling_terminal() -> bool;

    /// Get terminal width
    pub fn terminal_width() -> usize;

    /// Word wrap text to terminal width
    pub fn wrap(&self, text: &str, width: Option<usize>) -> String;
}

/// Color code definitions
/// #R{} = red, #G{} = green, #Y{} = yellow, #B{} = blue, etc.
/// #M{} = magenta, #C{} = cyan, #W{} = white
/// #r{}, #g{}, etc. = dim variants
/// #*{} = bold
```

**Implementation Notes:**
- Parse color codes using regex
- Support for nested color codes
- Automatic color disabling when not in TTY
- Respect `NO_COLOR` environment variable

### 3.4 Process Execution (genesis-core/util/process)

**Purpose:** Safe command execution with environment management.

**Key Types:**

```rust
use std::process::{Command, Stdio};
use std::collections::HashMap;

/// Process execution options
pub struct RunOptions {
    pub env: Option<HashMap<String, String>>,
    pub stdin: Option<String>,
    pub stderr: Stdio,
    pub stdout: Stdio,
    pub timeout: Option<Duration>,
    pub redact: Vec<String>,  // Secrets to redact from output
}

/// Execute command with options
pub fn run(
    command: &str,
    args: &[&str],
    opts: Option<RunOptions>
) -> Result<(String, i32, String)>;

/// Execute and return just stdout lines
pub fn lines(command: &str, args: &[&str]) -> Result<Vec<String>>;

/// Execute in background
pub async fn run_async(
    command: &str,
    args: &[&str],
    opts: Option<RunOptions>
) -> Result<(String, i32, String)>;

/// HTTP request wrapper
pub async fn curl(url: &str, opts: Option<CurlOptions>) -> Result<String>;
```

**Implementation Notes:**
- Use `tokio::process` for async execution
- Automatic secret redaction from output
- Timeout support
- Environment variable management (inheritance, override)
- Signal handling for graceful shutdown

### 3.5 YAML/JSON Handling (genesis-core/util/data)

**Purpose:** Safe YAML and JSON loading/saving with type conversions.

**Key Functions:**

```rust
use serde::{Serialize, Deserialize};
use serde_yaml::Value as YamlValue;
use serde_json::Value as JsonValue;

/// Load YAML from string
pub fn load_yaml(content: &str) -> Result<YamlValue>;

/// Load YAML from file
pub fn load_yaml_file(path: impl AsRef<Path>) -> Result<YamlValue>;

/// Save YAML to file
pub fn save_yaml_file(path: impl AsRef<Path>, data: &impl Serialize) -> Result<()>;

/// Load JSON from string
pub fn load_json(content: &str) -> Result<JsonValue>;

/// Load JSON from file
pub fn load_json_file(path: impl AsRef<Path>) -> Result<JsonValue>;

/// Save JSON to file
pub fn save_json_file(path: impl AsRef<Path>, data: &impl Serialize) -> Result<()>;

/// Convert to YAML string
pub fn to_yaml(data: &impl Serialize) -> Result<String>;

/// Deep merge two YAML values (spruce-style)
pub fn deep_merge(base: YamlValue, overlay: YamlValue) -> YamlValue;

/// Priority merge (overlay wins completely for each key)
pub fn priority_merge(base: YamlValue, overlay: YamlValue) -> YamlValue;

/// Flatten YAML to dotted key-value pairs
pub fn flatten(data: &YamlValue) -> HashMap<String, YamlValue>;

/// Unflatten dotted keys back to nested structure
pub fn unflatten(flat: &HashMap<String, YamlValue>) -> YamlValue;
```

**Implementation Notes:**
- Use `serde_yaml` and `serde_json`
- Handle multi-document YAML files
- Preserve key order where possible
- Error context for parse failures

---

## 4. Type System Design

### 4.1 Core Types (genesis-types)

```rust
/// Environment name type with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvName(String);

impl EnvName {
    pub fn new(name: impl AsRef<str>) -> Result<Self> {
        // Validate environment name format
        let name = name.as_ref();
        if !Self::is_valid(name) {
            bail!("Invalid environment name: {}", name);
        }
        Ok(Self(name.to_string()))
    }

    fn is_valid(name: &str) -> bool {
        // Must match: [a-z0-9][-a-z0-9]*
        name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && name.chars().next().map_or(false, |c| c.is_ascii_lowercase() || c.is_ascii_digit())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Semantic version type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl SemVer {
    pub fn parse(version: &str) -> Result<Self>;
    pub fn is_compatible(&self, requirement: &VersionReq) -> bool;
}

/// Kit identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitId {
    pub name: String,
    pub version: SemVer,
}

impl Display for KitId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.name, self.version)
    }
}

/// Hook type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookType {
    New,
    Features,
    Blueprint,
    Info,
    Check,
    PreDeploy,
    PostDeploy,
    Terminate,
    Addon,
    CloudConfig,
    RuntimeConfig,
    CpiConfig,
    Edit,
    Shell,
}

/// Manifest type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestType {
    Unevaluated,
    UnevaluatedEnvironment,
    PartialEnvironment,
    Partial,
    Unredacted,
    Redacted,
    Vaultified,
    VaultifiedRedacted,
    Entombed,
    VaultifiedEntombed,
}

/// Secret type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretType {
    X509,
    SSH,
    RSA,
    DHParams,
    Random,
    UUID,
    UserProvided,
    Invalid,
}
```

### 4.2 Trait Definitions

```rust
/// Kit provider trait
#[async_trait]
pub trait KitProvider: Send + Sync {
    /// Fetch kit from provider
    async fn fetch(&self, name: &str, version: &SemVer) -> Result<PathBuf>;

    /// List available versions
    async fn versions(&self, name: &str) -> Result<Vec<SemVer>>;

    /// Get provider configuration
    fn config(&self) -> ProviderConfig;
}

/// Vault store trait
#[async_trait]
pub trait VaultStore: Send + Sync {
    /// Read secret from vault
    async fn read(&self, path: &str) -> Result<HashMap<String, String>>;

    /// Write secret to vault
    async fn write(&self, path: &str, data: &HashMap<String, String>) -> Result<()>;

    /// Check if path exists
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Delete secret
    async fn delete(&self, path: &str) -> Result<()>;

    /// List paths
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
}

/// Secret type trait
pub trait Secret: Send + Sync {
    /// Get secret type
    fn secret_type(&self) -> SecretType;

    /// Validate secret definition
    fn validate_definition(&self) -> Result<()>;

    /// Generate secret value
    fn generate(&self) -> Result<HashMap<String, String>>;

    /// Validate secret value
    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult>;

    /// Get required value keys
    fn required_keys(&self) -> &[&str];

    /// Get secret path
    fn path(&self) -> &str;
}

/// Manifest provider trait
pub trait ManifestProvider: Send + Sync {
    /// Get manifest type
    fn manifest_type(&self) -> ManifestType;

    /// Generate manifest
    fn generate(&self) -> Result<String>;

    /// Get manifest from cache if available
    fn cached(&self) -> Option<String>;
}
```

---

## 5. Error Handling Strategy

### 5.1 Error Types

```rust
use thiserror::Error;

/// Genesis-wide error type
#[derive(Error, Debug)]
pub enum GenesisError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Environment error: {0}")]
    Environment(String),

    #[error("Kit error: {0}")]
    Kit(String),

    #[error("Secret error: {0}")]
    Secret(String),

    #[error("Vault error: {0}")]
    Vault(String),

    #[error("BOSH error: {0}")]
    Bosh(String),

    #[error("Manifest error: {0}")]
    Manifest(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Hook execution error: {0}")]
    Hook(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Bug detected: {0}\nPlease report this at https://github.com/genesis-community/genesis-rs/issues")]
    Bug(String),
}

pub type Result<T> = std::result::Result<T, GenesisError>;

/// Bail macro for fatal errors
#[macro_export]
macro_rules! bail {
    ($msg:expr) => {
        return Err(GenesisError::from($msg))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(GenesisError::from(format!($fmt, $($arg)*)))
    };
}

/// Bug macro for internal errors
#[macro_export]
macro_rules! bug {
    ($msg:expr) => {
        return Err(GenesisError::Bug($msg.to_string()))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(GenesisError::Bug(format!($fmt, $($arg)*)))
    };
}
```

### 5.2 Error Context

```rust
use anyhow::{Context, Result as AnyhowResult};

// For application-level code (CLI), use anyhow::Result with context
pub fn load_environment(name: &str) -> AnyhowResult<Environment> {
    let env = Environment::load(name)
        .context(format!("Failed to load environment '{}'", name))?;
    Ok(env)
}

// For library code, use custom Result types
pub fn validate_config(&self) -> Result<()> {
    // ...
}
```

---

## 6. External Dependencies

### 6.1 Required External Tools

Genesis requires these external tools to be installed:

1. **BOSH CLI** (v6.4.4+)
   - Detection: `bosh --version`
   - Usage: Deployment operations

2. **spruce** (v1.28.0+)
   - Detection: `spruce --version`
   - Usage: YAML merging and operators

3. **safe/vault** (safe v0.9.0+, vault v1.6.1+)
   - Detection: `safe --version` or `vault --version`
   - Usage: Secret storage and retrieval

4. **credhub** (v2.7.0+)
   - Detection: `credhub --version`
   - Usage: CredHub integration

5. **jq** (v1.6+)
   - Detection: `jq --version`
   - Usage: JSON processing

6. **git**
   - Detection: `git --version`
   - Usage: Repository management

### 6.2 Prerequisite Checking

```rust
pub struct Prerequisite {
    pub name: &'static str,
    pub command: &'static str,
    pub min_version: Option<SemVer>,
    pub required: bool,
}

impl Prerequisite {
    pub async fn check(&self) -> Result<PrereqStatus>;
}

pub enum PrereqStatus {
    Ok(SemVer),
    Missing,
    TooOld { found: SemVer, required: SemVer },
}

pub async fn check_all_prerequisites() -> Result<Vec<(Prerequisite, PrereqStatus)>>;
```

---

## 7. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)

**Deliverables:**
- ✓ Project structure and workspace setup
- ✓ genesis-types crate with core types and traits
- ✓ genesis-core crate:
  - Configuration system
  - Logging system
  - Terminal utilities
  - File operations
  - Process execution
  - YAML/JSON handling

**Testing:**
- Unit tests for all utilities
- Integration tests for config system
- Property-based tests for YAML merging

### Phase 2: Services (Weeks 5-8)

**Deliverables:**
- genesis-services crate:
  - Vault client (Remote, Local, None)
  - BOSH client
  - CredHub client
  - GitHub client
- Async I/O implementation
- Connection pooling and retries

**Testing:**
- Mock service implementations
- Integration tests with real services (optional)
- Error handling tests

### Phase 3: Secrets Management (Weeks 9-12)

**Deliverables:**
- genesis-secrets crate:
  - Secret type implementations (X509, SSH, RSA, etc.)
  - Secret plan and parser
  - Secret generator
  - Secret validator
- OpenSSL integration for crypto

**Testing:**
- Secret generation tests
- Validation tests
- Certificate chain tests
- Rotation logic tests

### Phase 4: Kit System (Weeks 13-16)

**Deliverables:**
- genesis-kit crate:
  - Compiled and Dev kit handling
  - Kit provider implementations
  - Hook execution system
  - Blueprint processor
- Archive handling (tar.gz)

**Testing:**
- Kit extraction tests
- Hook execution tests
- Blueprint logic tests

### Phase 5: Manifest Pipeline (Weeks 17-20)

**Deliverables:**
- genesis-manifest crate:
  - Manifest types (all variants)
  - Spruce integration
  - Transformations (redact, vaultify, entomb)
  - Caching system
  - Subset operations

**Testing:**
- Manifest generation tests
- Transformation tests
- Caching tests

### Phase 6: Environment & Deployment (Weeks 21-24)

**Deliverables:**
- genesis-env crate:
  - Environment loading and validation
  - Deployment orchestration
  - Feature management
  - Exodus data handling

**Testing:**
- Environment lifecycle tests
- Deployment flow tests
- Rollback tests

### Phase 7: CLI (Weeks 25-28)

**Deliverables:**
- genesis-cli crate:
  - All command implementations (50+ commands)
  - User interaction (prompts, progress)
  - Help system
  - Shell completion

**Testing:**
- CLI integration tests
- Command-specific tests
- User interaction tests

### Phase 8: Integration & Testing (Weeks 29-32)

**Deliverables:**
- End-to-end integration tests
- Performance testing and optimization
- Documentation completion
- Migration guides

**Testing:**
- Full deployment scenarios
- Multi-environment tests
- Performance benchmarks

---

## 8. Testing Strategy

### 8.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_name_validation() {
        assert!(EnvName::new("valid-name").is_ok());
        assert!(EnvName::new("Invalid-Name").is_err());
        assert!(EnvName::new("-invalid").is_err());
    }

    #[tokio::test]
    async fn test_vault_read() {
        let vault = MockVault::new();
        let result = vault.read("secret/test").await;
        assert!(result.is_ok());
    }
}
```

### 8.2 Integration Tests

```rust
// tests/integration/deployment_flow.rs
#[tokio::test]
async fn test_full_deployment_flow() {
    // Setup test environment
    let temp_dir = tempdir()?;
    let repo = setup_test_repo(&temp_dir)?;

    // Create environment
    let env = repo.create_environment("test-env")?;

    // Deploy
    let result = env.deploy().await?;
    assert!(result.is_success());

    // Verify exodus data
    let exodus = env.exodus_data().await?;
    assert!(exodus.contains_key("version"));
}
```

### 8.3 Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn deep_merge_is_associative(
        a in any::<YamlValue>(),
        b in any::<YamlValue>(),
        c in any::<YamlValue>()
    ) {
        let left = deep_merge(deep_merge(a.clone(), b.clone()), c.clone());
        let right = deep_merge(a, deep_merge(b, c));
        assert_eq!(left, right);
    }
}
```

### 8.4 Test Coverage

**Target Coverage:**
- Core utilities: 90%+
- Business logic: 80%+
- CLI commands: 70%+
- Integration: 60%+

**Tools:**
- `cargo-tarpaulin` for coverage reports
- `cargo-nextest` for faster test execution
- `cargo-mutants` for mutation testing

---

## 9. Migration from Perl

### 9.1 Perl to Rust Pattern Mapping

| Perl Pattern | Rust Equivalent |
|--------------|-----------------|
| `bless({}, $class)` | `struct` with `impl` blocks |
| `@ISA` / `use base` | Trait composition |
| Hash as object | `struct` with named fields |
| `//=` default assignment | `Option::unwrap_or()` or `get_or_insert()` |
| `wantarray` | Separate functions or enum return |
| `eval { }` | `Result` type |
| `die` / `bail` | `Result::Err` or `panic!` |
| `our` variables | `lazy_static!` or `OnceCell` |
| `@EXPORT` | Public functions in modules |
| String interpolation | `format!()` macro |
| Regex `/pattern/` | `regex!()` macro |
| `backticks` | `Command::output()` |

### 9.2 Key Differences

**Object System:**
- Perl: Dynamic typing, runtime method resolution
- Rust: Static typing, compile-time trait resolution
- **Migration:** Use enums for sum types, traits for polymorphism

**Error Handling:**
- Perl: `eval` blocks, manual error propagation
- Rust: `Result` type, `?` operator
- **Migration:** Convert all error paths to `Result` types

**Memory Management:**
- Perl: Reference counting
- Rust: Ownership system
- **Migration:** Use smart pointers (`Rc`, `Arc`) where needed

**Concurrency:**
- Perl: Fork-based or single-threaded
- Rust: async/await with tokio
- **Migration:** Use async for I/O, threads for CPU-bound work

### 9.3 Compatibility Requirements

**Maintain:**
1. All command-line interfaces and options
2. Environment file format and structure
3. Kit format and metadata
4. Vault path structure
5. Exodus data format
6. BOSH manifest structure
7. Hook environment variables
8. Configuration file formats

**Improve:**
1. Error messages (more context, suggestions)
2. Performance (parallel operations, caching)
3. Type safety (catch errors at compile time)
4. Testing (comprehensive test suite)
5. Documentation (inline docs, examples)

---

## 10. Appendices

### Appendix A: Perl Module to Rust Crate Mapping

| Perl Module | Rust Crate/Module | Notes |
|-------------|-------------------|-------|
| Genesis.pm | genesis-core | Core utilities |
| Genesis::Base.pm | genesis-core/base.rs | Memoization via `once_cell` |
| Genesis::Commands.pm | genesis-cli/commands | Command registry |
| Genesis::Config.pm | genesis-core/config | Configuration management |
| Genesis::Env.pm | genesis-env/environment.rs | Environment handling |
| Genesis::Kit.pm | genesis-kit | Kit abstraction |
| Genesis::Secret.pm | genesis-secrets/types | Secret types |
| Genesis::Log.pm | genesis-core/log | Logging system |
| Genesis::Term.pm | genesis-core/term | Terminal utilities |
| Genesis::UI.pm | genesis-cli/ui | User interaction |
| Service::Vault.pm | genesis-services/vault | Vault client |
| Service::BOSH.pm | genesis-services/bosh | BOSH client |

### Appendix B: Estimated Lines of Code

| Component | Estimated LoC | Complexity |
|-----------|---------------|------------|
| genesis-types | 500 | Low |
| genesis-core | 3,000 | Medium |
| genesis-manifest | 2,000 | High |
| genesis-services | 2,500 | Medium |
| genesis-secrets | 2,500 | High |
| genesis-kit | 2,000 | Medium |
| genesis-env | 3,000 | High |
| genesis-cli | 4,000 | Medium |
| Tests | 8,000 | - |
| **Total** | **27,500** | - |

### Appendix C: Performance Targets

| Operation | Perl Baseline | Rust Target | Improvement |
|-----------|---------------|-------------|-------------|
| Manifest generation | 2.5s | 0.5s | 5x |
| Secret validation | 1.2s | 0.3s | 4x |
| Environment load | 0.8s | 0.2s | 4x |
| Kit extraction | 1.5s | 0.4s | 3.75x |
| Full deployment | 120s | 100s | 1.2x |

### Appendix D: Documentation Requirements

**Code Documentation:**
- All public APIs must have rustdoc comments
- Examples for complex functions
- Error cases documented

**User Documentation:**
- Migration guide from Perl version
- Command reference
- Configuration guide
- Kit authoring guide
- Troubleshooting guide

**Developer Documentation:**
- Architecture overview
- Contributing guide
- Testing guide
- Release process

---

## Conclusion

This specification provides a comprehensive blueprint for rewriting Genesis from Perl to Rust. The multi-crate architecture ensures modularity, the type system provides safety, and the implementation roadmap provides a clear path to completion.

**Key Success Factors:**
1. Maintain full semantic compatibility
2. Comprehensive testing at all levels
3. Clear error messages with context
4. Performance improvements through parallelization
5. Excellent documentation

**Next Steps:**
1. Review and approve specification
2. Set up CI/CD pipeline
3. Begin Phase 1 implementation
4. Weekly progress reviews
5. Community feedback integration

---

**Document Version:** 1.0
**Date:** 2025-12-26
**Status:** Draft for Review
