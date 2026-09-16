//! WP-P1-06 单元/集成测试：GORM 特性显式层行为。
//!
//! - HasTimestamps：insert 自动填时间戳；update 只刷 updated_at；
//! - SoftDelete：删除后默认查询过滤，Unscoped 可查；
//! - Pagination：total_pages 计算与 LIMIT/OFFSET；
//! - 三个模型（User/Tenant/Cluster）完整 CRUD；
//! - Json<T> 字段往返。

use metaclouds_backend_rust::models::cluster::{self, NewCluster};
use metaclouds_backend_rust::models::tenant::{self, NewTenant};
use metaclouds_backend_rust::models::user::{self, NewUser};
use metaclouds_backend_rust::orm::{total_pages, PaginationParams, SoftDelete};
use sqlx::sqlite::SqlitePool;

/// 单连接内存库 + 全量迁移。
async fn memory_pool() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

// ---------------------------------------------------------------------------
// 分页纯函数
// ---------------------------------------------------------------------------

#[test]
fn total_pages_matches_go_pagination() {
    // 与 Go math.Ceil(total / pageSize) 对齐
    assert_eq!(total_pages(0, 10), 0);
    assert_eq!(total_pages(9, 10), 1);
    assert_eq!(total_pages(10, 10), 1);
    assert_eq!(total_pages(11, 10), 2);
    assert_eq!(total_pages(25, 10), 3);
    assert_eq!(total_pages(100, 100), 1);
    assert_eq!(total_pages(101, 100), 2);
}

#[test]
fn pagination_params_normalize() {
    let p = PaginationParams::new(-3, 0).normalize();
    assert_eq!(p.page, 1);
    assert_eq!(p.page_size, 10);

    let p = PaginationParams::new(2, 500).normalize();
    assert_eq!(p.page, 2);
    assert_eq!(p.page_size, 100);
    assert_eq!(p.offset(), 100);
    assert_eq!(p.limit(), 100);
}

// ---------------------------------------------------------------------------
// HasTimestamps / SoftDelete（Tenant 为载体）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn timestamps_filled_on_insert_updated_only_on_update() {
    let pool = memory_pool().await;

    let t = tenant::create(
        &pool,
        NewTenant {
            name: "ts-tenant",
            description: "",
            status: "active",
            gpu_quota: 0,
            cpu_quota: 0,
            memory_quota: 0,
            storage_quota: 0,
        },
    )
    .await
    .unwrap();

    // insert：created_at == updated_at
    assert_eq!(t.created_at, t.updated_at);

    // sleep 一小段，保证时间戳差异可测。
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    let updated = tenant::update(&pool, t.id, Some("disabled"), Some(42))
        .await
        .unwrap()
        .expect("tenant updated");

    // update：created_at 不变，updated_at 刷新
    assert_eq!(updated.created_at, t.created_at);
    assert!(updated.updated_at > t.updated_at);
    assert_eq!(updated.status, "disabled");
    assert_eq!(updated.gpu_quota, 42);
}

#[tokio::test]
async fn soft_delete_hides_row_by_default_but_visible_unscoped() {
    let pool = memory_pool().await;

    let t = tenant::create(
        &pool,
        NewTenant {
            name: "sd-tenant",
            description: "",
            status: "active",
            gpu_quota: 0,
            cpu_quota: 0,
            memory_quota: 0,
            storage_quota: 0,
        },
    )
    .await
    .unwrap();

    let ok = tenant::soft_delete(&pool, t.id).await.unwrap();
    assert!(ok);

    // 默认查询查不到
    assert!(tenant::get_by_id(&pool, t.id, false)
        .await
        .unwrap()
        .is_none());
    // include_deleted 查得到，且 deleted_at 非空
    let recycled = tenant::get_by_id(&pool, t.id, true)
        .await
        .unwrap()
        .expect("soft-deleted row visible with unscoped");
    assert!(recycled.is_deleted());
    assert!(recycled.deleted_at().is_some());

    // 二次软删除应失败（已软删）。
    let again = tenant::soft_delete(&pool, t.id).await.unwrap();
    assert!(!again);
}

// ---------------------------------------------------------------------------
// 分页行为
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_pagination_applies_limit_offset() {
    let pool = memory_pool().await;

    // 塞 25 条租户。
    for i in 0..25 {
        tenant::create(
            &pool,
            NewTenant {
                name: &format!("page-tenant-{i}"),
                description: "",
                status: "active",
                gpu_quota: 0,
                cpu_quota: 0,
                memory_quota: 0,
                storage_quota: 0,
            },
        )
        .await
        .unwrap();
    }

    // page=2, page_size=10 → 第 11..=20 条，total=25，total_pages=3。
    let page = tenant::list(&pool, PaginationParams::new(2, 10), false)
        .await
        .unwrap();
    assert_eq!(page.total, 25);
    assert_eq!(page.page, 2);
    assert_eq!(page.page_size, 10);
    assert_eq!(page.total_pages, 3);
    assert_eq!(page.data.len(), 10);

    // 最后一页只有 5 条。
    let last = tenant::list(&pool, PaginationParams::new(3, 10), false)
        .await
        .unwrap();
    assert_eq!(last.data.len(), 5);
}

// ---------------------------------------------------------------------------
// 三模型 CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn user_crud_through_orm_layer() {
    let pool = memory_pool().await;

    let u = user::create(
        &pool,
        NewUser {
            username: "orm_bob",
            email: "orm_bob@example.com",
            password_hash: "hash",
            role: "user",
            tenant_id: 1,
        },
    )
    .await
    .unwrap();

    let got = user::get_by_id(&pool, u.id, false).await.unwrap().unwrap();
    assert_eq!(got.username, "orm_bob");

    user::touch_updated_at(&pool, u.id).await.unwrap();
    user::soft_delete(&pool, u.id).await.unwrap();
    assert!(user::get_by_id(&pool, u.id, false).await.unwrap().is_none());
}

#[tokio::test]
async fn tenant_crud_through_orm_layer() {
    let pool = memory_pool().await;

    let t = tenant::create(
        &pool,
        NewTenant {
            name: "crud-tenant",
            description: "desc",
            status: "active",
            gpu_quota: 10,
            cpu_quota: 100,
            memory_quota: 1000,
            storage_quota: 10000,
        },
    )
    .await
    .unwrap();

    let got = tenant::get_by_id(&pool, t.id, false)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.description, "desc");
    assert_eq!(got.gpu_quota, 10);

    tenant::soft_delete(&pool, t.id).await.unwrap();
    assert!(tenant::get_by_id(&pool, t.id, false)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn cluster_crud_and_json_field_roundtrip() {
    let pool = memory_pool().await;

    let c = cluster::create(
        &pool,
        NewCluster {
            name: "gpu-cluster-1",
            description: "primary",
            status: "active",
            nodes: 5,
            gpus: 40,
            cpus: 256,
            memory: 2048,
            storage: 100,
            network_type: "InfiniBand",
            location: "US-West",
            gpu_vendors: vec!["nvidia".into(), "amd".into()],
            scheduler_types: vec!["kubernetes".into(), "slurm".into()],
            multi_cluster_enabled: true,
            federation_id: "fed-1",
        },
    )
    .await
    .unwrap();

    let got = cluster::get_by_id(&pool, c.id, false)
        .await
        .unwrap()
        .unwrap();

    // Json<T> 往返：Vec<String> 落 TEXT 再读回。
    assert_eq!(
        got.gpu_vendors.inner(),
        &vec!["nvidia".to_string(), "amd".to_string()]
    );
    assert_eq!(
        got.scheduler_types.inner(),
        &vec!["kubernetes".to_string(), "slurm".to_string()]
    );
    assert!(got.multi_cluster_enabled);
    assert_eq!(got.federation_id, "fed-1");

    // update_status 自动刷时间戳。
    let after = cluster::update_status(&pool, c.id, "maintenance")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.status, "maintenance");

    cluster::soft_delete(&pool, c.id).await.unwrap();
    assert!(cluster::get_by_id(&pool, c.id, false)
        .await
        .unwrap()
        .is_none());
}
