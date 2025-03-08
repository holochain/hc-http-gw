use std::ops::Deref;

use holochain_client::AppInfo;
use holochain_types::dna::DnaHash;
use thiserror::Error;

use crate::{config::AllowedAppIds, AdminCall};

#[derive(Debug, PartialEq, Error)]
pub enum AppSelectionError {
    #[error("App is not installed on the conductor")]
    NotInstalled,

    #[error("App is not in the list of allowed apps")]
    NotAllowed,

    #[error("Multiple matching apps were found, could not determine which to call")]
    MultipleMatching,
}

fn find_installed_app<'a>(
    dna_hash: &DnaHash,
    coordinator_identifier: &str,
    installed_apps: &'a [AppInfo],
) -> Result<&'a AppInfo, AppSelectionError> {
    let mut found_apps = installed_apps.iter().filter(|a| {
        // TODO: Use real `coordinator_identifier` when field available.
        a.installed_app_id == coordinator_identifier
            && a.manifest.app_roles().iter().any(|r| {
                r.dna
                    .installed_hash
                    .as_ref()
                    .is_some_and(|hash| &Into::<DnaHash>::into(hash.clone()) == dna_hash)
            })
    });

    let app_info = found_apps.next().ok_or(AppSelectionError::NotInstalled)?;

    if found_apps.next().is_some() {
        return Err(AppSelectionError::MultipleMatching);
    }

    Ok(app_info)
}

pub async fn try_get_valid_app(
    dna_hash: DnaHash,
    coordinator_identifier: String,
    installed_apps: &mut Vec<AppInfo>,
    allowed_apps: &AllowedAppIds,
    admin_websocket: impl Deref<Target = impl AdminCall + ?Sized>,
) -> Result<AppInfo, AppSelectionError> {
    let app_info = if let Ok(app_info) =
        find_installed_app(&dna_hash, &coordinator_identifier, installed_apps)
    {
        app_info
    } else {
        *installed_apps = admin_websocket.list_apps().await;
        find_installed_app(&dna_hash, &coordinator_identifier, installed_apps)?
    };

    allowed_apps
        .contains(&app_info.installed_app_id)
        .then_some(())
        .ok_or(AppSelectionError::NotAllowed)?;

    Ok(app_info.clone())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use crate::MockAdminCall;

    use super::*;
    use assert2::assert;
    use holochain_client::AgentPubKey;
    use holochain_types::app::{AppManifest, AppRoleDnaManifest, AppRoleManifest, AppStatus};

    fn new_fake_app_info(app_id: impl ToString, dna_hash: DnaHash) -> AppInfo {
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
                        installed_hash: Some(dna_hash.into()),
                        clone_limit: Default::default(),
                    },
                }],
                allow_deferred_memproofs: Default::default(),
            }),
        }
    }

    #[tokio::test]
    async fn returns_error_if_app_not_installed() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec()).into();
        let mut installed_apps = Vec::new();
        let allowed_apps = AllowedAppIds::from_str("").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        admin_websocket
            .expect_list_apps()
            .returning(|| Box::pin(async { Vec::new() }))
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert!(result == Err(AppSelectionError::NotInstalled));
    }

    #[tokio::test]
    async fn returns_error_if_app_installed_but_not_allowed() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let mut installed_apps = vec![new_fake_app_info("some_app_id", dna_hash.clone())];
        let allowed_apps = AllowedAppIds::from_str("other_app_id").unwrap();
        let admin_websocket = MockAdminCall::new();

        let result = try_get_valid_app(
            dna_hash,
            "some_app_id".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert!(result == Err(AppSelectionError::NotAllowed));
    }

    #[tokio::test]
    async fn returns_ok_if_app_is_installed_and_allowed() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let app_info = new_fake_app_info("some_app_id", dna_hash.clone());
        let mut installed_apps = vec![app_info.clone()];
        let allowed_apps = AllowedAppIds::from_str("some_app_id").unwrap();
        let admin_websocket = MockAdminCall::new();

        let result = try_get_valid_app(
            dna_hash,
            "some_app_id".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert!(result == Ok(app_info));
    }

    #[tokio::test]
    async fn checks_app_list_from_websocket_if_not_in_installed_apps() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let mut installed_apps = Vec::new();
        let allowed_apps = AllowedAppIds::from_str("some_app_id").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let app_info = new_fake_app_info("some_app_id", dna_hash.clone());
        let app_info_cloned = app_info.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move || {
                let app_info = app_info_cloned.clone();
                Box::pin(async { vec![app_info] })
            })
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "some_app_id".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert!(result == Ok(app_info));
    }

    #[tokio::test]
    async fn returns_error_if_multiple_apps_match() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let mut installed_apps = vec![
            new_fake_app_info("app_1", dna_hash.clone()),
            new_fake_app_info("app_1", dna_hash.clone()),
        ];
        let allowed_apps = AllowedAppIds::from_str("app_1,app_2").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let installed_apps_cloned = installed_apps.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move || {
                let installed_apps = installed_apps_cloned.clone();
                Box::pin(async move { installed_apps.clone() })
            })
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert!(result == Err(AppSelectionError::MultipleMatching));
    }

    #[tokio::test]
    async fn returns_error_if_coordinator_identifier_does_not_match_app_id() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let mut installed_apps = vec![new_fake_app_info("app_2", dna_hash.clone())];
        let allowed_apps = AllowedAppIds::from_str("app_2").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let installed_apps_cloned = installed_apps.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move || {
                let installed_apps = installed_apps_cloned.clone();
                Box::pin(async move { installed_apps.clone() })
            })
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert!(result == Err(AppSelectionError::NotInstalled));
    }

    #[tokio::test]
    async fn returns_error_if_matching_coordinator_identifier_not_in_allowed_list() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let mut installed_apps = vec![
            new_fake_app_info("app_1", dna_hash.clone()),
            new_fake_app_info("app_2", dna_hash.clone()),
        ];
        let allowed_apps = AllowedAppIds::from_str("app_2").unwrap();
        let admin_websocket = MockAdminCall::new();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert!(result == Err(AppSelectionError::NotAllowed));
    }

    #[tokio::test]
    async fn updates_installed_apps_list_if_not_in_initial_list() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let mut installed_apps = vec![];
        let allowed_apps = AllowedAppIds::from_str("app_1").unwrap();
        let mut admin_websocket = AdminWebsocketWrapper::new();
        let new_installed_apps = vec![new_fake_app_info("app_1", dna_hash.clone())];
        admin_websocket
            .expect_list_apps()
            .return_const(new_installed_apps.clone())
            .once();

        try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            &mut installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await
        .unwrap();

        assert!(installed_apps == new_installed_apps);
    }
}
