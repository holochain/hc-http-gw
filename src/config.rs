//! Configuration module for the HTTP Gateway.
//!
//! This module provides the configuration structure and related types for
//! controlling the behavior of the HTTP Gateway.

use std::{collections::HashMap, ops::Deref, str::FromStr};

use url2::Url2;

use crate::HcHttpGatewayError;

/// Default payload size limit (10 kilobytes)
pub const DEFAULT_PAYLOAD_LIMIT_BYTES: u32 = 10 * 1024;

/// Main configuration structure for the HTTP Gateway.
#[derive(Debug, Clone)]
pub struct Configuration {
    /// WebSocket URL for admin connections and management interfaces
    pub admin_ws_url: Url2,
    /// Maximum size in bytes that request payloads can be
    pub payload_limit_bytes: PayloadLimitBytes,
    /// Controls which applications are permitted to connect to the gateway
    pub allowed_app_ids: AllowedAppIds,
    /// Maps application IDs to their allowed function configurations
    pub allowed_fns: HashMap<AppId, AllowedFns>,
}

/// Collection of app ids that are permitted to connect to the gateway
#[derive(Debug, Clone)]
pub struct AllowedAppIds(Vec<AppId>);

impl Deref for AllowedAppIds {
    type Target = Vec<AppId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Expected format:
// - A comma separated string of allowed app_ids e.g "app1,app2,app3"
impl FromStr for AllowedAppIds {
    type Err = HcHttpGatewayError;

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
            .collect::<Vec<_>>();

        Ok(Self(allowed_app_ids))
    }
}

/// Maximum size in bytes that zome call payloads can be.
#[derive(Debug, Clone)]
pub struct PayloadLimitBytes(u32);

impl Deref for PayloadLimitBytes {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for PayloadLimitBytes {
    fn default() -> Self {
        Self(DEFAULT_PAYLOAD_LIMIT_BYTES)
    }
}

impl FromStr for PayloadLimitBytes {
    type Err = HcHttpGatewayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let payload_limit_bytes = if s.is_empty() {
            return Ok(Self::default());
        } else {
            s.parse::<u32>().map_err(|e| {
                HcHttpGatewayError::ConfigurationError(format!(
                    "Failed to parse the payload limit bytes value: {}",
                    e
                ))
            })?
        };

        Ok(Self(payload_limit_bytes))
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
    Restricted(Vec<ZomeFn>),

    /// All functions are allowed for all zomes.
    All,
}

/// Represents a function within a Holochain zome that can be called through the gateway
#[derive(Debug, Clone)]
pub struct ZomeFn {
    /// Name of the zome containing the function
    pub zome_name: String,
    /// Name of the specific function within the zome
    pub fn_name: String,
}

// Expected format
// - A comma separated string of zome_name/fn_name pairs, which should be separated
//   by a forward slash (/)
// - An asterix ("*") indicating that all functions in all zomes are allowed
impl FromStr for AllowedFns {
    type Err = HcHttpGatewayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "*" => Ok(AllowedFns::All),
            s => {
                let csv = s.split(',');
                let mut zome_fns = Vec::new();

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

                    zome_fns.push(ZomeFn {
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

    #[test]
    fn test_allowed_app_ids_from_str() {
        let result = AllowedAppIds::from_str("app1,app2,app3").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "app1");
        assert_eq!(result[1], "app2");
        assert_eq!(result[2], "app3");
    }

    #[test]
    fn test_allowed_app_ids_from_str_with_whitespace() {
        let result = AllowedAppIds::from_str(" app1 , app2 , app3 ").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "app1");
        assert_eq!(result[1], "app2");
        assert_eq!(result[2], "app3");
    }

    #[test]
    fn test_allowed_app_ids_from_str_empty_entries() {
        let result = AllowedAppIds::from_str("app1,,app3").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "app1");
        assert_eq!(result[1], "app3");
    }

    #[test]
    fn test_allowed_app_ids_from_str_empty_string() {
        let result = AllowedAppIds::from_str("").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_allowed_fns_from_str_all() {
        let result = AllowedFns::from_str("*").unwrap();
        assert!(matches!(result, AllowedFns::All));
    }

    #[test]
    fn test_allowed_fns_from_str_restricted() {
        let result = AllowedFns::from_str("zome1/fn1,zome2/fn2").unwrap();
        if let AllowedFns::Restricted(fns) = result {
            assert_eq!(fns.len(), 2);
            assert_eq!(fns[0].zome_name, "zome1");
            assert_eq!(fns[0].fn_name, "fn1");
            assert_eq!(fns[1].zome_name, "zome2");
            assert_eq!(fns[1].fn_name, "fn2");
        } else {
            panic!("Expected AllowedFns::Restricted");
        }
    }

    #[test]
    fn test_allowed_fns_from_str_with_whitespace() {
        let result = AllowedFns::from_str(" zome1/fn1 , zome2/fn2 ").unwrap();
        if let AllowedFns::Restricted(fns) = result {
            assert_eq!(fns.len(), 2);
            assert_eq!(fns[0].zome_name, "zome1");
            assert_eq!(fns[0].fn_name, "fn1");
            assert_eq!(fns[1].zome_name, "zome2");
            assert_eq!(fns[1].fn_name, "fn2");
        } else {
            panic!("Expected AllowedFns::Restricted");
        }
    }

    #[test]
    fn test_allowed_fns_from_str_missing_zome() {
        let result = AllowedFns::from_str("/fn1");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(HcHttpGatewayError::ConfigurationError(_))
        ));
    }

    #[test]
    fn test_allowed_fns_from_str_missing_fn() {
        let result = AllowedFns::from_str("zome1/");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(HcHttpGatewayError::ConfigurationError(_))
        ));
    }

    #[test]
    fn test_allowed_fns_from_str_invalid_format() {
        let result = AllowedFns::from_str("zome1");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(HcHttpGatewayError::ConfigurationError(_))
        ));
    }

    #[test]
    fn test_payload_limit_bytes_from_str() {
        // Test successful parsing
        let result = PayloadLimitBytes::from_str("1048576").unwrap();
        assert_eq!(*result, 1048576);

        // Test parsing with invalid input
        let result = PayloadLimitBytes::from_str("not a number");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(HcHttpGatewayError::ConfigurationError(_))
        ));
    }

    #[test]
    fn test_configuration_creation() {
        let admin_ws_url = Url2::parse("ws://localhost:8888");
        let allowed_app_ids = AllowedAppIds(vec!["app1".to_string(), "app2".to_string()]);
        let payload_limit_bytes = PayloadLimitBytes(1024 * 1024); // 1MB

        let mut allowed_fns = HashMap::new();
        allowed_fns.insert(
            "app1".to_string(),
            AllowedFns::Restricted(vec![ZomeFn {
                zome_name: "zome1".to_string(),
                fn_name: "fn1".to_string(),
            }]),
        );
        allowed_fns.insert("app2".to_string(), AllowedFns::All);

        let config = Configuration {
            admin_ws_url,
            payload_limit_bytes,
            allowed_app_ids,
            allowed_fns,
        };

        assert_eq!(config.admin_ws_url.to_string(), "ws://localhost:8888/");
        assert_eq!(*config.payload_limit_bytes, 1024 * 1024);

        // Test is_app_allowed method
        assert!(config.is_app_allowed("app1"));
        assert!(config.is_app_allowed("app2"));
        assert!(!config.is_app_allowed("app3"));

        // Test get_allowed_functions method
        assert!(config.get_allowed_functions("app1").is_some());
        assert!(config.get_allowed_functions("app2").is_some());
        assert!(config.get_allowed_functions("app3").is_none());

        if let Some(AllowedFns::Restricted(fns)) = config.get_allowed_functions("app1") {
            assert_eq!(fns.len(), 1);
            assert_eq!(fns[0].zome_name, "zome1");
            assert_eq!(fns[0].fn_name, "fn1");
        } else {
            panic!("Expected Some(AllowedFns::Restricted)");
        }

        if let Some(allowed_fns) = config.get_allowed_functions("app2") {
            assert!(matches!(allowed_fns, AllowedFns::All));
        } else {
            panic!("Expected Some(AllowedFns::All)");
        }
    }

    #[test]
    fn test_is_app_allowed() {
        let allowed_app_ids = AllowedAppIds(vec!["app1".to_string(), "app2".to_string()]);
        let config = Configuration {
            admin_ws_url: Url2::parse("ws://localhost:8888"),
            payload_limit_bytes: PayloadLimitBytes(1024),
            allowed_app_ids,
            allowed_fns: HashMap::new(),
        };

        assert!(config.is_app_allowed("app1"));
        assert!(config.is_app_allowed("app2"));
        assert!(!config.is_app_allowed("app3"));

        // Case sensitivity test
        assert!(!config.is_app_allowed("APP1"));
    }

    #[test]
    fn test_get_allowed_functions() {
        let mut allowed_fns = HashMap::new();
        allowed_fns.insert(
            "app1".to_string(),
            AllowedFns::Restricted(vec![ZomeFn {
                zome_name: "zome1".to_string(),
                fn_name: "fn1".to_string(),
            }]),
        );
        allowed_fns.insert("app2".to_string(), AllowedFns::All);

        let config = Configuration {
            admin_ws_url: Url2::parse("ws://localhost:8888"),
            payload_limit_bytes: PayloadLimitBytes(1024),
            allowed_app_ids: AllowedAppIds(vec!["app1".to_string(), "app2".to_string()]),
            allowed_fns,
        };

        // Test retrieving existing functions
        assert!(matches!(
            config.get_allowed_functions("app2"),
            Some(AllowedFns::All)
        ));

        // Test retrieving non-existent app
        assert!(config.get_allowed_functions("app3").is_none());

        // Test restricted functions
        if let Some(AllowedFns::Restricted(fns)) = config.get_allowed_functions("app1") {
            assert_eq!(fns.len(), 1);
            assert_eq!(fns[0].zome_name, "zome1");
            assert_eq!(fns[0].fn_name, "fn1");
        } else {
            panic!("Expected Some(AllowedFns::Restricted)");
        }
    }
}
