//! Configuration module for the HTTP Gateway.
//!
//! This module provides the configuration structure and related types for
//! controlling the behavior of the HTTP Gateway.

use std::net::SocketAddr;
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    str::FromStr,
};

/// Default payload size limit (10 kilobytes)
pub const DEFAULT_PAYLOAD_LIMIT_BYTES: u32 = 10 * 1024;

/// Default maximum number of app connections that the gateway will maintain concurrently.
pub const DEFAULT_MAX_APP_CONNECTIONS: u32 = 50;

/// Default timeout for zome calls
pub const DEFAULT_ZOME_CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Errors when parsing config arguments.
#[derive(Debug, thiserror::Error)]
pub enum ConfigParseError {
    /// Error when parsing an integer.
    #[error("Integer parse error: {0}")]
    IntParseError(#[from] std::num::ParseIntError),
    /// Other parsing error.
    #[error("Parse error: {0}")]
    Other(String),
}

/// Result of parsing config arguments.
pub type ConfigParseResult<T> = Result<T, ConfigParseError>;

/// Main configuration structure for the HTTP Gateway.
#[derive(Debug, Clone)]
pub struct Configuration {
    /// WebSocket URL for admin connections and management interfaces
    pub admin_socket_addr: SocketAddr,
    /// Maximum size in bytes that request payloads can be
    pub payload_limit_bytes: u32,
    /// Controls which applications are permitted to connect to the gateway
    pub allowed_app_ids: AllowedAppIds,
    /// Maps application IDs to their allowed function configurations
    pub allowed_fns: HashMap<AppId, AllowedFns>,
    /// Maximum number of app connections that the gateway will maintain concurrently.
    pub max_app_connections: u32,
    /// Timeout for zome calls
    pub zome_call_timeout: std::time::Duration,
}

impl Configuration {
    /// Creates a new [`Configuration`] by parsing and validating the provided string inputs.
    ///
    /// This constructor ensures that all components of the configuration are properly
    /// parsed and validated, including:
    /// * The payload limit bytes can be parsed as a number
    /// * The allowed app IDs are correctly parsed from a comma-separated string
    /// * Every app ID listed has a corresponding entry in the allowed_fns map
    /// * The max app connections can be parsed as a number
    /// * The zome call timeout can be parsed as a number
    pub fn try_new(
        admin_socket_addr: SocketAddr,
        payload_limit_bytes: &str,
        allowed_app_ids: &str,
        allowed_fns: HashMap<AppId, AllowedFns>,
        max_app_connections: &str,
        zome_call_timeout: &str,
    ) -> ConfigParseResult<Self> {
        let payload_limit_bytes = if payload_limit_bytes.is_empty() {
            DEFAULT_PAYLOAD_LIMIT_BYTES
        } else {
            payload_limit_bytes.parse::<u32>()?
        };

        let allowed_app_ids = AllowedAppIds::from_str(allowed_app_ids)?;

        for app_id in allowed_app_ids.iter() {
            if !allowed_fns.contains_key(app_id) {
                return Err(ConfigParseError::Other(format!(
                    "{} is not present in allowed_fns",
                    app_id
                )));
            }
        }

        let max_app_connections = if max_app_connections.is_empty() {
            DEFAULT_MAX_APP_CONNECTIONS
        } else {
            max_app_connections.parse::<u32>()?
        };

        let zome_call_timeout = if zome_call_timeout.is_empty() {
            DEFAULT_ZOME_CALL_TIMEOUT
        } else {
            std::time::Duration::from_millis(zome_call_timeout.parse::<u64>()?)
        };

        Ok(Configuration {
            admin_socket_addr,
            payload_limit_bytes,
            allowed_app_ids,
            allowed_fns,
            max_app_connections,
            zome_call_timeout,
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
    type Err = ConfigParseError;

    /// Expected format:
    /// - A comma separated string of allowed app_ids e.g "app1,app2,app3"
    fn from_str(s: &str) -> ConfigParseResult<Self> {
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

    /// Check if a function of an app is allowed
    pub fn is_function_allowed(&self, app_id: &str, zome_name: &str, fn_name: &str) -> bool {
        match self.get_allowed_functions(app_id) {
            None => false,
            Some(allowed_fns) => match allowed_fns {
                AllowedFns::All => true,
                AllowedFns::Restricted(zome_fns) => {
                    let zome_fn = ZomeFn {
                        zome_name: zome_name.to_string(),
                        fn_name: fn_name.to_string(),
                    };
                    zome_fns.contains(&zome_fn)
                }
            },
        }
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
    type Err = ConfigParseError;

    /// Expected format
    /// - A comma separated string of zome_name/fn_name pairs, which should be separated
    ///   by a forward slash (/)
    /// - An asterix ("*") indicating that all functions in all zomes are allowed
    fn from_str(s: &str) -> ConfigParseResult<Self> {
        match s.trim() {
            "*" => Ok(AllowedFns::All),
            s => {
                let csv = s.split(',');
                let mut zome_fns = HashSet::new();

                for zome_fn_path in csv {
                    let Some((zome_name, fn_name)) = zome_fn_path.trim().split_once('/') else {
                        return Err(ConfigParseError::Other(format!(
                            "Failed to parse the zome name and function name from value: {}",
                            zome_fn_path,
                        )));
                    };

                    if zome_name.is_empty() || fn_name.is_empty() {
                        return Err(ConfigParseError::Other(format!(
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
    use std::net::Ipv4Addr;
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
        let zome1_fn1 = create_zome_fn("zome1", "fn1");
        let app1_fns = HashSet::from([zome1_fn1.clone()]);

        let mut allowed_fns = HashMap::new();
        allowed_fns.insert("app1".to_string(), AllowedFns::Restricted(app1_fns));
        allowed_fns.insert("app2".to_string(), AllowedFns::All);

        Configuration {
            admin_socket_addr: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
            payload_limit_bytes: 1024 * 1024,
            allowed_app_ids: AllowedAppIds(HashSet::from(["app1".to_string(), "app2".to_string()])),
            allowed_fns,
            max_app_connections: DEFAULT_MAX_APP_CONNECTIONS,
            zome_call_timeout: DEFAULT_ZOME_CALL_TIMEOUT,
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
        use std::net::Ipv4Addr;

        #[test]
        fn creation_sets_up_correct_fields() {
            let config = create_test_config();

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
        fn is_function_allowed_returns_false_when_app_is_not_found() {
            let config = create_test_config();
            assert!(!config.is_function_allowed("nopp", "zome_name", "fn_name"));
        }

        #[test]
        fn is_function_allowed_returns_true_when_all_functions_allowed_for_app() {
            let config = create_test_config();
            assert!(config.is_function_allowed("app2", "zome_name", "fn_name"),);
        }

        #[test]
        fn is_function_allowed_returns_false_when_zome_not_found() {
            let config = create_test_config();
            assert!(!config.is_function_allowed("app1", "not_included_zome", "fn_name"),);
        }

        #[test]
        fn is_function_allowed_returns_false_when_function_not_in_restricted_functions() {
            let config = create_test_config();
            assert!(!config.is_function_allowed("app1", "zome1", "not_included"),);
        }

        #[test]
        fn is_function_allowed_returns_true_when_function_in_restricted_functions() {
            let config = create_test_config();
            assert!(config.is_function_allowed("app1", "zome1", "fn1"));
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
            let config = Configuration::try_new(
                SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
                "1048576",
                "app1,app2",
                allowed_fns,
                "50",
                "1000",
            )
            .unwrap();

            // Verify configuration
            assert_eq!(config.payload_limit_bytes, 1048576);
            assert_eq!(config.allowed_app_ids.len(), 2);
            assert_eq!(config.max_app_connections, 50);
            assert_eq!(config.zome_call_timeout.as_millis(), 1000);
        }

        #[test]
        fn new_handles_invalid_inputs() {
            let mut allowed_fns = HashMap::new();
            allowed_fns.insert("app1".to_string(), AllowedFns::All);

            // Invalid payload limit
            let result = Configuration::try_new(
                SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
                "not-a-number",
                "app1",
                allowed_fns.clone(),
                "",
                "",
            );
            assert!(result.is_err());

            // Missing allowed function for app2
            let result = Configuration::try_new(
                SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
                "1048576",
                "app1,app2",
                allowed_fns.clone(),
                "",
                "",
            );
            assert!(result.is_err());

            // Max app connections is not a valid number
            let result = Configuration::try_new(
                SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
                "1048576",
                "app1,app2",
                allowed_fns.clone(),
                "not-a-number",
                "",
            );
            assert!(result.is_err());

            // Zome call timeout is not a valid number
            let result = Configuration::try_new(
                SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
                "1048576",
                "app1,app2",
                allowed_fns,
                "",
                "not-a-number",
            );
            assert!(result.is_err());
        }
    }
}
