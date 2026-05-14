//! Build and runtime metadata helpers.

use std::fmt;

/// Product name used in user-agent metadata.
pub const PRODUCT_NAME: &str = "victors-lambdas";

/// Crate version used in user-agent metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runtime identifier for user-agent metadata.
pub const RUNTIME: &str = "rust";

/// Product token used in user-agent metadata.
///
/// This value is useful for SDK configuration surfaces that accept a product token without
/// comments.
pub const USER_AGENT_PRODUCT: &str = concat!("victors-lambdas/", env!("CARGO_PKG_VERSION"));

/// Complete user-agent value.
///
/// This value is dependency-free so utility crates can pass it into AWS SDK client configuration
/// without depending on SDK-specific types.
pub const USER_AGENT: &str = concat!("victors-lambdas/", env!("CARGO_PKG_VERSION"), " (rust)");

/// Default metadata for Victor's Lambdas.
pub const DEFAULT_METADATA: Metadata = Metadata {
    product_name: PRODUCT_NAME,
    version: VERSION,
    runtime: RUNTIME,
};

/// Build and runtime metadata used to format user-agent values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metadata {
    product_name: &'static str,
    version: &'static str,
    runtime: &'static str,
}

impl Metadata {
    /// Returns the product name.
    #[must_use]
    pub const fn product_name(self) -> &'static str {
        self.product_name
    }

    /// Returns the crate version.
    #[must_use]
    pub const fn version(self) -> &'static str {
        self.version
    }

    /// Returns the runtime identifier.
    #[must_use]
    pub const fn runtime(self) -> &'static str {
        self.runtime
    }

    /// Builds the `name/version` user-agent product token.
    #[must_use]
    pub fn user_agent_product(self) -> String {
        format!("{}/{}", self.product_name, self.version)
    }

    /// Builds the complete user-agent value.
    #[must_use]
    pub fn user_agent(self) -> String {
        format!("{}/{} ({})", self.product_name, self.version, self.runtime)
    }
}

impl Default for Metadata {
    fn default() -> Self {
        DEFAULT_METADATA
    }
}

impl fmt::Display for Metadata {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{} ({})",
            self.product_name, self.version, self.runtime
        )
    }
}

/// Returns the default Victor's Lambdas metadata.
#[must_use]
pub const fn default_metadata() -> Metadata {
    DEFAULT_METADATA
}

/// Returns the `name/version` user-agent product token.
#[must_use]
pub const fn user_agent_product() -> &'static str {
    USER_AGENT_PRODUCT
}

/// Builds the complete user-agent value.
#[must_use]
pub fn user_agent() -> String {
    USER_AGENT.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_expose_current_metadata() {
        assert_eq!(PRODUCT_NAME, "victors-lambdas");
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert_eq!(RUNTIME, "rust");
        assert_eq!(USER_AGENT_PRODUCT, format!("{PRODUCT_NAME}/{VERSION}"));
        assert_eq!(USER_AGENT, format!("{PRODUCT_NAME}/{VERSION} ({RUNTIME})"));
    }

    #[test]
    fn default_metadata_exposes_components() {
        let metadata = default_metadata();

        assert_eq!(metadata, DEFAULT_METADATA);
        assert_eq!(metadata, Metadata::default());
        assert_eq!(metadata.product_name(), PRODUCT_NAME);
        assert_eq!(metadata.version(), VERSION);
        assert_eq!(metadata.runtime(), RUNTIME);
    }

    #[test]
    fn metadata_builds_user_agent_values() {
        let metadata = Metadata {
            product_name: "custom-lambdas",
            version: "1.2.3",
            runtime: "rust",
        };

        assert_eq!(metadata.user_agent_product(), "custom-lambdas/1.2.3");
        assert_eq!(metadata.user_agent(), "custom-lambdas/1.2.3 (rust)");
        assert_eq!(metadata.to_string(), "custom-lambdas/1.2.3 (rust)");
    }

    #[test]
    fn user_agent_helpers_match_default_metadata() {
        assert_eq!(user_agent_product(), DEFAULT_METADATA.user_agent_product());
        assert_eq!(user_agent(), DEFAULT_METADATA.user_agent());
        assert_eq!(user_agent(), USER_AGENT);
    }
}
