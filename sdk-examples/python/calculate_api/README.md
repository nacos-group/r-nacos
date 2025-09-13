# 计算服务 API 示例

这是一个基于 FastAPI 的计算服务示例，演示了如何将 Python 微服务注册到 r-nacos 服务注册中心。

可用于验证r-nacos内置mcp server与接口转发功能：
> 在r-nacos配置对应的mcp服务后，对应mcp服务可调用到具体业务服务实例。

## 功能特性

- 提供基本的数学运算 API（加、减、乘、除）
- 支持多种请求格式：JSON、Form、URL参数
- 自动获取本机 IP 地址并注册到 r-nacos
- 支持自定义端口号配置
- 完整的服务注册与发现示例

## 依赖安装

在运行此示例之前，请确保安装以下依赖：

```bash
uv sync
```

或者

```bash
pip install fastapi uvicorn pydantic nacos-sdk-python
```

## 运行方法

### 基本运行（使用默认端口 8002）


```bash
uv run calculate_api.py
```

或者

```bash
python calculate_api.py
```

### 指定自定义端口

```bash
python calculate_api.py --port 9000
```

### 运行输出示例

```
服务 calculate 已成功注册到Nacos，IP: 192.168.1.100，端口: 8002
启动计算服务API服务器，端口: 8002...
```

## 配置说明

### 端口配置

- **默认端口**：8002
- **自定义端口**：通过 `--port` 命令行参数指定
- **监听地址**：0.0.0.0（所有网络接口）

### IP 地址获取

服务会自动获取本机 IP 地址进行注册：
1. 优先通过 UDP 套接字连接获取实际网络 IP
2. 如果获取失败，则使用回环地址 127.0.0.1

### r-nacos 配置

默认连接到本地的 r-nacos 服务：
- **服务器地址**：127.0.0.1:8848
- **命名空间**：dev
- **分组**：nacos-sdk-python
- **服务名**：calculate

## API 接口说明

### 基础信息

- **服务地址**：http://localhost:8002（或自定义端口）
- **API 前缀**：无

### 接口列表

#### 1. 根路径

获取服务基本信息

```http
GET /
```

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

#### 2. 加法运算

```http
POST /add
Content-Type: application/json

{
  "a": 10,
  "b": 5
}
```

**响应**：
```json
15.0
```

#### 3. 减法运算

```http
POST /subtract
Content-Type: application/json

{
  "a": 10,
  "b": 5
}
```

**响应**：
```json
5.0
```

#### 4. 乘法运算

```http
POST /multiply
Content-Type: application/json

{
  "a": 10,
  "b": 5
}
```

**响应**：
```json
50.0
```

#### 5. 除法运算

```http
POST /divide
Content-Type: application/json

{
  "a": 10,
  "b": 5
}
```

**响应**：
```json
2.0
```

**错误情况**（除数为零）：
```json
{
  "detail": "除数不能为零"
}
```

#### 6. POST Form 接口

所有运算操作都支持 POST form 格式，路径前缀为 `/form/`

##### POST Form 加法运算

```http
POST /form/add
Content-Type: application/x-www-form-urlencoded

a=10&b=5
```

**响应**：
```json
15.0
```

##### POST Form 减法运算

```http
POST /form/subtract
Content-Type: application/x-www-form-urlencoded

a=10&b=5
```

**响应**：
```json
5.0
```

##### POST Form 乘法运算

```http
POST /form/multiply
Content-Type: application/x-www-form-urlencoded

a=10&b=5
```

**响应**：
```json
50.0
```

##### POST Form 除法运算

```http
POST /form/divide
Content-Type: application/x-www-form-urlencoded

a=10&b=5
```

**响应**：
```json
2.0
```

#### 7. GET Query 参数接口

所有运算操作都支持 GET 请求，通过 URL 查询参数传递数据，路径前缀为 `/q/`

##### GET Query 加法运算

```http
GET /q/add?a=10&b=5
```

**响应**：
```json
15.0
```

##### GET Query 减法运算

```http
GET /q/subtract?a=10&b=5
```

**响应**：
```json
5.0
```

##### GET Query 乘法运算

```http
GET /q/multiply?a=10&b=5
```

**响应**：
```json
50.0
```

##### GET Query 除法运算

```http
GET /q/divide?a=10&b=5
```

**响应**：
```json
2.0
```

## 示例请求

### 使用 curl

```bash
# 加法
curl -X POST "http://localhost:8002/add" \
  -H "Content-Type: application/json" \
  -d '{"a": 15, "b": 7}'

# 减法
curl -X POST "http://localhost:8002/subtract" \
  -H "Content-Type: application/json" \
  -d '{"a": 15, "b": 7}'

# 乘法
curl -X POST "http://localhost:8002/multiply" \
  -H "Content-Type: application/json" \
  -d '{"a": 15, "b": 7}'

# 除法
curl -X POST "http://localhost:8002/divide" \
  -H "Content-Type: application/json" \
  -d '{"a": 15, "b": 7}'

# POST Form 加法
curl -X POST "http://localhost:8002/form/add" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "a=15&b=7"

# POST Form 减法
curl -X POST "http://localhost:8002/form/subtract" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "a=15&b=7"

# POST Form 乘法
curl -X POST "http://localhost:8002/form/multiply" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "a=15&b=7"

# POST Form 除法
curl -X POST "http://localhost:8002/form/divide" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "a=15&b=7"

# GET Query 加法
curl "http://localhost:8002/q/add?a=15&b=7"

# GET Query 减法
curl "http://localhost:8002/q/subtract?a=15&b=7"

# GET Query 乘法
curl "http://localhost:8002/q/multiply?a=15&b=7"

# GET Query 除法
curl "http://localhost:8002/q/divide?a=15&b=7"
```

### 使用 Python requests

```python
import requests

base_url = "http://localhost:8002"

# 加法
response = requests.post(f"{base_url}/add", json={"a": 15, "b": 7})
print(f"加法结果: {response.json()}")

# 减法
response = requests.post(f"{base_url}/subtract", json={"a": 15, "b": 7})
print(f"减法结果: {response.json()}")

# 乘法
response = requests.post(f"{base_url}/multiply", json={"a": 15, "b": 7})
print(f"乘法结果: {response.json()}")

# 除法
response = requests.post(f"{base_url}/divide", json={"a": 15, "b": 7})
print(f"除法结果: {response.json()}")

# POST Form 接口示例
print("\n=== POST Form 接口示例 ===")

# POST Form 加法
response = requests.post(f"{base_url}/form/add", data={"a": 15, "b": 7})
print(f"POST Form 加法结果: {response.json()}")

# POST Form 减法
response = requests.post(f"{base_url}/form/subtract", data={"a": 15, "b": 7})
print(f"POST Form 减法结果: {response.json()}")

# POST Form 乘法
response = requests.post(f"{base_url}/form/multiply", data={"a": 15, "b": 7})
print(f"POST Form 乘法结果: {response.json()}")

# POST Form 除法
response = requests.post(f"{base_url}/form/divide", data={"a": 15, "b": 7})
print(f"POST Form 除法结果: {response.json()}")

# GET Query 接口示例
print("\n=== GET Query 接口示例 ===")

# GET Query 加法
response = requests.get(f"{base_url}/q/add", params={"a": 15, "b": 7})
print(f"GET Query 加法结果: {response.json()}")

# GET Query 减法
response = requests.get(f"{base_url}/q/subtract", params={"a": 15, "b": 7})
print(f"GET Query 减法结果: {response.json()}")

# GET Query 乘法
response = requests.get(f"{base_url}/q/multiply", params={"a": 15, "b": 7})
print(f"GET Query 乘法结果: {response.json()}")

# GET Query 除法
response = requests.get(f"{base_url}/q/divide", params={"a": 15, "b": 7})
print(f"GET Query 除法结果: {response.json()}")
```

## 服务注册与发现

此示例展示了如何将服务注册到 r-nacos：

1. **自动注册**：服务启动时会自动将实例信息注册到 r-nacos
2. **动态 IP**：使用运行时获取的本机 IP 地址
3. **健康检查**：实例标记为健康状态
4. **临时实例**：配置为临时实例，服务下线后自动注销

## 故障排除

### 常见问题

1. **无法连接到 r-nacos**
   - 确保 r-nacos 服务正在运行
   - 检查 r-nacos 服务地址配置（默认为 127.0.0.1:8848）

2. **端口被占用**
   - 使用 `--port` 参数指定其他端口
   - 检查端口是否被其他进程占用

3. **IP 地址获取失败**
   - 检查网络连接
   - 服务会自动回退到 127.0.0.1

### 日志查看

服务启动时会输出注册信息和运行状态：
```
服务 calculate 已成功注册到Nacos，IP: 192.168.1.100，端口: 8002
启动计算服务API服务器，端口: 8002...
```

## 扩展建议

1. **配置外部化**：将 r-nacos 连接配置移到配置文件中
2. **健康检查**：添加健康检查接口
3. **优雅停机**：实现服务停机时的注销逻辑
4. **监控指标**：添加服务监控和指标收集
5. **日志优化**：完善日志记录和级别管理

## 许可证

此示例代码遵循 MIT 许可证。