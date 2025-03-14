use crate::sweet::{
    init_zome, install_fixture1, install_fixture2, CreateResponse, GetWithLimitRequest, TestType,
};
use base64::Engine;
use holochain::prelude::{CellId, DnaHash};
use holochain::sweettest::SweetConductor;
use holochain_conductor_api::CellInfo;
use holochain_http_gateway::test::test_tracing::initialize_testing_tracing_subscriber;
use holochain_http_gateway::ErrorResponse;
use holochain_types::app::InstalledApp;
use reqwest::StatusCode;
use setup::TestGateway;

mod setup;
mod sweet;

#[tokio::test(flavor = "multi_thread")]
async fn simple_zome_call() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app, "coordinator1".to_string())
        .await
        .unwrap();

    let cell_id = get_first_cell_from_app(&sweet_conductor, &app).await;

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;

    let response = gateway
        .call_zome(
            cell_id.dna_hash(),
            "fixture1",
            "coordinator1",
            "get_all_1",
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn respond_with_data() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();

    let cell_id = get_first_cell_from_app(&sweet_conductor, &app).await;

    // Create some data
    for _ in 0..3 {
        sweet_conductor
            .easy_call_zome::<_, CreateResponse, _>(
                &app.agent_key,
                None,
                cell_id.clone(),
                "coordinator1",
                "create_1",
                (),
            )
            .await
            .unwrap();
    }

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;

    let response = gateway
        .call_zome(
            cell_id.dna_hash(),
            "fixture1",
            "coordinator1",
            "get_all_1",
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json_response = response.json::<Vec<TestType>>().await.unwrap();
    assert_eq!(json_response.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_data_with_agent_key_payload() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();

    let cell_id = get_first_cell_from_app(&sweet_conductor, &app).await;

    // Create some data
    for _ in 0..3 {
        sweet_conductor
            .easy_call_zome::<_, CreateResponse, _>(
                &app.agent_key,
                None,
                cell_id.clone(),
                "coordinator1",
                "create_1",
                (),
            )
            .await
            .unwrap();
    }

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;

    let response = gateway
        .call_zome(
            cell_id.dna_hash(),
            "fixture1",
            "coordinator1",
            "get_mine",
            Some(&make_payload(&app.agent_key)),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json_response = response.json::<Vec<TestType>>().await.unwrap();
    assert_eq!(json_response.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_data_with_object_payload() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();

    let cell_id = get_first_cell_from_app(&sweet_conductor, &app).await;

    // Create some data
    for _ in 0..3 {
        sweet_conductor
            .easy_call_zome::<_, CreateResponse, _>(
                &app.agent_key,
                None,
                cell_id.clone(),
                "coordinator1",
                "create_1",
                (),
            )
            .await
            .unwrap();
    }

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;

    let response = gateway
        .call_zome(
            cell_id.dna_hash(),
            "fixture1",
            "coordinator1",
            "get_limited",
            Some(&make_payload(&GetWithLimitRequest { limit: 2 })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json_response = response.json::<Vec<TestType>>().await.unwrap();
    assert_eq!(json_response.len(), 2);
}

#[ignore = "Holochain incorrectly handles the same integrity zome but different coordinator zomes"]
#[tokio::test(flavor = "multi_thread")]
async fn get_data_from_multiple_apps() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app_1 = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    let cell_id_1 = get_first_cell_from_app(&sweet_conductor, &app_1).await;

    // Create some data
    for _ in 0..2 {
        sweet_conductor
            .easy_call_zome::<_, CreateResponse, _>(
                &app_1.agent_key,
                None,
                cell_id_1.clone(),
                "coordinator1",
                "create_1",
                (),
            )
            .await
            .unwrap();
    }

    let app_2 = install_fixture2(sweet_conductor.clone(), None)
        .await
        .unwrap();
    let cell_id_2 = get_first_cell_from_app(&sweet_conductor, &app_2).await;

    // Create some data
    for _ in 0..3 {
        sweet_conductor
            .easy_call_zome::<_, CreateResponse, _>(
                &app_2.agent_key,
                None,
                cell_id_2.clone(),
                "coordinator2",
                "create_2",
                (),
            )
            .await
            .unwrap();
    }

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;

    let response = gateway
        .call_zome(
            cell_id_1.dna_hash(),
            "fixture1",
            "coordinator1",
            "get_all_1",
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json_response = response.json::<Vec<TestType>>().await.unwrap();
    assert_eq!(json_response.len(), 2);

    let response = gateway
        .call_zome(
            cell_id_2.dna_hash(),
            "fixture2",
            "coordinator2",
            "get_all_2",
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json_response = response.json::<Vec<TestType>>().await.unwrap();
    assert_eq!(json_response.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn call_function_that_is_not_exposed() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app, "coordinator1".to_string())
        .await
        .unwrap();

    let cell_id = get_first_cell_from_app(&sweet_conductor, &app).await;

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;

    let response = gateway
        .call_zome(
            cell_id.dna_hash(),
            "fixture1",
            "coordinator1",
            "create_1",
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = response.json::<ErrorResponse>().await.unwrap();
    assert_eq!(
        response.error,
        "Function create_1 in zome coordinator1 in app fixture1 is not allowed"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn call_missing_app() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;

    let response = gateway
        .call_zome(
            &DnaHash::from_raw_32(vec![2; 32]),
            "asdf",
            "asdf",
            "asdf",
            None,
        )
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn get_first_cell_from_app(sweet_conductor: &SweetConductor, app: &InstalledApp) -> CellId {
    let app_info = sweet_conductor
        .list_apps(None)
        .await
        .unwrap()
        .into_iter()
        .find(|a| &a.installed_app_id == app.id())
        .unwrap();

    let cell_id = match app_info
        .cell_info
        .values()
        .next()
        .unwrap()
        .iter()
        .next()
        .unwrap()
    {
        CellInfo::Provisioned(provisioned) => provisioned.cell_id.clone(),
        _ => panic!("Expected a provisioned cell"),
    };
    cell_id
}

#[tokio::test(flavor = "multi_thread")]
async fn zome_call_load_test() {
    initialize_testing_tracing_subscriber();
    let sweet_conductor = SweetConductor::from_standard_config().await;
    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    let cell_id = get_first_cell_from_app(&sweet_conductor, &app).await;

    // create some test data
    for _ in 0..3 {
        sweet_conductor
            .easy_call_zome::<_, CreateResponse, _>(
                &app.agent_key,
                None,
                cell_id.clone(),
                "coordinator1",
                "create_1",
                (),
            )
            .await
            .unwrap();
    }

    let gateway = TestGateway::spawn(sweet_conductor.clone()).await;
    let address = gateway.address.clone();

    //  test parameters
    const NUM_CLIENTS: usize = 5;
    const REQUEST_INTERVAL_MS: u64 = 500;
    const TEST_DURATION_SEC: u64 = 10;
    const HTTP_TIMEOUT_SEC: u64 = 5;

    let start_time = std::time::Instant::now();
    let mut handles = Vec::new();

    // spawn client tasks
    for _ in 0..NUM_CLIENTS {
        let cell_id_clone = cell_id.clone();
        let address_clone = address.clone();

        // set up http client with timeout
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SEC))
            .pool_max_idle_per_host(1)
            .build()
            .unwrap();

        let handle = tokio::spawn(async move {
            // keep sending requests until test duration is reached
            while start_time.elapsed().as_secs() < TEST_DURATION_SEC {
                let url = format!(
                    "http://{}/{}/fixture1/coordinator1/get_all_1",
                    address_clone,
                    cell_id_clone.dna_hash(),
                );

                let response = client.get(url).send().await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);

                tokio::time::sleep(tokio::time::Duration::from_millis(REQUEST_INTERVAL_MS)).await;
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task failed");
    }

    // ensure test ran for at least 10 seconds
    let test_duration = start_time.elapsed().as_secs();
    assert!(
        test_duration >= 10,
        "Test did not run for at least 10 seconds"
    );

    println!("Load test completed in {} seconds", test_duration);
}

fn make_payload<T: serde::Serialize>(payload: &T) -> String {
    let v = serde_json::to_string(payload).unwrap();
    base64::prelude::BASE64_URL_SAFE.encode(v)
}
