use super::super::common::{
    ClientFactory, Protocol, generate_test_data_id, generate_test_group, wait_for,
};
use nacos_rust_client::client::config_client::ConfigKey;
use std::time::Duration;

#[tokio::test]
async fn test_config_add_query_http_http_standalone() {
    test_config_add_query(Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_add_query_grpc_grpc_standalone() {
    test_config_add_query(Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_add_query_http_grpc_standalone() {
    test_config_add_query(Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_add_query_grpc_http_standalone() {
    test_config_add_query(Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_add_update_query_http_http_standalone() {
    test_config_add_update_query(Protocol::Http, Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_add_update_query_grpc_grpc_standalone() {
    test_config_add_update_query(Protocol::Grpc, Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_add_update_query_http_grpc_http_standalone() {
    test_config_add_update_query(Protocol::Http, Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_add_update_query_grpc_http_grpc_standalone() {
    test_config_add_update_query(Protocol::Grpc, Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_add_delete_query_http_http_standalone() {
    test_config_add_delete_query(Protocol::Http, Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_add_delete_query_grpc_grpc_standalone() {
    test_config_add_delete_query(Protocol::Grpc, Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_add_delete_query_http_grpc_http_standalone() {
    test_config_add_delete_query(Protocol::Http, Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_add_delete_query_grpc_http_grpc_standalone() {
    test_config_add_delete_query(Protocol::Grpc, Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_listen_update_http_http_standalone() {
    test_config_listen_update(Protocol::Http, Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_listen_update_grpc_grpc_standalone() {
    test_config_listen_update(Protocol::Grpc, Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_listen_update_http_grpc_http_standalone() {
    test_config_listen_update(Protocol::Http, Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_listen_update_grpc_http_grpc_standalone() {
    test_config_listen_update(Protocol::Grpc, Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_listen_delete_http_http_standalone() {
    test_config_listen_delete(Protocol::Http, Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_listen_delete_grpc_grpc_standalone() {
    test_config_listen_delete(Protocol::Grpc, Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_listen_delete_http_grpc_http_standalone() {
    test_config_listen_delete(Protocol::Http, Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_listen_delete_grpc_http_grpc_standalone() {
    test_config_listen_delete(Protocol::Grpc, Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_batch_add_query_http_http_standalone() {
    test_config_batch_add_query(Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_config_batch_add_query_grpc_grpc_standalone() {
    test_config_batch_add_query(Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_batch_add_query_http_grpc_standalone() {
    test_config_batch_add_query(Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_config_batch_add_query_grpc_http_standalone() {
    test_config_batch_add_query(Protocol::Grpc, Protocol::Http, false, None).await;
}

async fn test_config_add_query(
    add_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let add_client = factory.create_config_client(add_protocol, use_cluster, cluster_node);
    let query_client = factory.create_config_client(query_protocol, use_cluster, cluster_node);

    let data_id = generate_test_data_id();
    let group = generate_test_group();
    let key = ConfigKey::new(&data_id, &group, "");

    let test_content = "test_config_value";

    // Add config
    add_client.set_config(&key, test_content).await.unwrap();

    // Query config
    let retrieved_content = query_client.get_config(&key).await.unwrap();
    assert_eq!(retrieved_content, test_content);

    // Cleanup
    add_client.del_config(&key).await.unwrap();
}

async fn test_config_add_update_query(
    add_protocol: Protocol,
    update_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let add_client = factory.create_config_client(add_protocol, use_cluster, cluster_node);
    let update_client = factory.create_config_client(update_protocol, use_cluster, cluster_node);
    let query_client = factory.create_config_client(query_protocol, use_cluster, cluster_node);

    let data_id = generate_test_data_id();
    let group = generate_test_group();
    let key = ConfigKey::new(&data_id, &group, "");

    let initial_content = "initial_config_value";
    let updated_content = "updated_config_value";

    // Add config
    add_client.set_config(&key, initial_content).await.unwrap();

    // Update config
    update_client
        .set_config(&key, updated_content)
        .await
        .unwrap();

    // Query config
    let retrieved_content = query_client.get_config(&key).await.unwrap();
    assert_eq!(retrieved_content, updated_content);

    // Cleanup
    add_client.del_config(&key).await.unwrap();
}

async fn test_config_add_delete_query(
    add_protocol: Protocol,
    delete_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let add_client = factory.create_config_client(add_protocol, use_cluster, cluster_node);
    let delete_client = factory.create_config_client(delete_protocol, use_cluster, cluster_node);
    let query_client = factory.create_config_client(query_protocol, use_cluster, cluster_node);

    let data_id = generate_test_data_id();
    let group = generate_test_group();
    let key = ConfigKey::new(&data_id, &group, "");

    let test_content = "test_config_value";

    // Add config
    add_client.set_config(&key, test_content).await.unwrap();

    // Delete config
    delete_client.del_config(&key).await.unwrap();

    // Query config - should not exist
    let result = query_client.get_config(&key).await;
    assert!(result.is_err());
}

async fn test_config_listen_update(
    listen_protocol: Protocol,
    add_protocol: Protocol,
    update_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    use nacos_rust_client::client::config_client::ConfigDefaultListener;
    use std::sync::Arc;

    let factory = ClientFactory::new();
    let listen_client = factory.create_config_client(listen_protocol, use_cluster, cluster_node);
    let add_client = factory.create_config_client(add_protocol, use_cluster, cluster_node);
    let update_client = factory.create_config_client(update_protocol, use_cluster, cluster_node);

    let data_id = generate_test_data_id();
    let group = generate_test_group();
    let key = ConfigKey::new(&data_id, &group, "");

    let initial_content = "initial_config_value";
    let updated_content = "updated_config_value";

    // Create listener
    let listener = Box::new(ConfigDefaultListener::new(
        key.clone(),
        Arc::new(|s| Some(s.to_string())),
    ));

    // Add config
    add_client.set_config(&key, initial_content).await.unwrap();

    // Subscribe to changes
    listen_client.subscribe(listener.clone()).await.unwrap();

    // Update config
    update_client
        .set_config(&key, updated_content)
        .await
        .unwrap();

    // Wait for listener to receive update
    let received_update = wait_for(
        || async {
            if let Some(value) = listener.get_value() {
                *value == updated_content
            } else {
                false
            }
        },
        Duration::from_secs(5),
    )
    .await;

    assert!(received_update, "Listener did not receive config update");

    // Cleanup
    add_client.del_config(&key).await.unwrap();
}

async fn test_config_listen_delete(
    listen_protocol: Protocol,
    add_protocol: Protocol,
    delete_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    use nacos_rust_client::client::config_client::ConfigDefaultListener;
    use std::sync::Arc;

    let factory = ClientFactory::new();
    let listen_client = factory.create_config_client(listen_protocol, use_cluster, cluster_node);
    let add_client = factory.create_config_client(add_protocol, use_cluster, cluster_node);
    let delete_client = factory.create_config_client(delete_protocol, use_cluster, cluster_node);

    let data_id = generate_test_data_id();
    let group = generate_test_group();
    let key = ConfigKey::new(&data_id, &group, "");

    let test_content = "test_config_value";

    // Create listener
    let listener = Box::new(ConfigDefaultListener::new(
        key.clone(),
        Arc::new(|s| Some(s.to_string())),
    ));

    // Add config
    add_client.set_config(&key, test_content).await.unwrap();

    // Subscribe to changes
    listen_client.subscribe(listener.clone()).await.unwrap();

    // Delete config
    delete_client.del_config(&key).await.unwrap();

    let v = listen_client.get_config(&key).await;
    assert!(v.is_err());
    assert_eq!(listener.get_value().unwrap().as_str(), test_content);
}

async fn test_config_batch_add_query(
    add_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let add_client = factory.create_config_client(add_protocol, use_cluster, cluster_node);
    let query_client = factory.create_config_client(query_protocol, use_cluster, cluster_node);

    let group = generate_test_group();
    let configs = vec![
        (generate_test_data_id(), "value1"),
        (generate_test_data_id(), "value2"),
        (generate_test_data_id(), "value3"),
    ];

    // Add multiple configs
    for (data_id, value) in &configs {
        let key = ConfigKey::new(data_id, &group, "");
        add_client.set_config(&key, value).await.unwrap();
    }

    // Query all configs
    for (data_id, expected_value) in &configs {
        let key = ConfigKey::new(data_id, &group, "");
        let retrieved_content = query_client.get_config(&key).await.unwrap();
        assert_eq!(retrieved_content, *expected_value);
    }

    // Cleanup
    for (data_id, _) in &configs {
        let key = ConfigKey::new(data_id, &group, "");
        add_client.del_config(&key).await.unwrap();
    }
}
