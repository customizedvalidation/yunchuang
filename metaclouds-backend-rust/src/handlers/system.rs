//! 系统运行模式标识 `GET /api/v1/system/mode`（公开、只读、无 JWT）。
//!
//! 存在的唯一理由：让「模拟」不再是只有读源码才知道的隐含事实。前端据此常驻
//! 展示演示模式横幅，运维/调用方也能用一条 curl 判定当前实例是否会伪造作业
//! 完成态、合成利用率或返回 mock 集群数据（P0-1）。
//!
//! 该端点只暴露非敏感的能力状态，不含密钥、连接串或内部拓扑。

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::auth::middleware::AppState;
use crate::response::ApiResponse;

/// 当前实例的执行/数据来源模式。
#[derive(Debug, Serialize)]
pub struct ExecutionMode {
    /// `SERVER_ENV`（development / production）。
    pub environment: String,
    /// 是否存在任一模拟路径：为 true 时前端必须展示演示模式横幅。
    pub simulated: bool,
    /// 派发闭环是否开启。
    pub dispatch_enabled: bool,
    /// `simulated` / `disabled`（当前不存在真实执行器实现）。
    pub execution_mode: &'static str,
    /// K8s 只读面是否允许返回 mock 数据。
    pub k8s_simulation_mode: bool,
    /// `mock` / `unavailable`（K8S_SIMULATION_MODE=false 时 K8s 端点 fail-closed）。
    pub k8s_data_source: &'static str,
    /// 作业实耗（jobs.*_actual）是否为合成值。
    pub usage_samples_synthetic: bool,
    /// 面向人的中文说明，可直接作为横幅文案。
    pub notice: String,
}

/// `GET /api/v1/system/mode`
pub async fn mode(State(state): State<AppState>) -> Json<ApiResponse<ExecutionMode>> {
    let cfg = &state.config;
    let dispatch_simulated = cfg.dispatch_enabled;
    let k8s_mock = cfg.k8s_simulation_mode;
    let simulated = dispatch_simulated || k8s_mock;

    let notice = if simulated {
        let mut parts = Vec::new();
        if dispatch_simulated {
            parts.push("作业执行为模拟（不会真正运行算力任务，完成态与实耗均为合成数据）");
        }
        if k8s_mock {
            parts.push("K8s 集群数据来自内置 mock");
        }
        format!(
            "演示模式：{}。请勿据此评估真实算力交付或计费。",
            parts.join("；")
        )
    } else {
        "演示模拟已关闭：派发闭环停用，K8s 只读端点在接入真实客户端前不可用。".to_string()
    };

    Json(ApiResponse::success(ExecutionMode {
        environment: cfg.environment.clone(),
        simulated,
        dispatch_enabled: cfg.dispatch_enabled,
        execution_mode: if dispatch_simulated {
            "simulated"
        } else {
            "disabled"
        },
        k8s_simulation_mode: k8s_mock,
        k8s_data_source: if k8s_mock { "mock" } else { "unavailable" },
        usage_samples_synthetic: dispatch_simulated,
        notice,
    }))
}
