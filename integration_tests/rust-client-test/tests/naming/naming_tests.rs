use super::super::common::{ClientFactory, Protocol, generate_test_group, generate_test_service};
use nacos_rust_client::client::naming_client::{Instance, QueryInstanceListParams};
use std::time::Duration;

#[tokio::test]
async fn test_naming_register_query_http_http_standalone() {
    test_naming_register_query(Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_naming_register_query_grpc_grpc_standalone() {
    test_naming_register_query(Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_naming_register_query_http_grpc_standalone() {
    test_naming_register_query(Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_naming_register_query_grpc_http_standalone() {
    test_naming_register_query(Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_naming_register_update_query_http_http_standalone() {
    test_naming_register_update_query(Protocol::Http, Protocol::Http, Protocol::Http, false, None)
        .await;
}

#[tokio::test]
async fn test_naming_register_update_query_grpc_grpc_standalone() {
    test_naming_register_update_query(Protocol::Grpc, Protocol::Grpc, Protocol::Grpc, false, None)
        .await;
}

#[tokio::test]
async fn test_naming_register_update_query_http_grpc_http_standalone() {
    test_naming_register_update_query(Protocol::Http, Protocol::Grpc, Protocol::Http, false, None)
        .await;
}

#[tokio::test]
async fn test_naming_register_update_query_grpc_http_grpc_standalone() {
    test_naming_register_update_query(Protocol::Grpc, Protocol::Http, Protocol::Grpc, false, None)
        .await;
}

#[tokio::test]
async fn test_naming_register_unregister_query_http_http_standalone() {
    test_naming_register_unregister_query(
        Protocol::Http,
        Protocol::Http,
        Protocol::Http,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_unregister_query_grpc_grpc_standalone() {
    test_naming_register_unregister_query(
        Protocol::Grpc,
        Protocol::Grpc,
        Protocol::Grpc,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_unregister_query_http_grpc_http_standalone() {
    test_naming_register_unregister_query(
        Protocol::Http,
        Protocol::Grpc,
        Protocol::Http,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_unregister_query_grpc_http_grpc_standalone() {
    test_naming_register_unregister_query(
        Protocol::Grpc,
        Protocol::Http,
        Protocol::Grpc,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_heartbeat_query_http_http_standalone() {
    test_naming_register_heartbeat_query(
        Protocol::Http,
        Protocol::Http,
        Protocol::Http,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_heartbeat_query_grpc_grpc_standalone() {
    test_naming_register_heartbeat_query(
        Protocol::Grpc,
        Protocol::Grpc,
        Protocol::Grpc,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_heartbeat_query_http_grpc_http_standalone() {
    test_naming_register_heartbeat_query(
        Protocol::Http,
        Protocol::Grpc,
        Protocol::Http,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_heartbeat_query_grpc_http_grpc_standalone() {
    test_naming_register_heartbeat_query(
        Protocol::Grpc,
        Protocol::Http,
        Protocol::Grpc,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_multiple_query_http_http_standalone() {
    test_naming_register_multiple_query(Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_naming_register_multiple_query_grpc_grpc_standalone() {
    test_naming_register_multiple_query(Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_naming_register_multiple_query_http_grpc_standalone() {
    test_naming_register_multiple_query(Protocol::Http, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_naming_register_multiple_query_grpc_http_standalone() {
    test_naming_register_multiple_query(Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_naming_register_metadata_query_http_http_standalone() {
    test_naming_register_metadata_query(
        Protocol::Http,
        Protocol::Http,
        Protocol::Http,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_metadata_query_grpc_grpc_standalone() {
    test_naming_register_metadata_query(
        Protocol::Grpc,
        Protocol::Grpc,
        Protocol::Grpc,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_metadata_query_http_grpc_http_standalone() {
    test_naming_register_metadata_query(
        Protocol::Http,
        Protocol::Grpc,
        Protocol::Http,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_register_metadata_query_grpc_http_grpc_standalone() {
    test_naming_register_metadata_query(
        Protocol::Grpc,
        Protocol::Http,
        Protocol::Grpc,
        false,
        None,
    )
    .await;
}

#[tokio::test]
async fn test_naming_listen_register_http_http_standalone() {
    test_naming_listen_register(Protocol::Http, Protocol::Http, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_naming_listen_register_grpc_grpc_standalone() {
    test_naming_listen_register(Protocol::Grpc, Protocol::Grpc, Protocol::Grpc, false, None).await;
}

#[tokio::test]
async fn test_naming_listen_register_http_grpc_standalone() {
    test_naming_listen_register(Protocol::Http, Protocol::Grpc, Protocol::Http, false, None).await;
}

#[tokio::test]
async fn test_naming_listen_register_grpc_http_standalone() {
    test_naming_listen_register(Protocol::Grpc, Protocol::Http, Protocol::Grpc, false, None).await;
}

async fn test_naming_register_query(
    register_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let register_client =
        factory.create_naming_client(register_protocol, use_cluster, cluster_node);
    let query_client = factory.create_naming_client(query_protocol, use_cluster, cluster_node);

    let service_name = generate_test_service();
    let group_name = generate_test_group();

    let instance = Instance::new_simple("127.0.0.1", 8080, &service_name, &group_name);

    // Register instance
    register_client.register(instance.clone());

    // Wait for registration
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Query instances
    let params = QueryInstanceListParams::new_simple(&service_name, &group_name);
    let instances = query_client.query_instances(params).await.unwrap();

    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].ip, "127.0.0.1");
    assert_eq!(instances[0].port, 8080);

    // Cleanup
    register_client.unregister(instance);
}

async fn test_naming_register_update_query(
    register_protocol: Protocol,
    update_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let register_client =
        factory.create_naming_client(register_protocol, use_cluster, cluster_node);
    let update_client = factory.create_naming_client(update_protocol, use_cluster, cluster_node);
    let query_client = factory.create_naming_client(query_protocol, use_cluster, cluster_node);

    let service_name = generate_test_service();
    let group_name = generate_test_group();

    let mut instance = Instance::new_simple("127.0.0.1", 8080, &service_name, &group_name);

    // Register instance
    register_client.register(instance.clone());

    // Wait for registration
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Update instance metadata
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("version".to_string(), "2.0".to_string());
    instance.metadata = Some(metadata);
    update_client.register(instance.clone());

    // Wait for update
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Query instances
    let params = QueryInstanceListParams::new_simple(&service_name, &group_name);
    let instances = query_client.query_instances(params).await.unwrap();

    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances[0]
            .metadata
            .as_ref()
            .unwrap()
            .get("version")
            .unwrap(),
        "2.0"
    );

    // Cleanup
    register_client.unregister(instance);
}

async fn test_naming_register_unregister_query(
    register_protocol: Protocol,
    _unregister_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let register_client =
        factory.create_naming_client(register_protocol, use_cluster, cluster_node);
    let query_client = factory.create_naming_client(query_protocol, use_cluster, cluster_node);

    let service_name = generate_test_service();
    let group_name = generate_test_group();

    let instance = Instance::new_simple("127.0.0.1", 8080, &service_name, &group_name);

    // Register instance
    register_client.register(instance.clone());

    // Wait for registration
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Unregister instance
    register_client.unregister(instance.clone());

    // Wait for unregistration
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Query instances - should be empty
    let params = QueryInstanceListParams::new_simple(&service_name, &group_name);
    let instances = query_client.query_instances(params).await.unwrap();

    assert_eq!(instances.len(), 0);
}

async fn test_naming_register_heartbeat_query(
    register_protocol: Protocol,
    heartbeat_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let register_client =
        factory.create_naming_client(register_protocol, use_cluster, cluster_node);
    let _heartbeat_client =
        factory.create_naming_client(heartbeat_protocol, use_cluster, cluster_node);
    let query_client = factory.create_naming_client(query_protocol, use_cluster, cluster_node);

    let service_name = generate_test_service();
    let group_name = generate_test_group();

    let instance = Instance::new_simple("127.0.0.1", 8080, &service_name, &group_name);

    // Register instance (heartbeat is automatic)
    register_client.register(instance.clone());

    // Wait for registration and heartbeat
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Query instances - should still be healthy
    let params = QueryInstanceListParams::new_simple(&service_name, &group_name);
    let instances = query_client.query_instances(params).await.unwrap();

    assert_eq!(instances.len(), 1);
    assert!(instances[0].healthy);

    // Cleanup
    register_client.unregister(instance);
}

async fn test_naming_register_multiple_query(
    register_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let register_client =
        factory.create_naming_client(register_protocol, use_cluster, cluster_node);
    let query_client = factory.create_naming_client(query_protocol, use_cluster, cluster_node);

    let service_name = generate_test_service();
    let group_name = generate_test_group();

    let instances = vec![
        Instance::new_simple("127.0.0.1", 8080, &service_name, &group_name),
        Instance::new_simple("127.0.0.1", 8081, &service_name, &group_name),
        Instance::new_simple("127.0.0.1", 8082, &service_name, &group_name),
    ];

    // Register multiple instances
    for instance in &instances {
        register_client.register(instance.clone());
    }

    // Wait for registration
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Query instances
    let params = QueryInstanceListParams::new_simple(&service_name, &group_name);
    let retrieved_instances = query_client.query_instances(params).await.unwrap();

    assert_eq!(retrieved_instances.len(), 3);

    // Cleanup
    for instance in &instances {
        register_client.unregister(instance.clone());
    }
}

async fn test_naming_register_metadata_query(
    register_protocol: Protocol,
    metadata_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    let factory = ClientFactory::new();
    let register_client =
        factory.create_naming_client(register_protocol, use_cluster, cluster_node);
    let metadata_client =
        factory.create_naming_client(metadata_protocol, use_cluster, cluster_node);
    let query_client = factory.create_naming_client(query_protocol, use_cluster, cluster_node);

    let service_name = generate_test_service();
    let group_name = generate_test_group();

    let mut instance = Instance::new_simple("127.0.0.1", 8080, &service_name, &group_name);
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("env".to_string(), "test".to_string());
    instance.metadata = Some(metadata);

    // Register instance
    register_client.register(instance.clone());

    // Wait for registration
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Update metadata
    instance
        .metadata
        .as_mut()
        .unwrap()
        .insert("version".to_string(), "1.0".to_string());
    metadata_client.register(instance.clone());

    // Wait for update
    tokio::time::sleep(Duration::from_millis(700)).await;

    // Query instances
    let params = QueryInstanceListParams::new_simple(&service_name, &group_name);
    let instances = query_client.query_instances(params).await.unwrap();

    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances[0].metadata.as_ref().unwrap().get("env").unwrap(),
        "test"
    );
    assert_eq!(
        instances[0]
            .metadata
            .as_ref()
            .unwrap()
            .get("version")
            .unwrap(),
        "1.0"
    );

    // Cleanup
    register_client.unregister(instance);
}

async fn test_naming_listen_register(
    listen_protocol: Protocol,
    register_protocol: Protocol,
    query_protocol: Protocol,
    use_cluster: bool,
    cluster_node: Option<usize>,
) {
    use nacos_rust_client::client::naming_client::{InstanceDefaultListener, ServiceInstanceKey};
    use std::sync::Arc;

    let factory = ClientFactory::new();
    let listen_client = factory.create_naming_client(listen_protocol, use_cluster, cluster_node);
    let register_client =
        factory.create_naming_client(register_protocol, use_cluster, cluster_node);
    let query_client = factory.create_naming_client(query_protocol, use_cluster, cluster_node);

    let service_name = generate_test_service();
    let group_name = generate_test_group();
    let service_key = ServiceInstanceKey::new(&service_name, &group_name);

    let instance = Instance::new_simple("127.0.0.1", 8080, &service_name, &group_name);

    // Create listener
    let listener = Box::new(InstanceDefaultListener::new(
        service_key.clone(),
        Some(Arc::new(|instances, add_list, remove_list| {
            println!(
                "Service instances change: count={}, add={}, remove={}",
                instances.len(),
                add_list.len(),
                remove_list.len()
            );
        })),
    ));

    // Subscribe to changes
    listen_client.subscribe(listener.clone()).await.unwrap();

    // Register instance
    register_client.register(instance.clone());

    // Wait for registration and listener update
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Query instances
    let params = QueryInstanceListParams::new_simple(&service_name, &group_name);
    let instances = query_client.query_instances(params).await.unwrap();

    assert_eq!(instances.len(), 1);

    // Cleanup
    register_client.unregister(instance);
}
