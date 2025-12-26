//! Secret generation utilities.

use genesis_types::{Result};
use crate::plan::SecretPlan;

/// Secret generator.
pub struct SecretGenerator;

impl SecretGenerator {
    /// Generate all missing secrets in a plan.
    pub async fn generate_all(plan: &SecretPlan) -> Result<Vec<String>> {
        plan.generate_missing().await
    }

    /// Generate specific secrets by path.
    pub async fn generate_paths(
        plan: &SecretPlan,
        paths: &[String],
    ) -> Result<Vec<String>> {
        plan.rotate(paths).await
    }
}
