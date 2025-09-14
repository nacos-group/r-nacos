# Calculate API - 计算服务API

这是一个基于Rust和Actix Web构建的计算服务API，提供基本的数学运算功能（加、减、乘、除），并将服务注册到 r-nacos 服务注册中心。

可用于验证r-nacos内置mcp server与接口转发功能：
> 在r-nacos配置对应的mcp服务后，对应mcp服务可调用到具体业务服务实例。

## 功能特性

- **多种API接口格式**：支持JSON、Form表单和Query参数三种请求方式
- **基本数学运算**：提供加法、减法、乘法、除法运算
- **服务注册发现**：集成Nacos，支持自动服务注册
- **错误处理**：完善的错误处理机制，如除零错误检查
- **RESTful API**：遵循REST设计原则的API接口

## 技术栈

- **Rust** - 系统编程语言
- **Actix Web** - 高性能Web框架
- **Nacos** - 服务注册与发现中心
- **Serde** - 序列化/反序列化库
- **Tokio** - 异步运行时

## 项目结构

```
calculate-api/
├── src/
│   └── main.rs          # 主程序入口
├── Cargo.toml           # 项目配置文件
├── README.md            # 项目说明文档
└── .gitignore           # Git忽略文件
```

## 依赖项

主要依赖项包括：
- `actix-web` - Web框架
- `serde` - JSON序列化
- `nacos_rust_client` - Nacos客户端
- `tokio` - 异步运行时
- `clap` - 命令行参数解析
- `reqwest` - HTTP客户端
- `env_logger` - 日志记录

## 安装与运行

### 前置条件

- Rust 1.70+ 版本
- Nacos服务（可选，用于服务注册发现）

### 运行

#### 基本运行

切换到当前目录下，并执行以下命令

```bash
# 使用默认配置运行（端口8002，Nacos服务器127.0.0.1:8848）
cargo run

# 或者运行编译后的二进制文件
./target/release/calculate-api
```

#### 自定义配置运行

```bash
# 指定端口
cargo run -- --port 8080

# 指定Nacos服务器地址
cargo run -- --nacos-server "192.168.1.100:8848"

# 指定命名空间
cargo run -- --namespace "prod"

# 完整自定义配置
cargo run -- --port 9000 --nacos-server "192.168.1.100:8848" --namespace "test"
```

#### 命令行参数

| 参数 | 短参数 | 描述 | 默认值 |
|------|--------|------|--------|
| `--port` | `-p` | 服务监听端口 | 8002 |
| `--nacos-server` | `-s` | Nacos服务器地址 | 127.0.0.1:8848 |
| `--namespace` | `-N` | Nacos命名空间 | dev |

## API接口文档

### 基础信息

- **服务地址**：`http://localhost:8002`
- **Content-Type**：`application/json`

### 根路径

**GET** `/`

获取API服务信息和可用端点列表。

**响应示例**：
```json
{
  "message": "计算服务API",
  "version": "1.0.0",
  "endpoints": {
    "POST JSON": {
      "add": "/add - 加法运算",
      "subtract": "/subtract - 减法运算",
      "multiply": "/multiply - 乘法运算",
      "divide": "/divide - 除法运算"
    },
    "POST Form": {
      "add": "/form/add - 加法运算",
      "subtract": "/form/subtract - 减法运算",
      "multiply": "/form/multiply - 乘法运算",
      "divide": "/form/divide - 除法运算"
    },
    "GET Query": {
      "add": "/q/add?a=1&b=2 - 加法运算",
      "subtract": "/q/subtract?a=1&b=2 - 减法运算",
      "multiply": "/q/multiply?a=1&b=2 - 乘法运算",
      "divide": "/q/divide?a=1&b=2 - 除法运算"
    }
  }
}
```

### JSON格式接口

#### 加法运算

**POST** `/add`

**请求体**：
```json
{
  "a": 10.5,
  "b": 5.5
}
```

**响应**：
```json
{
  "result": 16.0
}
```

#### 减法运算

**POST** `/subtract`

**请求体**：
```json
{
  "a": 10.5,
  "b": 5.5
}
```

**响应**：
```json
{
  "result": 5.0
}
```

#### 乘法运算

**POST** `/multiply`

**请求体**：
```json
{
  "a": 10.5,
  "b": 2.0
}
```

**响应**：
```json
{
  "result": 21.0
}
```

#### 除法运算

**POST** `/divide`

**请求体**：
```json
{
  "a": 10.0,
  "b": 2.0
}
```

**成功响应**：
```json
{
  "result": 5.0
}
```

**错误响应**（除数为零）：
```json
{
  "error": "除数不能为零"
}
```

### Form表单格式接口

Form表单接口与JSON接口功能相同，只是请求格式不同。

#### 加法运算

**POST** `/form/add`

**请求体**（application/x-www-form-urlencoded）：
```
a=10.5&b=5.5
```

**响应**：
```json
{
  "result": 16.0
}
```

其他Form接口：
- `/form/subtract` - 减法运算
- `/form/multiply` - 乘法运算
- `/form/divide` - 除法运算

### Query参数格式接口

Query参数接口通过URL查询参数传递数据。

#### 加法运算

**GET** `/q/add?a=10.5&b=5.5`

**响应**：
```json
{
  "result": 16.0
}
```

其他Query接口：
- `/q/subtract?a=10.5&b=5.5` - 减法运算
- `/q/multiply?a=10.5&b=2.0` - 乘法运算
- `/q/divide?a=10.0&b=2.0` - 除法运算

## 服务注册与发现

### Nacos集成

服务启动时会自动注册到Nacos服务注册中心：

- **服务名称**：`calculate`
- **分组名称**：`dev`
- **IP地址**：自动获取本地IP地址
- **端口**：通过命令行参数指定

### 配置Nacos

确保Nacos服务正常运行，默认配置为：
- **Nacos地址**：`127.0.0.1:8848`
- **命名空间**：`dev`

可以通过命令行参数修改Nacos配置：
```bash
cargo run -- --nacos-server "192.168.1.100:8848" --namespace "prod"
```

## 使用示例

### cURL示例

#### JSON格式请求
```bash
# 加法运算
curl -X POST http://localhost:8002/add \
  -H "Content-Type: application/json" \
  -d '{"a": 10.5, "b": 5.5}'

# 除法运算
curl -X POST http://localhost:8002/divide \
  -H "Content-Type: application/json" \
  -d '{"a": 10.0, "b": 2.0}'
```

#### Form表单请求
```bash
# 乘法运算
curl -X POST http://localhost:8002/form/multiply \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "a=10.5&b=2.0"
```

#### Query参数请求
```bash
# 减法运算
curl "http://localhost:8002/q/subtract?a=10.5&b=5.5"
```
