from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import uvicorn
import threading
import time
import socket
import argparse
from nacos import NacosClient

# 定义请求模型
class CalculateRequest(BaseModel):
    a: float
    b: float

# 创建FastAPI应用
app = FastAPI(title="计算服务API", description="提供加、减、乘、除运算的API服务")

# 加法API
@app.post("/add", response_model=float)
async def add(request: CalculateRequest):
    """加法运算"""
    return request.a + request.b

# 减法API
@app.post("/subtract", response_model=float)
async def subtract(request: CalculateRequest):
    """减法运算"""
    return request.a - request.b

# 乘法API
@app.post("/multiply", response_model=float)
async def multiply(request: CalculateRequest):
    """乘法运算"""
    return request.a * request.b

# 除法API
@app.post("/divide", response_model=float)
async def divide(request: CalculateRequest):
    """除法运算"""
    if request.b == 0:
        raise HTTPException(status_code=400, detail="除数不能为零")
    return request.a / request.b

# 根路径
@app.get("/")
async def root():
    """根路径，返回服务信息"""
    return {
        "message": "计算服务API",
        "version": "1.0.0",
        "endpoints": {
            "add": "/add - 加法运算",
            "subtract": "/subtract - 减法运算", 
            "multiply": "/multiply - 乘法运算",
            "divide": "/divide - 除法运算"
        }
    }

def get_local_ip():
    """获取本机IP地址"""
    try:
        # 创建一个UDP套接字连接到公共DNS服务器
        # 这不会实际发送数据，只是用来获取本机IP
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
            s.connect(("8.8.8.8", 80))
            local_ip = s.getsockname()[0]
        return local_ip
    except Exception:
        # 如果获取失败，返回回环地址
        return "127.0.0.1"

def register_to_nacos(port=8002):
    """将服务注册到Nacos"""
    # Nacos服务器地址
    server_addresses = "127.0.0.1:8848"  # 默认Nacos地址，可根据实际情况修改
    namespace = "dev"
    group = "nacos-sdk-python"
    service_name = "calculate"
    
    # 获取本机IP
    local_ip = get_local_ip()
    
    # 创建Nacos客户端
    client = NacosClient(server_addresses=server_addresses, namespace=namespace)
    
    # 服务实例信息
    instance_config = {
        "ip": local_ip,      # 动态获取的本机IP
        "port": port,        # 服务端口
        "weight": 1.0,       # 权重
        "enabled": True,     # 是否启用
        "healthy": True,     # 是否健康
        "metadata": {},      # 元数据
        "cluster_name": "DEFAULT",  # 集群名称
        "service_name": service_name,
        "group_name": group,
        "namespace": namespace,
        "ephemeral": True    # 是否为临时实例
    }
    
    try:
        # 注册服务实例
        client.add_naming_instance(
            service_name=service_name,
            group_name=group,
            ip=instance_config["ip"],
            port=instance_config["port"],
            weight=instance_config["weight"],
            enabled=instance_config["enabled"],
            healthy=instance_config["healthy"],
            metadata=instance_config["metadata"],
            cluster_name=instance_config["cluster_name"],
            ephemeral=instance_config["ephemeral"]
        )
        print(f"服务 {service_name} 已成功注册到Nacos，IP: {local_ip}，端口: {port}")
    except Exception as e:
        print(f"注册服务到Nacos失败: {e}")

def start_server(port=8002):
    """启动FastAPI服务器"""
    uvicorn.run(app, host="0.0.0.0", port=port)

if __name__ == "__main__":
    # 解析命令行参数
    parser = argparse.ArgumentParser(description="计算服务API")
    parser.add_argument("--port", type=int, default=8002, help="服务端口号，默认为8002")
    args = parser.parse_args()
    
    # 注册服务到Nacos
    register_to_nacos(args.port)
    
    # 启动FastAPI服务器
    print(f"启动计算服务API服务器，端口: {args.port}...")
    start_server(args.port)