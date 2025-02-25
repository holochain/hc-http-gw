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
        match s {
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
        match s {
            "*" => Ok(AllowedFns::All),
            s => {
                let csv = s.trim().split(',');
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
