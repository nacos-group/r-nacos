# AGENTS.md - r-nacos 开发指南

## 构建/测试命令
- 构建：`cargo build --release`
- 测试：`python3 integration_tests/scripts/run_test.py` (运行完整集成测试)
- 单元测试：`cargo test` (运行所有单元测试)
- 单个测试：`cargo test test_name` (运行指定测试)

## 代码风格指南

### 基础规则
- Rust 2018 edition
- 使用 `#![allow(unused_imports)]` 允许未使用的导入
- Clippy 配置：`uninlined_format_args = "allow"`

### 导入规范
- 标准库导入优先，然后是第三方库，最后是本地模块
- 使用 `use` 语句按字母顺序组织
- 在 main.rs 中广泛使用 `#![allow(unused_imports)]`

### 命名约定
- 模块名：小写下划线分隔 (如 `appdata.rs`)
- 结构体：PascalCase (如 `AppShareData`)
- 函数和变量：snake_case
- 常量：SCREAMING_SNAKE_CASE

### 错误处理
- 使用 `anyhow` 进行错误处理
- 使用 `thiserror` 定义自定义错误类型
- 使用 `Result<T, Error>` 模式

### 异步编程
- 基于 tokio 运行时
- 使用 `async-trait` 处理异步特征
- Actix actor 模式用于并发处理

### 项目结构
- `src/` 主要源码，按功能模块组织 (config, naming, grpc, raft 等)
- `integration_tests/` 集成测试
- 支持工作空间 (workspace) 包含 loadtest 和 rust-client-test

### 部署相关
- 环境变量前缀：`RNACOS_`
- 默认端口：8848 (HTTP), 9848 (gRPC), 10848 (Console)
- 支持单机和集群部署模式