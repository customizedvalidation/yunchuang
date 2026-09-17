//! WP-P2-B5 FluidCache + DistributedTrainingConfig 基本 CRUD 集成测试。
//!
//! 直接通过 service 层调用（无独立 handler 端点，作为 acceleration suite 子资源）。

use sqlx::SqlitePool;

use metaclouds_backend_rust::orm::PaginationParams;
use metaclouds_backend_rust::services::distributed_training as training_service;
use metaclouds_backend_rust::services::fluid_cache as fluid_service;
use serde_json::json;

async fn setup_pool() -> SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    pool
}

#[tokio::test]
async fn b5_fluid_cache_basic_crud() {
    let pool = setup_pool().await;

    // Create a dataset first (FK)
    sqlx::query("INSERT INTO datasets (created_at, updated_at, name, description, type, source_path, format, size_bytes, tenant_id, created_by, status, labels) \
                 VALUES (?1, ?2, 'fc-ds', '', 'private', '/data', '', 0, 1, 1, 'active', '{}')")
        .bind(chrono::Utc::now())
        .bind(chrono::Utc::now())
        .execute(&pool)
        .await
        .unwrap();

    // Create
    let created = fluid_service::create_fluid_cache(
        &pool,
        fluid_service::CreateFluidCacheInput {
            name: "cache-1".to_string(),
            dataset_id: 1,
            namespace: "default".to_string(),
            path: "/mnt/cache".to_string(),
            cache_class: "alluxio".to_string(),
            replicas: 3,
            status: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(created.name, "cache-1");
    assert_eq!(created.replicas, 3);
    assert_eq!(created.dataset_id, 1);

    // Get by id
    let fetched = fluid_service::get_fluid_cache(&pool, created.id)
        .await
        .unwrap();
    assert_eq!(fetched.id, created.id);

    // List by dataset_id
    let list = fluid_service::list_fluid_caches(&pool, PaginationParams::default(), Some(1))
        .await
        .unwrap();
    assert!(list.total >= 1);

    // Update
    let updated = fluid_service::update_fluid_cache(
        &pool,
        created.id,
        fluid_service::UpdateFluidCacheInput {
            replicas: Some(5),
            status: Some("active".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.replicas, 5);
    assert_eq!(updated.status, "active");

    // Delete
    fluid_service::delete_fluid_cache(&pool, created.id)
        .await
        .unwrap();
    let result = fluid_service::get_fluid_cache(&pool, created.id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn b5_training_config_basic_crud() {
    let pool = setup_pool().await;

    // Create
    let created = training_service::create_training_config(
        &pool,
        training_service::CreateTrainingConfigInput {
            name: "torch-ddp".to_string(),
            description: "PyTorch DDP".to_string(),
            framework: "pytorch".to_string(),
            worker_replicas: 4,
            gpu_per_worker: 1,
            cpu_per_worker: 4,
            memory_per_worker_gb: 16,
            entrypoint: "train.py".to_string(),
            env_vars: json!({"NCCL_DEBUG": "INFO"}),
            tenant_id: 1,
            created_by: 1,
            status: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(created.framework, "pytorch");
    assert_eq!(created.worker_replicas, 4);
    assert_eq!(created.gpu_per_worker, 1);

    // Get by id
    let fetched = training_service::get_training_config(&pool, created.id)
        .await
        .unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.framework, "pytorch");

    // List
    let list = training_service::list_training_configs(&pool, PaginationParams::default(), Some(1))
        .await
        .unwrap();
    assert!(list.total >= 1);

    // Update
    let updated = training_service::update_training_config(
        &pool,
        created.id,
        training_service::UpdateTrainingConfigInput {
            worker_replicas: Some(8),
            gpu_per_worker: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.worker_replicas, 8);
    assert_eq!(updated.gpu_per_worker, 2);

    // Delete
    training_service::delete_training_config(&pool, created.id)
        .await
        .unwrap();
    let result = training_service::get_training_config(&pool, created.id).await;
    assert!(result.is_err());
}
