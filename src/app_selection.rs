use holochain_client::AppInfo;
use holochain_types::dna::DnaHashB64;
use thiserror::Error;

use crate::config::AllowedAppIds;

#[derive(Debug, PartialEq, Error)]
pub enum AppSelectionError {
    #[error("App is not installed on the conductor")]
    NotInstalled,

    #[error("App is not in the list of allowed apps")]
    NotAllowed,
}

fn check_app_valid(
    dna_hash: DnaHashB64,
    installed_apps: &[AppInfo],
    allowed_apps: &AllowedAppIds,
) -> Result<(), AppSelectionError> {
    let app_info = installed_apps
        .iter()
        .find(|a| {
            a.manifest.app_roles().iter().any(|r| {
                r.dna
                    .installed_hash
                    .as_ref()
                    .is_some_and(|hash| hash == &dna_hash)
            })
        })
        .ok_or(AppSelectionError::NotInstalled)?;

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
    use holochain::core::DnaHash;
    use holochain_client::AgentPubKey;
    use holochain_types::app::{AppManifest, AppRoleDnaManifest, AppRoleManifest, AppStatus};

    fn new_fake_app_info(app_id: impl ToString, dna_hash: DnaHashB64) -> AppInfo {
        AppInfo {
            installed_app_id: app_id.to_string(),
            cell_info: HashMap::new(),
            status: AppStatus::Running.into(),
            agent_pub_key: AgentPubKey::from_raw_32([1; 32].to_vec()),
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
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec()).into();
        let installed_apps = [];
        let allowed_apps = AllowedAppIds::from_str("").unwrap();

        let result = check_app_valid(dna_hash, &installed_apps, &allowed_apps);

        assert!(result == Err(AppSelectionError::NotInstalled));
    }

    #[test]
    fn returns_error_if_app_installed_but_not_allowed() {
        let dna_hash: DnaHashB64 = DnaHash::from_raw_32([1; 32].to_vec()).into();
        let installed_apps = [new_fake_app_info("some_app_id", dna_hash.clone())];
        let allowed_apps = AllowedAppIds::from_str("other_app_id").unwrap();

        let result = check_app_valid(dna_hash, &installed_apps, &allowed_apps);

        assert!(result == Err(AppSelectionError::NotAllowed));
    }

    #[test]
    fn returns_ok_if_app_is_installed_and_allowed() {
        let dna_hash: DnaHashB64 = DnaHash::from_raw_32([1; 32].to_vec()).into();
        let installed_apps = [new_fake_app_info("some_app_id", dna_hash.clone())];
        let allowed_apps = AllowedAppIds::from_str("some_app_id").unwrap();

        let result = check_app_valid(dna_hash, &installed_apps, &allowed_apps);

        assert!(result == Ok(()));
    }
}
