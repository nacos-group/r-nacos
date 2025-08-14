# ToolSpec 控制台 API 集成测试

本目录包含 ToolSpec 控制台 API 端点的集成测试。

## 文件说明

- `toolspec_console_api_test.py` - 主要集成测试脚本
- `run_toolspec_test.py` - 测试运行器，自动启动 r-nacos 并运行测试
- `README_TOOLSPEC_TESTS.md` - 本文档文件

## 前置条件

- Python 3.6+ 并安装 `requests` 库
- r-nacos 已构建为调试模式（在 r-nacos 目录下执行 `cargo build`）

## 运行测试

### 方式一：自动化测试运行器（推荐）

`run_toolspec_test.py` 脚本会自动启动 r-nacos 服务器并运行测试：

```bash
# 自动启动服务器并运行测试
./run_toolspec_test.py

# 先构建 r-nacos，然后运行测试
./run_toolspec_test.py --build

# 测试完成后保持服务器运行，便于手动检查
./run_toolspec_test.py --keep-running

# 使用自定义主机/端口
./run_toolspec_test.py --host 192.168.1.100 --port 8849
```

### 方式二：手动管理服务器

如果您希望手动管理 r-nacos 服务器：

1. 启动 r-nacos 服务器：
```bash
cd r-nacos
cargo run
```

2. 对运行中的服务器执行测试：
```bash
./toolspec_console_api_test.py

# 或使用自定义参数
./toolspec_console_api_test.py --host 127.0.0.1 --port 8848 --namespace my-test-ns
```

## 测试覆盖范围

集成测试涵盖以下场景：

### 1. 查询 ToolSpec 列表 (`GET /rnacos/api/console/v2/toolspec/list`)
- 无参数的基本查询
- 带分页的查询（`pageNo`, `pageSize`）
- 带过滤条件的查询（`namespaceId`, `groupFilter`, `toolNameFilter`）
- 无效分页参数（验证测试）

### 2. 获取单个 ToolSpec (`GET /rnacos/api/console/v2/toolspec/info`)
- 通过键值获取已存在的 ToolSpec（namespace, group, toolName）
- 获取不存在的 ToolSpec（未找到处理）
- 使用无效参数获取（验证测试）

### 3. 创建/更新 ToolSpec (`POST /rnacos/api/console/v2/toolspec/add` 和 `POST /rnacos/api/console/v2/toolspec/update`)
- 使用完整参数创建有效的 ToolSpec
- 使用无效参数创建 ToolSpec（验证测试）
- 更新已存在的 ToolSpec 为新版本
- 创建多个 ToolSpec 用于测试

### 4. 删除 ToolSpec (`POST /rnacos/api/console/v2/toolspec/remove`)
- 删除已存在的 ToolSpec
- 删除不存在的 ToolSpec（未找到处理）
- 使用无效参数删除（验证测试）

### 5. 数据一致性测试
- 完整的 CRUD 循环（创建 → 读取 → 更新 → 删除）
- 操作后列表计数一致性
- 更新后版本一致性
- 通过后续读取验证操作结果

### 6. 数据结构验证测试
- 验证 ToolSpec 数据使用正确的 `function` 字段结构
- 验证 `function` 字段包含必需的子字段（name, description, parameters）
- 确保测试数据符合预期的 API 格式

### 7. 错误处理测试
- 参数验证错误
- 未找到错误
- 系统错误
- 正确的 HTTP 状态码和错误消息

## 测试数据结构

测试使用以下 ToolSpec 结构（注意：使用 `function` 字段而不是旧的 `parameters` 字段）：

```json
{
  "namespace": "test-namespace",
  "group": "test-group", 
  "toolName": "test-tool",
  "function": {
    "name": "test-tool",
    "description": "Test tool for test-tool",
    "parameters": {
      "input": {
        "type": "object",
        "properties": {
          "query": {
            "type": "string",
            "description": "The query string"
          },
          "limit": {
            "type": "integer", 
            "description": "Maximum number of results"
          }
        },
        "required": ["query"]
      }
    }
  },
  "version": 1
}
```

## 预期的 API 端点

测试期望以下 API 端点可用：

- `GET /rnacos/api/console/v2/toolspec/list` - 查询 ToolSpec 列表
- `GET /rnacos/api/console/v2/toolspec/info` - 获取单个 ToolSpec
- `POST /rnacos/api/console/v2/toolspec/add` - 创建 ToolSpec
- `POST /rnacos/api/console/v2/toolspec/update` - 更新 ToolSpec  
- `POST /rnacos/api/console/v2/toolspec/remove` - 删除 ToolSpec

## 响应格式

所有端点应返回以下格式的响应：

```json
{
  "success": true|false,
  "message": "error message if success=false",
  "data": {
    // Response data varies by endpoint
  }
}
```

对于列表查询，数据应包含：
```json
{
  "totalCount": 123,
  "list": [...]
}
```

## 故障排除

### 服务器连接问题
- 确保 r-nacos 在预期的主机:端口上运行
- 检查防火墙设置
- 验证控制台 API 端点已启用

### 测试失败
- 检查 r-nacos 服务器日志中的错误
- 验证 ToolSpec 控制台 API 实现是否完整
- 确保 API 端点中有适当的错误处理

### 权限问题
- 确保脚本可执行（`chmod +x *.py`）
- 验证 Python 有创建临时目录的必要权限

## 自定义配置

您可以自定义测试参数：

```bash
# 自定义测试命名空间和标识符
./toolspec_console_api_test.py --namespace my-ns --group my-group --tool-name my-tool

# 自定义服务器位置
./toolspec_console_api_test.py --host 192.168.1.100 --port 8849

# 自定义超时时间
./toolspec_console_api_test.py --timeout 30
```

## CI/CD 集成

测试脚本返回适当的退出码：
- `0` - 所有测试通过
- `1` - 部分测试失败
- `130` - 用户中断（Ctrl+C）

CI 使用示例：
```bash
#!/bin/bash
cd r-nacos/integration_tests/scripts
./run_toolspec_test.py --build
exit_code=$?
if [ $exit_code -eq 0 ]; then
    echo "✅ ToolSpec API tests passed"
else
    echo "❌ ToolSpec API tests failed with code $exit_code"
    exit $exit_code
fi
```