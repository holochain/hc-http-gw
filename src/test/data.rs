//! Test data helpers

use holochain_client::{AgentPubKey, CellId, Timestamp};
use holochain_conductor_api::{AppInfo, CellInfo};
use holochain_types::app::{AppManifest, AppStatus};
use holochain_types::prelude::{DnaHash, DnaModifiersBuilder};

/// Create a test [`AppInfo`] for use in tests
pub fn new_test_app_info(app_id: impl ToString, dna_hash: DnaHash) -> AppInfo {
    AppInfo {
        installed_app_id: app_id.to_string(),
        cell_info: [(
            "test-role".to_string(),
            vec![CellInfo::new_provisioned(
                CellId::new(dna_hash, AgentPubKey::from_raw_32(vec![1; 32])),
                DnaModifiersBuilder::default()
                    .network_seed("".to_string())
                    .build()
                    .unwrap(),
                "test-dna".to_string(),
            )],
        )]
        .into_iter()
        .collect(),
        status: AppStatus::Running.into(),
        agent_pub_key: AgentPubKey::from_raw_32([1; 32].to_vec()),
        manifest: AppManifest::V1(holochain_types::app::AppManifestV1 {
            name: Default::default(),
            description: Default::default(),
            roles: Vec::with_capacity(0),
            allow_deferred_memproofs: Default::default(),
        }),
        installed_at: Timestamp::now(),
    }
}
