//! Configuration module for the HTTP Gateway.
//!
//! This module provides the configuration structure and related types for
//! controlling the behavior of the HTTP Gateway.

use std::{collections::HashMap, str::FromStr};

use url2::Url2;

use crate::HcHttpGatewayError;

/// Main configuration structure for the HTTP Gateway.
#[derive(Debug, Clone)]
pub struct Configuration {
    /// WebSocket URL for admin connections and management interfaces
    pub admin_ws_url: Url2,
    /// Maximum size in bytes that request payloads can be
    pub payload_limit_bytes: u16,
    /// Controls which applications are permitted to connect to the gateway
    pub allowed_app_ids: AllowedAppIds,
    /// Maps application IDs to their allowed function configurations
    pub allowed_fns: HashMap<AppId, AllowedFns>,
}

/// Type alias for application identifiers.
pub type AppId = String;

/// Controls which applications are allowed to connect.
#[derive(Debug, Clone)]
pub enum AllowedAppIds {
    /// Only specific applications are allowed.
    Restricted(Vec<AppId>),

    /// All applications are allowed.
    All,
}

impl FromStr for AllowedAppIds {
    type Err = HcHttpGatewayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "*" => Ok(AllowedAppIds::All),
            s => {
                if s.is_empty() {
                    return Err(HcHttpGatewayError::ConfigurationError(
                        "Allowed AppIds cannot be empty".to_string(),
                    ));
                }
                let app_ids = s
                    .trim()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<String>>();
                Ok(AllowedAppIds::Restricted(app_ids))
            }
        }
    }
}

/// Controls which functions can be called.
#[derive(Debug, Clone)]
pub enum AllowedFns {
    /// Only specific functions are allowed.
    Restricted(Vec<ZomeFn>),

    /// All functions are allowed for all applications.
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

impl FromStr for AllowedFns {
    type Err = HcHttpGatewayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "*" => Ok(AllowedFns::All),
            s => {
                let csv = s.split(',');
                let mut zome_fns = Vec::new();

                for fns in csv {
                    let mut fns = fns.trim().split('/');
                    let zome_name = fns.next().ok_or_else(|| {
                        HcHttpGatewayError::ConfigurationError(format!(
                            "Failed to parse zome name from: {}",
                            s
                        ))
                    })?;
                    let fn_name = fns.next().ok_or_else(|| {
                        HcHttpGatewayError::ConfigurationError(format!(
                            "Failed to parse zome fn from: {}",
                            s
                        ))
                    })?;

                    if zome_name.is_empty() || fn_name.is_empty() {
                        return Err(HcHttpGatewayError::ConfigurationError(format!(
                            "Zome name or Fn name is empty for: {:?}",
                            fns
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
    fn test_allowed_app_ids_from_str_all() {
        let result = AllowedAppIds::from_str("*").unwrap();
        assert!(matches!(result, AllowedAppIds::All));
    }

    #[test]
    fn test_allowed_app_ids_from_str_restricted() {
        let result = AllowedAppIds::from_str("app1,app2,app3").unwrap();
        if let AllowedAppIds::Restricted(apps) = result {
            assert_eq!(apps.len(), 3);
            assert_eq!(apps[0], "app1");
            assert_eq!(apps[1], "app2");
            assert_eq!(apps[2], "app3");
        } else {
            panic!("Expected AllowedAppIds::Restricted");
        }
    }

    #[test]
    fn test_allowed_app_ids_from_str_with_whitespace() {
        let result = AllowedAppIds::from_str(" app1 , app2 , app3 ").unwrap();
        if let AllowedAppIds::Restricted(apps) = result {
            assert_eq!(apps.len(), 3);
            assert_eq!(apps[0], "app1");
            assert_eq!(apps[1], "app2");
            assert_eq!(apps[2], "app3");
        } else {
            panic!("Expected AllowedAppIds::Restricted");
        }
    }

    #[test]
    fn test_allowed_app_ids_from_str_empty() {
        let result = AllowedAppIds::from_str("");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(HcHttpGatewayError::ConfigurationError(_))
        ));
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
    fn test_configuration_creation() {
        let admin_ws_url = Url2::parse("ws://localhost:8888");
        let allowed_app_ids =
            AllowedAppIds::Restricted(vec!["app1".to_string(), "app2".to_string()]);

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
            payload_limit_bytes: 1024,
            allowed_app_ids,
            allowed_fns,
        };

        assert_eq!(config.admin_ws_url.to_string(), "ws://localhost:8888/");
        assert_eq!(config.payload_limit_bytes, 1024);

        if let AllowedAppIds::Restricted(apps) = &config.allowed_app_ids {
            assert_eq!(apps.len(), 2);
            assert!(apps.contains(&"app1".to_string()));
            assert!(apps.contains(&"app2".to_string()));
        } else {
            panic!("Expected AllowedAppIds::Restricted");
        }

        assert_eq!(config.allowed_fns.len(), 2);

        if let AllowedFns::Restricted(fns) = &config.allowed_fns["app1"] {
            assert_eq!(fns.len(), 1);
            assert_eq!(fns[0].zome_name, "zome1");
            assert_eq!(fns[0].fn_name, "fn1");
        } else {
            panic!("Expected AllowedFns::Restricted");
        }

        assert!(matches!(config.allowed_fns["app2"], AllowedFns::All));
    }
}
