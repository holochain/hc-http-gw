//! Configuration module for the HTTP Gateway.
//!
//! This module provides the configuration structure and related types for
//! controlling the behavior of the HTTP Gateway.

use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    str::FromStr,
};

use url2::Url2;

use crate::{HcHttpGatewayError, HcHttpGatewayResult};

/// Default payload size limit (10 kilobytes)
pub const DEFAULT_PAYLOAD_LIMIT_BYTES: u32 = 10 * 1024;

/// Main configuration structure for the HTTP Gateway.
#[derive(Debug, Clone)]
pub struct Configuration {
    /// WebSocket URL for admin connections and management interfaces
    pub admin_ws_url: Url2,
    /// Maximum size in bytes that request payloads can be
    pub payload_limit_bytes: u32,
    /// Controls which applications are permitted to connect to the gateway
    pub allowed_app_ids: AllowedAppIds,
    /// Maps application IDs to their allowed function configurations
    pub allowed_fns: HashMap<AppId, AllowedFns>,
}

impl Configuration {
    /// Creates a new `Configuration` by parsing and validating the provided string inputs.
    ///
    /// This constructor ensures that all components of the configuration are properly
    /// parsed and validated, including:
    /// * The admin WebSocket URL is a valid URL
    /// * The payload limit bytes can be parsed as a number
    /// * The app IDs are correctly parsed from a comma-separated string
    /// * Every app ID listed has a corresponding entry in the allowed_fns map
    pub fn try_new(
        admin_ws_url: &str,
        payload_limit_bytes: &str,
        allowed_app_ids: &str,
        allowed_fns: HashMap<AppId, AllowedFns>,
    ) -> HcHttpGatewayResult<Self> {
        let admin_ws_url = Url2::try_parse(admin_ws_url).map_err(|e| {
            HcHttpGatewayError::ConfigurationError(format!("Url parse error: {}", e))
        })?;

        let payload_limit_bytes = if payload_limit_bytes.is_empty() {
            DEFAULT_PAYLOAD_LIMIT_BYTES
        } else {
            payload_limit_bytes.parse::<u32>().map_err(|e| {
                HcHttpGatewayError::ConfigurationError(format!(
                    "Failed to parse the payload limit bytes value: {}",
                    e
                ))
            })?
        };

        let allowed_app_ids = AllowedAppIds::from_str(allowed_app_ids)?;

        for app_id in allowed_app_ids.iter() {
            if !allowed_fns.contains_key(app_id) {
                return Err(HcHttpGatewayError::ConfigurationError(format!(
                    "{} is not present in allowed_fns",
                    app_id
                )));
            }
        }

        Ok(Configuration {
            admin_ws_url,
            payload_limit_bytes,
            allowed_app_ids,
            allowed_fns,
        })
    }
}

/// Collection of app ids that are permitted to connect to the gateway
#[derive(Debug, Clone)]
pub struct AllowedAppIds(HashSet<AppId>);

impl Deref for AllowedAppIds {
    type Target = HashSet<AppId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for AllowedAppIds {
    type Err = HcHttpGatewayError;

    /// Expected format:
    /// - A comma separated string of allowed app_ids e.g "app1,app2,app3"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let allowed_app_ids = s
            .trim()
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect::<HashSet<_>>();

        Ok(Self(allowed_app_ids))
    }
}

impl Configuration {
    /// Check if the app_id is in the allowed list
    pub fn is_app_allowed(&self, app_id: &str) -> bool {
        self.allowed_app_ids.contains(&app_id.to_string())
    }

    /// Get the allowed functions for a given app_id
    pub fn get_allowed_functions(&self, app_id: &str) -> Option<&AllowedFns> {
        self.allowed_fns.get(app_id)
    }
}

/// Type alias for application identifiers.
pub type AppId = String;

/// Controls which functions can be called.
#[derive(Debug, Clone)]
pub enum AllowedFns {
    /// Only specific functions are allowed.
    Restricted(HashSet<ZomeFn>),

    /// All functions are allowed for all zomes.
    All,
}

/// Represents a function within a Holochain zome that can be called through the gateway
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ZomeFn {
    /// Name of the zome containing the function
    pub zome_name: String,
    /// Name of the specific function within the zome
    pub fn_name: String,
}

impl FromStr for AllowedFns {
    type Err = HcHttpGatewayError;

    /// Expected format
    /// - A comma separated string of zome_name/fn_name pairs, which should be separated
    ///   by a forward slash (/)
    /// - An asterix ("*") indicating that all functions in all zomes are allowed
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "*" => Ok(AllowedFns::All),
            s => {
                let csv = s.split(',');
                let mut zome_fns = HashSet::new();

                for zome_fn_path in csv {
                    let Some((zome_name, fn_name)) = zome_fn_path.trim().split_once('/') else {
                        return Err(HcHttpGatewayError::ConfigurationError(format!(
                            "Failed to parse the zome name and function name from value: {}",
                            zome_fn_path
                        )));
                    };

                    if zome_name.is_empty() || fn_name.is_empty() {
                        return Err(HcHttpGatewayError::ConfigurationError(format!(
                            "Zome name or function name is empty for value: {}",
                            zome_fn_path
                        )));
                    }

                    zome_fns.insert(ZomeFn {
                        zome_name: zome_name.to_string(),
                        fn_name: fn_name.to_string(),
                    });
                }

                Ok(AllowedFns::Restricted(zome_fns))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Helper function to create a ZomeFn
    fn create_zome_fn(zome_name: &str, fn_name: &str) -> ZomeFn {
        ZomeFn {
            zome_name: zome_name.to_string(),
            fn_name: fn_name.to_string(),
        }
    }

    // Helper function to create a test Configuration
    fn create_test_config() -> Configuration {
        let admin_ws_url = Url2::parse("ws://localhost:8888");

        let zome1_fn1 = create_zome_fn("zome1", "fn1");
        let app1_fns = HashSet::from([zome1_fn1.clone()]);

        let mut allowed_fns = HashMap::new();
        allowed_fns.insert("app1".to_string(), AllowedFns::Restricted(app1_fns));
        allowed_fns.insert("app2".to_string(), AllowedFns::All);

        Configuration {
            admin_ws_url,
            payload_limit_bytes: 1024 * 1024,
            allowed_app_ids: AllowedAppIds(HashSet::from(["app1".to_string(), "app2".to_string()])),
            allowed_fns,
        }
    }

    mod allowed_app_ids_tests {
        use super::*;

        #[test]
        fn from_str_parses_various_formats() {
            // Standard case
            let result = AllowedAppIds::from_str("app1,app2,app3").unwrap();
            assert_eq!(result.len(), 3);
            assert!(result.contains("app1"));

            // With whitespace
            let result = AllowedAppIds::from_str(" app1 , app2 , app3 ").unwrap();
            assert_eq!(result.len(), 3);

            // Empty entries
            let result = AllowedAppIds::from_str("app1,,app3").unwrap();
            assert_eq!(result.len(), 2);

            // Duplicate entries
            let result = AllowedAppIds::from_str("app1,app1,app2").unwrap();
            assert_eq!(result.len(), 2);
            assert!(result.contains("app1"));
            assert!(result.contains("app2"));

            // Empty string
            let result = AllowedAppIds::from_str("").unwrap();
            assert_eq!(result.len(), 0);
        }
    }

    mod allowed_fns_tests {
        use super::*;

        #[test]
        fn from_str_all_wildcard() {
            let result = AllowedFns::from_str("*").unwrap();
            assert!(matches!(result, AllowedFns::All));
        }

        #[test]
        fn from_str_parses_function_lists() {
            // Standard case
            let result = AllowedFns::from_str("zome1/fn1,zome2/fn2").unwrap();
            if let AllowedFns::Restricted(fns) = result {
                assert_eq!(fns.len(), 2);
                assert!(fns.contains(&create_zome_fn("zome1", "fn1")));
                assert!(fns.contains(&create_zome_fn("zome2", "fn2")));
            }

            // With whitespace
            let result = AllowedFns::from_str(" zome1/fn1 , zome2/fn2 ").unwrap();
            if let AllowedFns::Restricted(fns) = result {
                assert_eq!(fns.len(), 2);
            }

            // With duplicates
            let result = AllowedFns::from_str("zome1/fn1,zome1/fn1,zome2/fn2").unwrap();
            if let AllowedFns::Restricted(fns) = result {
                assert_eq!(fns.len(), 2);
            }
        }

        #[test]
        fn from_str_handles_errors() {
            // Missing zome
            let result = AllowedFns::from_str("/fn1");
            assert!(result.is_err());

            // Missing function
            let result = AllowedFns::from_str("zome1/");
            assert!(result.is_err());

            // Invalid format
            let result = AllowedFns::from_str("zome1");
            assert!(result.is_err());
        }
    }

    mod configuration_tests {
        use super::*;

        #[test]
        fn creation_sets_up_correct_fields() {
            let config = create_test_config();

            assert_eq!(config.admin_ws_url.to_string(), "ws://localhost:8888/");
            assert_eq!(config.payload_limit_bytes, 1024 * 1024);
            assert_eq!(config.allowed_app_ids.len(), 2);
        }

        #[test]
        fn is_app_allowed_checks_app_presence() {
            let config = create_test_config();

            assert!(config.is_app_allowed("app1"));
            assert!(config.is_app_allowed("app2"));
            assert!(!config.is_app_allowed("app3"));
            assert!(!config.is_app_allowed("APP1")); // Case sensitivity
        }

        #[test]
        fn get_allowed_functions_retrieves_functions() {
            let config = create_test_config();
            let zome1_fn1 = create_zome_fn("zome1", "fn1");

            // Test All variant
            assert!(matches!(
                config.get_allowed_functions("app2"),
                Some(AllowedFns::All)
            ));

            // Test Restricted variant
            if let Some(AllowedFns::Restricted(fns)) = config.get_allowed_functions("app1") {
                assert_eq!(fns.len(), 1);
                assert!(fns.contains(&zome1_fn1));
            } else {
                panic!("Expected Some(AllowedFns::Restricted)");
            }

            // Test non-existent app
            assert!(config.get_allowed_functions("app3").is_none());
        }

        #[test]
        fn new_constructs_valid_configuration() {
            // Setup allowed functions
            let mut allowed_fns = HashMap::new();
            allowed_fns.insert(
                "app1".to_string(),
                AllowedFns::Restricted(HashSet::from([create_zome_fn("zome1", "fn1")])),
            );
            allowed_fns.insert("app2".to_string(), AllowedFns::All);

            // Create configuration with valid inputs
            let config =
                Configuration::try_new("ws://localhost:8888", "1048576", "app1,app2", allowed_fns)
                    .unwrap();

            // Verify configuration
            assert_eq!(config.admin_ws_url.to_string(), "ws://localhost:8888/");
            assert_eq!(config.payload_limit_bytes, 1048576);
            assert_eq!(config.allowed_app_ids.len(), 2);
        }

        #[test]
        fn new_handles_invalid_inputs() {
            let mut allowed_fns = HashMap::new();
            allowed_fns.insert("app1".to_string(), AllowedFns::All);

            // Invalid URL
            let result =
                Configuration::try_new("not-a-valid-url", "1048576", "app1", allowed_fns.clone());
            assert!(result.is_err());

            // Invalid payload limit
            let result = Configuration::try_new(
                "ws://localhost:8888",
                "not-a-number",
                "app1",
                allowed_fns.clone(),
            );
            assert!(result.is_err());

            // Missing allowed function for app2
            let result =
                Configuration::try_new("ws://localhost:8888", "1048576", "app1,app2", allowed_fns);
            assert!(result.is_err());
        }
    }
}
