//! 打印 OpenAPI 3.0 spec 到 stdout（用于生成 docs/openapi-rust.json）。
//!
//! 运行：`cargo run --example dump_openapi > docs/openapi-rust.json`

use metaclouds_backend_rust::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    match ApiDoc::openapi().to_pretty_json() {
        Ok(json) => print!("{json}"),
        Err(e) => {
            eprintln!("failed to serialize openapi: {e}");
            std::process::exit(1);
        }
    }
}
