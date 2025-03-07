use holochain_client::AppInfo;
use holochain_types::dna::DnaHashB64;
use thiserror::Error;

use crate::config::AllowedAppIds;

#[mockall_double::double]
pub(crate) use admin_websocket::AdminWebsocket;

mod admin_websocket {
    use super::*;

    /// Fake AdminWebsocket until https://github.com/holochain/hc-http-gw/issues/11 is done.
    #[cfg(not(test))]
    #[derive(Debug, Clone)]
    pub struct AdminWebsocket;

    #[cfg(not(test))]
    impl AdminWebsocket {
        pub fn new() -> Self {
            Self {}
        }

        pub fn list_apps(&self) -> Vec<AppInfo> {
            use holochain_client::AgentPubKey;
            use holochain_types::app::{
                AppManifest, AppRoleDnaManifest, AppRoleManifest, AppStatus,
            };
            use std::collections::HashMap;

            vec![
                AppInfo {
                    installed_app_id: "app1".to_string(),
                    cell_info: HashMap::new(),
                    status: AppStatus::Running.into(),
                    agent_pub_key: AgentPubKey::from_raw_39(vec![
                        132, 32, 36, 54, 1, 132, 0, 1, 0, 99, 255, 122, 1, 0, 1, 255, 1, 0, 106,
                        46, 186, 188, 245, 255, 0, 121, 188, 1, 239, 235, 123, 0, 169, 19, 0, 136,
                        254, 243, 140,
                    ])
                    .unwrap(),
                    manifest: AppManifest::V1(holochain_types::app::AppManifestV1 {
                        name: Default::default(),
                        description: Default::default(),
                        roles: vec![AppRoleManifest {
                            name: Default::default(),
                            provisioning: Default::default(),
                            dna: AppRoleDnaManifest {
                                location: Default::default(),
                                modifiers: Default::default(),
                                installed_hash: Some(
                                    DnaHashB64::from_b64_str(
                                        "uhC0kVKaUAQEBAP8BHVUAAAGeFP8LzP8BAQEB3__4_9EAAAD9L-hZ",
                                    )
                                    .unwrap(),
                                ),
                                clone_limit: Default::default(),
                            },
                        }],
                        allow_deferred_memproofs: Default::default(),
                    }),
                },
                AppInfo {
                    installed_app_id: "app2".to_string(),
                    cell_info: HashMap::new(),
                    status: AppStatus::Running.into(),
                    agent_pub_key: AgentPubKey::from_raw_39(vec![
                        132, 32, 36, 54, 1, 132, 0, 1, 0, 99, 255, 122, 1, 0, 1, 255, 1, 0, 106,
                        46, 186, 188, 245, 255, 0, 121, 188, 1, 239, 235, 123, 0, 169, 19, 0, 136,
                        254, 243, 140,
                    ])
                    .unwrap(),
                    manifest: AppManifest::V1(holochain_types::app::AppManifestV1 {
                        name: Default::default(),
                        description: Default::default(),
                        roles: vec![AppRoleManifest {
                            name: Default::default(),
                            provisioning: Default::default(),
                            dna: AppRoleDnaManifest {
                                location: Default::default(),
                                modifiers: Default::default(),
                                installed_hash: Some(
                                    DnaHashB64::from_b64_str(
                                        "uhC0k9gEAAf90Af9kZ0OfAQH_egrtAf20YBMB_w0B6gCEo_8k8aBt",
                                    )
                                    .unwrap(),
                                ),
                                clone_limit: Default::default(),
                            },
                        }],
                        allow_deferred_memproofs: Default::default(),
                    }),
                },
            ]
        }
    }

    #[cfg(test)]
    mockall::mock! {
        #[derive(Debug)]
        pub AdminWebsocket {
            pub fn list_apps(&self) -> Vec<AppInfo>;
        }
        impl Clone for AdminWebsocket {
            fn clone(&self) -> Self;
        }
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum AppSelectionError {
    #[error("App is not installed on the conductor")]
    NotInstalled,

    #[error("App is not in the list of allowed apps")]
    NotAllowed,
}

fn find_installed_app<'a>(
    dna_hash: &DnaHashB64,
    installed_apps: &'a [AppInfo],
) -> Option<&'a AppInfo> {
    installed_apps.iter().find(|a| {
        a.manifest.app_roles().iter().any(|r| {
            r.dna
                .installed_hash
                .as_ref()
                .is_some_and(|hash| hash == dna_hash)
        })
    })
}

pub fn check_app_valid(
    dna_hash: DnaHashB64,
    installed_apps: &mut Vec<AppInfo>,
    allowed_apps: &AllowedAppIds,
    admin_websocket: &AdminWebsocket,
) -> Result<(), AppSelectionError> {
    let app_info = if let Some(app_info) = find_installed_app(&dna_hash, installed_apps) {
        app_info
    } else {
        *installed_apps = admin_websocket.list_apps();
        find_installed_app(&dna_hash, installed_apps).ok_or(AppSelectionError::NotInstalled)?
    };

    allowed_apps
        .contains(&app_info.installed_app_id)
        .then_some(())
        .ok_or(AppSelectionError::NotAllowed)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use super::*;
    use assert2::assert;
    use fixt::fixt;
    use holochain_types::{
        app::{AppManifest, AppRoleDnaManifest, AppRoleManifest, AppStatus},
        prelude::fixt::*,
    };

    fn new_fake_app_info(app_id: impl ToString, dna_hash: DnaHashB64) -> AppInfo {
        AppInfo {
            installed_app_id: app_id.to_string(),
            cell_info: HashMap::new(),
            status: AppStatus::Running.into(),
            agent_pub_key: fixt!(AgentPubKey),
            manifest: AppManifest::V1(holochain_types::app::AppManifestV1 {
                name: Default::default(),
                description: Default::default(),
                roles: vec![AppRoleManifest {
                    name: Default::default(),
                    provisioning: Default::default(),
                    dna: AppRoleDnaManifest {
                        location: Default::default(),
                        modifiers: Default::default(),
                        installed_hash: Some(dna_hash),
                        clone_limit: Default::default(),
                    },
                }],
                allow_deferred_memproofs: Default::default(),
            }),
        }
    }

    #[test]
    fn returns_error_if_app_not_installed() {
        let dna_hash = fixt!(DnaHashB64);
        let mut installed_apps = Vec::new();
        let allowed_apps = AllowedAppIds::from_str("").unwrap();
        let mut admin_websocket = AdminWebsocket::new();
        admin_websocket
            .expect_list_apps()
            .return_const(Vec::new())
            .once();

        let result = check_app_valid(
            dna_hash,
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        );

        assert!(result == Err(AppSelectionError::NotInstalled));
    }

    #[test]
    fn returns_error_if_app_installed_but_not_allowed() {
        let dna_hash = fixt!(DnaHashB64);
        let mut installed_apps = vec![new_fake_app_info("some_app_id", dna_hash.clone())];
        let allowed_apps = AllowedAppIds::from_str("other_app_id").unwrap();
        let admin_websocket = AdminWebsocket::new();

        let result = check_app_valid(
            dna_hash,
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        );

        assert!(result == Err(AppSelectionError::NotAllowed));
    }

    #[test]
    fn returns_ok_if_app_is_installed_and_allowed() {
        let dna_hash = fixt!(DnaHashB64);
        let mut installed_apps = vec![new_fake_app_info("some_app_id", dna_hash.clone())];
        let allowed_apps = AllowedAppIds::from_str("some_app_id").unwrap();
        let admin_websocket = AdminWebsocket::new();

        let result = check_app_valid(
            dna_hash,
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        );

        assert!(result == Ok(()));
    }

    #[test]
    fn checks_app_list_from_websocket_if_not_in_installed_apps() {
        let dna_hash = fixt!(DnaHashB64);
        let mut installed_apps = Vec::new();
        let allowed_apps = AllowedAppIds::from_str("some_app_id").unwrap();
        let mut admin_websocket = AdminWebsocket::new();
        admin_websocket
            .expect_list_apps()
            .return_const(vec![new_fake_app_info("some_app_id", dna_hash.clone())])
            .once();

        let result = check_app_valid(
            dna_hash,
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        );

        assert!(result == Ok(()));
    }
}
