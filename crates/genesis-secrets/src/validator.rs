//! Secret validation utilities.

use genesis_types::{Result};
use genesis_types::traits::ValidationResult;
use crate::plan::SecretPlan;
use std::collections::HashMap;

/// Secret validator.
pub struct SecretValidator;

impl SecretValidator {
    /// Validate all secrets in a plan.
    pub async fn validate_all(
        plan: &SecretPlan,
    ) -> Result<HashMap<String, ValidationResult>> {
        plan.validate().await
    }

    /// Check which secrets exist.
    pub async fn check_exists(
        plan: &SecretPlan,
    ) -> Result<HashMap<String, bool>> {
        plan.check().await
    }

    /// Get summary of validation results.
    pub async fn summary(
        plan: &SecretPlan,
    ) -> Result<ValidationSummary> {
        let results = plan.validate().await?;

        let mut summary = ValidationSummary::default();

        for (path, result) in results {
            match result {
                ValidationResult::Ok => summary.ok.push(path),
                ValidationResult::Missing => summary.missing.push(path),
                ValidationResult::Warning(warnings) => {
                    summary.warnings.push((path, warnings));
                }
                ValidationResult::Error(errors) => {
                    summary.errors.push((path, errors));
                }
            }
        }

        Ok(summary)
    }
}

/// Summary of validation results.
#[derive(Debug, Default)]
pub struct ValidationSummary {
    /// Secrets that are valid
    pub ok: Vec<String>,
    /// Secrets that are missing
    pub missing: Vec<String>,
    /// Secrets with warnings (path, warnings)
    pub warnings: Vec<(String, Vec<String>)>,
    /// Secrets with errors (path, errors)
    pub errors: Vec<(String, Vec<String>)>,
}

impl ValidationSummary {
    /// Check if all secrets are valid.
    pub fn is_all_valid(&self) -> bool {
        self.missing.is_empty() && self.errors.is_empty()
    }

    /// Get total count of secrets.
    pub fn total(&self) -> usize {
        self.ok.len() + self.missing.len() + self.warnings.len() + self.errors.len()
    }
}
