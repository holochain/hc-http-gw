use holochain_client::AppInfo;
use holochain_conductor_api::{AppStatusFilter, CellInfo};
use holochain_types::dna::DnaHash;
use std::ops::Deref;
use std::sync::Arc;
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

pub type AppInfoCache = Arc<tokio::sync::RwLock<Vec<AppInfo>>>;

/// Return the [`AppInfo`] of the matching valid app if unique.
///
/// The returned app must meet the following criteria:
/// - It contains a cell with the given `dna_hash`.
/// - It can be identified by the given `coordinator_identifier`.
/// - It is in the list of `installed_apps` configured for the gateway.
///
/// If a unique app matching the criteria cannot be found, then an error is returned.
///
/// # Side effects
/// If a matching app is not found in the provided list of installed apps then a request to the
/// admin websocket will be made and the list will be updated with the results of that request.
pub async fn try_get_valid_app(
    dna_hash: DnaHash,
    coordinator_identifier: String,
    installed_apps: AppInfoCache,
    allowed_apps: &AllowedAppIds,
    admin_call: impl Deref<Target = impl AdminCall + ?Sized>,
) -> Result<AppInfo, AppSelectionError> {
    let app_info = {
        let installed_apps = installed_apps.read().await;
        choose_unique_app(&dna_hash, &coordinator_identifier, &installed_apps)
            .ok()
            .cloned()
    };

    let app_info = match app_info {
        Some(app_info) => app_info,
        None => {
            let new_installed_apps = admin_call
                .list_apps(Some(AppStatusFilter::Running))
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to get a list of apps from Holochain: {}", e);
                    vec![]
                });

            if !new_installed_apps.is_empty() {
                // If we got a response from Holochain, then we have a chance of finding the app.
                // Update the app info cache and search again.

                let mut installed_apps = installed_apps.write().await;
                *installed_apps = new_installed_apps.clone();
                choose_unique_app(
                    &dna_hash,
                    &coordinator_identifier,
                    &installed_apps.downgrade(),
                )?
                .clone()
            } else {
                // We either couldn't get a response from Holochain or the response was empty.
                // In either case, we can't find the app.

                return Err(AppSelectionError::NotInstalled);
            }
        }
    };

    if !allowed_apps.contains(&app_info.installed_app_id) {
        tracing::info!(
            "Found an app but access is not permitted: {}",
            app_info.installed_app_id
        );
        return Err(AppSelectionError::NotAllowed);
    }

    Ok(app_info)
}

fn choose_unique_app<'a>(
    dna_hash: &DnaHash,
    coordinator_identifier: &str,
    installed_apps: &'a [AppInfo],
) -> Result<&'a AppInfo, AppSelectionError> {
    let mut found_apps = installed_apps.iter().filter(|a| {
        // TODO: Use real `coordinator_identifier` when field available.
        a.installed_app_id == coordinator_identifier
            && a.cell_info.values().any(|cell_infos| {
                cell_infos.iter().any(|cell_info| match cell_info {
                    CellInfo::Provisioned(provisioned) => {
                        provisioned.cell_id.dna_hash() == dna_hash
                    }
                    _ => false,
                })
            })
    });

    let app_info = found_apps.next().ok_or(AppSelectionError::NotInstalled)?;

    // TODO From Holochain 0.5 we could use `installed_at` to pick the earliest installed app.
    if found_apps.next().is_some() {
        tracing::warn!(
            ?dna_hash,
            ?coordinator_identifier,
            "Multiple apps identified, could not determine which to call"
        );
        return Err(AppSelectionError::MultipleMatching);
    }

    Ok(app_info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::data;
    use crate::MockAdminCall;
    use std::str::FromStr;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn returns_error_if_app_not_installed() {
        let dna_hash = DnaHash::from_raw_32(vec![1; 32]);
        let installed_apps: AppInfoCache = Default::default();
        let allowed_apps = AllowedAppIds::from_str("").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        admin_websocket
            .expect_list_apps()
            .returning(|_| Box::pin(async { Ok(Vec::new()) }))
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert_eq!(result, Err(AppSelectionError::NotInstalled));
    }

    #[tokio::test]
    async fn returns_error_if_app_installed_but_not_allowed() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let installed_apps = vec![data::new_test_app_info("some_app_id", dna_hash.clone())];
        let installed_apps = Arc::new(RwLock::new(installed_apps));
        let allowed_apps = AllowedAppIds::from_str("other_app_id").unwrap();
        let admin_websocket = MockAdminCall::new();

        let result = try_get_valid_app(
            dna_hash,
            "some_app_id".to_string(),
            installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert_eq!(result, Err(AppSelectionError::NotAllowed));
    }

    #[tokio::test]
    async fn returns_ok_if_app_is_installed_and_allowed() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let app_info = data::new_test_app_info("some_app_id", dna_hash.clone());
        let installed_apps = vec![app_info.clone()];
        let installed_apps = Arc::new(RwLock::new(installed_apps));
        let allowed_apps = AllowedAppIds::from_str("some_app_id").unwrap();
        let admin_websocket = MockAdminCall::new();

        let result = try_get_valid_app(
            dna_hash,
            "some_app_id".to_string(),
            installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert_eq!(result, Ok(app_info));
    }

    #[tokio::test]
    async fn checks_app_list_from_websocket_if_not_in_installed_apps() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let installed_apps = Vec::new();
        let installed_apps = Arc::new(RwLock::new(installed_apps));
        let allowed_apps = AllowedAppIds::from_str("some_app_id").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let app_info = data::new_test_app_info("some_app_id", dna_hash.clone());
        let app_info_cloned = app_info.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move |_| {
                let app_info = app_info_cloned.clone();
                Box::pin(async { Ok(vec![app_info]) })
            })
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "some_app_id".to_string(),
            installed_apps,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert_eq!(result, Ok(app_info));
    }

    #[tokio::test]
    async fn returns_error_if_multiple_apps_match() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let installed_apps = vec![
            data::new_test_app_info("app_1", dna_hash.clone()),
            data::new_test_app_info("app_1", dna_hash.clone()),
        ];
        let installed_apps_cache = Arc::new(RwLock::new(installed_apps.clone()));
        let allowed_apps = AllowedAppIds::from_str("app_1,app_2").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let installed_apps_cloned = installed_apps.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move |_| {
                let installed_apps = installed_apps_cloned.clone();
                Box::pin(async move { Ok(installed_apps.clone()) })
            })
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            installed_apps_cache,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert_eq!(result, Err(AppSelectionError::MultipleMatching));
    }

    #[tokio::test]
    async fn returns_error_if_coordinator_identifier_does_not_match_app_id() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let installed_apps = vec![data::new_test_app_info("app_2", dna_hash.clone())];
        let installed_apps_cache = Arc::new(RwLock::new(installed_apps.clone()));
        let allowed_apps = AllowedAppIds::from_str("app_2").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let installed_apps_cloned = installed_apps.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move |_| {
                let installed_apps = installed_apps_cloned.clone();
                Box::pin(async move { Ok(installed_apps.clone()) })
            })
            .once();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            installed_apps_cache,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert_eq!(result, Err(AppSelectionError::NotInstalled));
    }

    #[tokio::test]
    async fn returns_error_if_matching_coordinator_identifier_not_in_allowed_list() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let installed_apps_cache = Arc::new(RwLock::new(vec![
            data::new_test_app_info("app_1", dna_hash.clone()),
            data::new_test_app_info("app_2", dna_hash.clone()),
        ]));
        let allowed_apps = AllowedAppIds::from_str("app_2").unwrap();
        let admin_websocket = MockAdminCall::new();

        let result = try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            installed_apps_cache,
            &allowed_apps,
            &admin_websocket,
        )
        .await;

        assert_eq!(result, Err(AppSelectionError::NotAllowed));
    }

    #[tokio::test]
    async fn updates_installed_apps_list_if_not_in_initial_list() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let installed_apps: AppInfoCache = Default::default();
        let allowed_apps = AllowedAppIds::from_str("app_1").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let new_installed_apps = vec![data::new_test_app_info("app_1", dna_hash.clone())];
        let new_installed_apps_cloned = new_installed_apps.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move |_| {
                let new_installed_apps = new_installed_apps_cloned.clone();
                Box::pin(async move { Ok(new_installed_apps.clone()) })
            })
            .once();

        try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            installed_apps.clone(),
            &allowed_apps,
            &admin_websocket,
        )
        .await
        .unwrap();

        assert_eq!(&*installed_apps.read().await, &new_installed_apps);
    }

    #[tokio::test]
    async fn installed_apps_results_are_cached_and_reused() {
        let dna_hash = DnaHash::from_raw_32([1; 32].to_vec());
        let allowed_apps = AllowedAppIds::from_str("app_1").unwrap();
        let mut admin_websocket = MockAdminCall::new();
        let new_installed_apps = vec![data::new_test_app_info("app_1", dna_hash.clone())];

        // Cache is empty so...
        let installed_apps_cache: AppInfoCache = Default::default();

        // ...make a request to the admin websocket.
        let new_installed_apps_cloned = new_installed_apps.clone();
        admin_websocket
            .expect_list_apps()
            .returning(move |_| {
                let new_installed_apps = new_installed_apps_cloned.clone();
                Box::pin(async move { Ok(new_installed_apps.clone()) })
            })
            .once();

        try_get_valid_app(
            dna_hash.clone(),
            "app_1".to_string(),
            installed_apps_cache.clone(),
            &allowed_apps,
            &admin_websocket,
        )
        .await
        .unwrap();

        // Prevent the cache being written to.
        let cache_handle = installed_apps_cache.clone();
        let _lock = cache_handle.read().await;

        // This time the cache is used and so no new request is made.
        try_get_valid_app(
            dna_hash,
            "app_1".to_string(),
            installed_apps_cache,
            &allowed_apps,
            &admin_websocket,
        )
        .await
        .unwrap();
    }
}
