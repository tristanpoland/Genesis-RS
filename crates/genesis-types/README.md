# genesis-types

Core types, traits, and enums for the Genesis BOSH deployment tool.

## Overview

This crate provides the foundational type system for Genesis, including:

- **Type-safe identifiers**: `EnvName`, `KitId`, `SemVer`
- **Common enums**: `LogLevel`, `HookType`, `ManifestType`, `SecretType`
- **Core traits**: `KitProvider`, `VaultStore`, `Secret`, `ManifestProvider`
- **Error types**: `GenesisError` and `Result<T>`
- **Configuration types**: Provider configs, logging configs, etc.

## Features

- Validated environment names that enforce Genesis naming conventions
- Semantic versioning with comparison support
- Comprehensive error types with contextual information
- Trait-based abstractions for extensibility

## Usage

```rust
use genesis_types::{EnvName, SemVer, KitId};

// Create a validated environment name
let env = EnvName::new("us-west-prod")?;

// Parse semantic versions
let version = SemVer::parse("1.2.3")?;

// Create kit identifiers
let kit = KitId {
    name: "shield".to_string(),
    version,
};

println!("Deploying {} to {}", kit, env);
```

## Design Principles

1. **Type Safety**: Use the type system to prevent invalid states
2. **Zero-Cost Abstractions**: No runtime overhead for type wrappers
3. **Clear Errors**: Descriptive error messages with context
4. **Minimal Dependencies**: Keep this crate lightweight

## Dependencies

This crate has minimal dependencies:
- `serde` - Serialization support
- `thiserror` - Error type derivation
- `async-trait` - Async trait support

## See Also

- [genesis-core](../genesis-core) - Core utilities and configuration
- [genesis-env](../genesis-env) - Environment management
- [genesis-kit](../genesis-kit) - Kit handling
