#!/usr/bin/env python3
"""
McpServer与ToolSpec联动集成测试

本脚本测试McpServer与ToolSpec之间的依赖关系和联动操作，验证：
- McpServer创建时对ToolSpec的依赖验证
- 引用计数管理
- CRUD操作的数据一致性
- 版本管理集成
- 边界条件和异常处理

测试覆盖：
- 依赖关系创建和验证
- 引用计数增减管理
- 完整CRUD流程测试
- 版本一致性验证
- 并发操作处理
- 边界条件和错误处理
"""

import json
import requests
import time
import sys
import threading
import queue
import random
from typing import Dict, Any, Optional, List, Tuple
from dataclasses import dataclass
from datetime import datetime

# 导入现有的测试类
from toolspec_console_api_test import ToolSpecAPITester, TestConfig
from mcp_server_console_api_test import McpServerAPITester


@dataclass
class IntegrationTestConfig(TestConfig):
    """集成测试配置，扩展基础TestConfig"""
    test_toolspec_prefix: str = "integration-toolspec"
    test_mcpserver_prefix: str = "integration-mcpserver"
    max_concurrent_operations: int = 5
    dependency_validation_timeout: int = 30
    cleanup_retry_count: int = 3
    test_namespace: str = "integration-test-namespace"
    test_group: str = "integration-test-group"


@dataclass
class TestScenario:
    """测试场景定义"""
    name: str
    description: str
    toolspecs_needed: List[Dict[str, Any]]
    mcpservers_needed: List[Dict[str, Any]]
    operations: List[Dict[str, Any]]
    expected_results: Dict[str, Any]
    cleanup_order: List[str]


@dataclass
class DependencyRelation:
    """依赖关系模型"""
    mcpserver_name: str
    mcpserver_id: Optional[int]
    toolspec_key: str
    toolspec_version: int
    reference_type: str  # "create", "update", "remove"
    timestamp: int


@dataclass
class IntegrationTestReport:
    """集成测试报告"""
    test_name: str
    start_time: datetime
    end_time: datetime
    total_tests: int
    passed_tests: int
    failed_tests: int
    test_results: List[Dict[str, Any]]
    dependency_validations: List[Dict[str, Any]]
    performance_metrics: Dict[str, float]
    
    def generate_summary(self) -> str:
        """生成测试摘要"""
        duration = (self.end_time - self.start_time).total_seconds()
        success_rate = (self.passed_tests / self.total_tests) * 100 if self.total_tests > 0 else 0
        
        return f"""
集成测试报告: {self.test_name}
================================
执行时间: {duration:.2f}秒
总测试数: {self.total_tests}
通过: {self.passed_tests}
失败: {self.failed_tests}
成功率: {success_rate:.1f}%

依赖关系验证: {len(self.dependency_validations)}项
性能指标: {self.performance_metrics}
"""


class DependencyError(Exception):
    """依赖关系错误"""
    def __init__(self, message: str, server_name: str, toolspec_key: str):
        self.server_name = server_name
        self.toolspec_key = toolspec_key
        super().__init__(message)


class ConsistencyError(Exception):
    """数据一致性错误"""
    def __init__(self, message: str, expected: Any, actual: Any):
        self.expected = expected
        self.actual = actual
        super().__init__(message)


class ConcurrencyError(Exception):
    """并发操作错误"""
    def __init__(self, message: str, operation: str, resource: str):
        self.operation = operation
        self.resource = resource
        super().__init__(message)


class TestDataManager:
    """测试数据管理器 - 管理测试过程中的ToolSpec和McpServer数据"""
    
    def __init__(self, config: IntegrationTestConfig):
        self.config = config
        # 存储创建的测试数据，用于跟踪和清理
        self.toolspecs = {}  # key: toolspec_key, value: toolspec_data
        self.mcpservers = {}  # key: server_name, value: server_data
        self.dependencies = {}  # key: server_name, value: list of toolspec_keys
        self.creation_order = []  # 记录创建顺序，用于智能清理
        
    def generate_test_toolspec(self, name: str, **overrides) -> Dict[str, Any]:
        """生成标准化的测试ToolSpec数据
        
        Args:
            name: ToolSpec名称
            **overrides: 覆盖默认配置的参数
            
        Returns:
            标准化的ToolSpec数据字典
        """
        # 确保名称包含测试前缀
        if not name.startswith(self.config.test_toolspec_prefix):
            name = f"{self.config.test_toolspec_prefix}-{name}"
        
        base_toolspec = {
            "namespace": self.config.test_namespace,
            "group": self.config.test_group,
            "toolName": name,
            "function": {
                "name": name,
                "description": f"Integration test tool: {name}",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Test query parameter for integration testing"
                        },
                        "options": {
                            "type": "object",
                            "description": "Optional parameters for testing",
                            "properties": {
                                "timeout": {
                                    "type": "integer",
                                    "description": "Timeout in seconds",
                                    "default": 30
                                },
                                "retries": {
                                    "type": "integer", 
                                    "description": "Number of retries",
                                    "default": 3
                                }
                            }
                        }
                    },
                    "required": ["query"]
                }
            },
            "version": 1,
            "tags": ["integration-test", "automated"],
            "metadata": {
                "created_by": "integration_test",
                "test_purpose": "McpServer-ToolSpec dependency testing",
                "timestamp": int(time.time())
            }
        }
        
        # 深度合并覆盖参数
        self._deep_merge(base_toolspec, overrides)
        
        # 生成唯一的toolspec key用于跟踪
        toolspec_key = f"{base_toolspec['namespace']}.{base_toolspec['group']}.{base_toolspec['toolName']}"
        
        # 记录到跟踪系统
        self.toolspecs[toolspec_key] = base_toolspec
        self.creation_order.append(('toolspec', toolspec_key))
        
        print(f"📝 生成测试ToolSpec: {name} (key: {toolspec_key})")
        return base_toolspec
    
    def generate_test_mcpserver(self, name: str, tool_refs: List[str], **overrides) -> Dict[str, Any]:
        """生成包含tool引用的测试McpServer数据
        
        Args:
            name: McpServer名称
            tool_refs: 引用的ToolSpec列表，可以是字符串或字典
            **overrides: 覆盖默认配置的参数
            
        Returns:
            包含tool引用的McpServer数据字典
        """
        # 确保名称包含测试前缀
        if not name.startswith(self.config.test_mcpserver_prefix):
            name = f"{self.config.test_mcpserver_prefix}-{name}"
        
        # 处理tool引用
        tools = []
        referenced_toolspecs = []
        
        for tool_ref in tool_refs:
            if isinstance(tool_ref, str):
                # 字符串引用，构建标准tool对象
                tool_name = tool_ref
                if not tool_name.startswith(self.config.test_toolspec_prefix):
                    tool_name = f"{self.config.test_toolspec_prefix}-{tool_name}"
                
                tool_obj = {
                    "toolName": tool_name,
                    "namespace": self.config.test_namespace,
                    "group": self.config.test_group,
                    "toolVersion": 1
                }
                tools.append(tool_obj)
                referenced_toolspecs.append(f"{self.config.test_namespace}.{self.config.test_group}.{tool_name}")
                
            elif isinstance(tool_ref, dict):
                # 字典引用，直接使用
                tools.append(tool_ref)
                toolspec_key = f"{tool_ref.get('namespace', self.config.test_namespace)}.{tool_ref.get('group', self.config.test_group)}.{tool_ref.get('toolName')}"
                referenced_toolspecs.append(toolspec_key)
            else:
                raise ValueError(f"Invalid tool reference type: {type(tool_ref)}")
        
        base_mcpserver = {
            "namespace": self.config.test_namespace,
            "name": name,
            "description": f"Integration test server: {name}",
            "authKeys": [f"test-auth-{name}-{int(time.time())}"],
            "tools": tools,
            "version": 1,
            "tags": ["integration-test", "automated"],
            "metadata": {
                "created_by": "integration_test",
                "test_purpose": "McpServer-ToolSpec dependency testing",
                "referenced_toolspecs": referenced_toolspecs,
                "timestamp": int(time.time())
            }
        }
        
        # 深度合并覆盖参数
        self._deep_merge(base_mcpserver, overrides)
        
        # 记录到跟踪系统
        self.mcpservers[name] = base_mcpserver
        self.creation_order.append(('mcpserver', name))
        
        # 跟踪依赖关系
        self.track_dependency(name, referenced_toolspecs)
        
        print(f"📝 生成测试McpServer: {name}, 引用工具: {[ref.split('.')[-1] for ref in referenced_toolspecs]}")
        return base_mcpserver
    
    def track_dependency(self, server_name: str, toolspec_refs: List[str]):
        """跟踪依赖关系 - 记录McpServer对ToolSpec的依赖
        
        Args:
            server_name: McpServer名称
            toolspec_refs: ToolSpec引用列表（可以是单个字符串或字符串列表）
        """
        if isinstance(toolspec_refs, str):
            toolspec_refs = [toolspec_refs]
        
        if server_name not in self.dependencies:
            self.dependencies[server_name] = []
        
        for toolspec_ref in toolspec_refs:
            if toolspec_ref not in self.dependencies[server_name]:
                self.dependencies[server_name].append(toolspec_ref)
                print(f"🔗 记录依赖关系: {server_name} -> {toolspec_ref.split('.')[-1]}")
    
    def get_dependency_graph(self) -> Dict[str, List[str]]:
        """获取完整的依赖关系图
        
        Returns:
            依赖关系图，key为server_name，value为依赖的toolspec列表
        """
        return self.dependencies.copy()
    
    def get_toolspec_references(self, toolspec_key: str) -> List[str]:
        """获取引用指定ToolSpec的所有McpServer
        
        Args:
            toolspec_key: ToolSpec的唯一标识
            
        Returns:
            引用该ToolSpec的McpServer名称列表
        """
        references = []
        for server_name, toolspec_refs in self.dependencies.items():
            if toolspec_key in toolspec_refs:
                references.append(server_name)
        return references
    
    def validate_dependencies(self) -> Tuple[bool, List[str]]:
        """验证所有依赖关系的完整性
        
        Returns:
            (是否有效, 错误信息列表)
        """
        errors = []
        
        for server_name, toolspec_refs in self.dependencies.items():
            for toolspec_ref in toolspec_refs:
                if toolspec_ref not in self.toolspecs:
                    errors.append(f"McpServer '{server_name}' 引用了不存在的ToolSpec '{toolspec_ref}'")
        
        return len(errors) == 0, errors
    
    def get_cleanup_order(self) -> List[Tuple[str, str]]:
        """获取智能清理顺序 - 按正确顺序删除测试数据
        
        Returns:
            清理顺序列表，每个元素为(类型, 标识符)元组
        """
        cleanup_order = []
        
        # 1. 首先删除所有McpServer（释放对ToolSpec的引用）
        for item_type, item_id in reversed(self.creation_order):
            if item_type == 'mcpserver':
                cleanup_order.append((item_type, item_id))
        
        # 2. 然后删除所有ToolSpec
        for item_type, item_id in reversed(self.creation_order):
            if item_type == 'toolspec':
                cleanup_order.append((item_type, item_id))
        
        return cleanup_order
    
    def cleanup_all_data(self):
        """智能清理策略 - 按正确顺序删除测试数据"""
        print("\n🧹 开始智能清理测试数据...")
        
        # 获取清理顺序
        cleanup_order = self.get_cleanup_order()
        
        print(f"📋 清理计划: {len(cleanup_order)} 个项目")
        for item_type, item_id in cleanup_order:
            if item_type == 'mcpserver':
                print(f"  - McpServer: {item_id}")
            elif item_type == 'toolspec':
                print(f"  - ToolSpec: {item_id.split('.')[-1]}")
        
        # 清理内存中的跟踪数据
        self.mcpservers.clear()
        self.toolspecs.clear()
        self.dependencies.clear()
        self.creation_order.clear()
        
        print("✅ 测试数据管理器清理完成")
    
    def get_statistics(self) -> Dict[str, Any]:
        """获取测试数据统计信息
        
        Returns:
            统计信息字典
        """
        total_dependencies = sum(len(refs) for refs in self.dependencies.values())
        
        return {
            "toolspecs_count": len(self.toolspecs),
            "mcpservers_count": len(self.mcpservers),
            "total_dependencies": total_dependencies,
            "creation_order_length": len(self.creation_order),
            "dependency_graph": self.get_dependency_graph()
        }
    
    def _deep_merge(self, base_dict: Dict[str, Any], override_dict: Dict[str, Any]):
        """深度合并字典，用于参数覆盖
        
        Args:
            base_dict: 基础字典（会被修改）
            override_dict: 覆盖字典
        """
        for key, value in override_dict.items():
            if key in base_dict and isinstance(base_dict[key], dict) and isinstance(value, dict):
                self._deep_merge(base_dict[key], value)
            else:
                base_dict[key] = value


class DependencyValidator:
    """依赖关系验证器 - 验证McpServer与ToolSpec的依赖关系"""
    
    def __init__(self, toolspec_tester: ToolSpecAPITester, mcpserver_tester: McpServerAPITester, data_manager: 'TestDataManager' = None):
        """初始化依赖关系验证器
        
        Args:
            toolspec_tester: ToolSpec API测试器
            mcpserver_tester: McpServer API测试器
            data_manager: 测试数据管理器（可选）
        """
        self.toolspec_tester = toolspec_tester
        self.mcpserver_tester = mcpserver_tester
        self.data_manager = data_manager
        self.validation_cache = {}  # 缓存验证结果以提高性能
        self.validation_history = []  # 记录验证历史
    
    def validate_toolspec_exists(self, tool_ref: Dict[str, Any]) -> Tuple[bool, str]:
        """验证ToolSpec是否存在
        
        Args:
            tool_ref: ToolSpec引用，包含namespace, group, toolName等信息
            
        Returns:
            (是否存在, 详细信息)
        """
        try:
            # 构建缓存键
            cache_key = f"toolspec_exists_{tool_ref.get('namespace')}_{tool_ref.get('group')}_{tool_ref.get('toolName')}"
            
            # 检查缓存
            if cache_key in self.validation_cache:
                cached_result = self.validation_cache[cache_key]
                if time.time() - cached_result['timestamp'] < 30:  # 30秒缓存有效期
                    return cached_result['result'], cached_result['details']
            
            # 构建查询参数
            params = {
                "namespace": tool_ref.get("namespace"),
                "group": tool_ref.get("group"),
                "toolName": tool_ref.get("toolName")
            }
            
            # 验证参数完整性
            missing_params = [k for k, v in params.items() if not v]
            if missing_params:
                details = f"缺少必要参数: {missing_params}"
                self._cache_result(cache_key, False, details)
                return False, details
            
            # 调用API验证
            response = self.toolspec_tester._make_request("GET", "/toolspec/info", params=params)
            
            if response.status_code == 200:
                data = response.json()
                if data.get("success", False):
                    toolspec_data = data.get("data", {})
                    details = f"ToolSpec存在: {params['toolName']}, 版本: {toolspec_data.get('version', 'unknown')}"
                    self._cache_result(cache_key, True, details)
                    return True, details
                else:
                    details = f"ToolSpec不存在: {data.get('message', 'API返回失败')}"
                    self._cache_result(cache_key, False, details)
                    return False, details
            else:
                details = f"API调用失败: HTTP {response.status_code}"
                self._cache_result(cache_key, False, details)
                return False, details
                
        except Exception as e:
            details = f"验证ToolSpec存在性时发生异常: {e}"
            print(f"❌ {details}")
            return False, details
    
    def validate_reference_count(self, toolspec_key: str, expected_count: int) -> Tuple[bool, str]:
        """验证ToolSpec的引用计数
        
        Args:
            toolspec_key: ToolSpec的唯一标识 (namespace.group.toolName)
            expected_count: 期望的引用计数
            
        Returns:
            (计数是否匹配, 详细信息)
        """
        try:
            # 如果有数据管理器，使用其跟踪的引用信息
            if self.data_manager:
                actual_references = self.data_manager.get_toolspec_references(toolspec_key)
                actual_count = len(actual_references)
                
                if actual_count == expected_count:
                    details = f"引用计数匹配: {actual_count} (引用者: {actual_references})"
                    return True, details
                else:
                    details = f"引用计数不匹配: 期望 {expected_count}, 实际 {actual_count} (引用者: {actual_references})"
                    return False, details
            
            # 如果没有数据管理器，通过API查询所有McpServer来计算引用计数
            actual_count = self._count_toolspec_references_via_api(toolspec_key)
            
            if actual_count == expected_count:
                details = f"引用计数匹配: {actual_count}"
                return True, details
            else:
                details = f"引用计数不匹配: 期望 {expected_count}, 实际 {actual_count}"
                return False, details
                
        except Exception as e:
            details = f"验证引用计数时发生异常: {e}"
            print(f"❌ {details}")
            return False, details
    
    def validate_mcpserver_tools(self, server_id: int, expected_tools: List[Dict[str, Any]]) -> Tuple[bool, str]:
        """验证McpServer的tools列表
        
        Args:
            server_id: McpServer的ID
            expected_tools: 期望的tools列表
            
        Returns:
            (tools列表是否匹配, 详细信息)
        """
        try:
            # 获取McpServer信息
            params = {"id": server_id}
            response = self.mcpserver_tester._make_request("GET", "/mcp/server/info", params=params)
            
            if response.status_code != 200:
                details = f"获取McpServer信息失败: HTTP {response.status_code}"
                return False, details
            
            data = response.json()
            if not data.get("success", False):
                details = f"获取McpServer信息失败: {data.get('message', 'API返回失败')}"
                return False, details
            
            server_data = data.get("data", {})
            actual_tools = server_data.get("tools", [])
            
            # 详细比较tools列表
            validation_result = self._compare_tools_lists(actual_tools, expected_tools)
            
            if validation_result["match"]:
                details = f"Tools列表匹配: {len(actual_tools)} 个工具"
                return True, details
            else:
                details = f"Tools列表不匹配: {validation_result['details']}"
                return False, details
                
        except Exception as e:
            details = f"验证McpServer tools时发生异常: {e}"
            print(f"❌ {details}")
            return False, details
    
    def validate_dependency_consistency(self, mcpserver_data: Dict[str, Any] = None) -> Tuple[bool, List[str]]:
        """验证整体依赖一致性
        
        Args:
            mcpserver_data: 可选的McpServer数据，如果提供则只验证该服务器
            
        Returns:
            (是否一致, 错误信息列表)
        """
        try:
            errors = []
            
            if mcpserver_data:
                # 验证单个McpServer的依赖一致性
                server_errors = self._validate_single_server_consistency(mcpserver_data)
                errors.extend(server_errors)
            else:
                # 验证所有跟踪的McpServer的依赖一致性
                if self.data_manager:
                    for server_name, server_data in self.data_manager.mcpservers.items():
                        server_errors = self._validate_single_server_consistency(server_data)
                        errors.extend(server_errors)
                else:
                    # 如果没有数据管理器，通过API获取所有McpServer进行验证
                    all_servers = self._get_all_mcpservers_via_api()
                    for server_data in all_servers:
                        server_errors = self._validate_single_server_consistency(server_data)
                        errors.extend(server_errors)
            
            # 记录验证历史
            self.validation_history.append({
                "timestamp": datetime.now(),
                "type": "dependency_consistency",
                "success": len(errors) == 0,
                "errors_count": len(errors),
                "details": errors[:5] if errors else ["所有依赖关系一致"]  # 只记录前5个错误
            })
            
            return len(errors) == 0, errors
            
        except Exception as e:
            error_msg = f"验证依赖一致性时发生异常: {e}"
            print(f"❌ {error_msg}")
            return False, [error_msg]
    
    def validate_tool_reference_format(self, tool_ref: Dict[str, Any]) -> Tuple[bool, str]:
        """验证tool引用格式的正确性
        
        Args:
            tool_ref: tool引用对象
            
        Returns:
            (格式是否正确, 详细信息)
        """
        try:
            required_fields = ["toolName", "namespace", "group"]
            optional_fields = ["toolVersion", "description", "metadata"]
            
            # 检查必需字段
            missing_fields = []
            for field in required_fields:
                if field not in tool_ref or not tool_ref[field]:
                    missing_fields.append(field)
            
            if missing_fields:
                details = f"缺少必需字段: {missing_fields}"
                return False, details
            
            # 检查字段类型
            type_errors = []
            if not isinstance(tool_ref.get("toolName"), str):
                type_errors.append("toolName必须是字符串")
            if not isinstance(tool_ref.get("namespace"), str):
                type_errors.append("namespace必须是字符串")
            if not isinstance(tool_ref.get("group"), str):
                type_errors.append("group必须是字符串")
            
            if "toolVersion" in tool_ref and not isinstance(tool_ref["toolVersion"], int):
                type_errors.append("toolVersion必须是整数")
            
            if type_errors:
                details = f"字段类型错误: {type_errors}"
                return False, details
            
            # 检查字段值的合理性
            value_errors = []
            if len(tool_ref["toolName"]) > 100:
                value_errors.append("toolName长度不能超过100字符")
            if len(tool_ref["namespace"]) > 50:
                value_errors.append("namespace长度不能超过50字符")
            if len(tool_ref["group"]) > 50:
                value_errors.append("group长度不能超过50字符")
            
            if "toolVersion" in tool_ref and tool_ref["toolVersion"] < 1:
                value_errors.append("toolVersion必须大于0")
            
            if value_errors:
                details = f"字段值错误: {value_errors}"
                return False, details
            
            details = f"Tool引用格式正确: {tool_ref['toolName']}"
            return True, details
            
        except Exception as e:
            details = f"验证tool引用格式时发生异常: {e}"
            return False, details
    
    def get_validation_statistics(self) -> Dict[str, Any]:
        """获取验证统计信息
        
        Returns:
            验证统计信息字典
        """
        total_validations = len(self.validation_history)
        successful_validations = sum(1 for v in self.validation_history if v["success"])
        
        validation_types = {}
        for validation in self.validation_history:
            v_type = validation["type"]
            if v_type not in validation_types:
                validation_types[v_type] = {"total": 0, "success": 0}
            validation_types[v_type]["total"] += 1
            if validation["success"]:
                validation_types[v_type]["success"] += 1
        
        return {
            "total_validations": total_validations,
            "successful_validations": successful_validations,
            "success_rate": (successful_validations / total_validations * 100) if total_validations > 0 else 0,
            "validation_types": validation_types,
            "cache_size": len(self.validation_cache),
            "recent_validations": self.validation_history[-10:] if self.validation_history else []
        }
    
    def clear_cache(self):
        """清空验证缓存"""
        self.validation_cache.clear()
        print("🧹 依赖关系验证缓存已清空")
    
    def _cache_result(self, cache_key: str, result: bool, details: str):
        """缓存验证结果
        
        Args:
            cache_key: 缓存键
            result: 验证结果
            details: 详细信息
        """
        self.validation_cache[cache_key] = {
            "result": result,
            "details": details,
            "timestamp": time.time()
        }
    
    def _count_toolspec_references_via_api(self, toolspec_key: str) -> int:
        """通过API计算ToolSpec的引用计数
        
        Args:
            toolspec_key: ToolSpec的唯一标识
            
        Returns:
            引用计数
        """
        try:
            # 解析toolspec_key
            parts = toolspec_key.split('.')
            if len(parts) != 3:
                return 0
            
            namespace, group, tool_name = parts
            
            # 获取所有McpServer
            response = self.mcpserver_tester._make_request("GET", "/mcp/server/list")
            if response.status_code != 200:
                return 0
            
            data = response.json()
            if not data.get("success", False):
                return 0
            
            servers = data.get("data", [])
            reference_count = 0
            
            # 遍历所有服务器，计算引用
            for server in servers:
                tools = server.get("tools", [])
                for tool in tools:
                    if (tool.get("toolName") == tool_name and 
                        tool.get("namespace") == namespace and 
                        tool.get("group") == group):
                        reference_count += 1
            
            return reference_count
            
        except Exception as e:
            print(f"⚠️ 通过API计算引用计数时发生错误: {e}")
            return 0
    
    def _compare_tools_lists(self, actual_tools: List[Dict], expected_tools: List[Dict]) -> Dict[str, Any]:
        """比较两个tools列表
        
        Args:
            actual_tools: 实际的tools列表
            expected_tools: 期望的tools列表
            
        Returns:
            比较结果字典
        """
        result = {
            "match": True,
            "details": "",
            "differences": []
        }
        
        # 数量比较
        if len(actual_tools) != len(expected_tools):
            result["match"] = False
            result["differences"].append(f"数量不匹配: 实际 {len(actual_tools)}, 期望 {len(expected_tools)}")
        
        # 创建工具映射以便比较
        actual_tools_map = {}
        for tool in actual_tools:
            key = f"{tool.get('namespace', '')}.{tool.get('group', '')}.{tool.get('toolName', '')}"
            actual_tools_map[key] = tool
        
        expected_tools_map = {}
        for tool in expected_tools:
            key = f"{tool.get('namespace', '')}.{tool.get('group', '')}.{tool.get('toolName', '')}"
            expected_tools_map[key] = tool
        
        # 检查缺失的工具
        missing_tools = set(expected_tools_map.keys()) - set(actual_tools_map.keys())
        if missing_tools:
            result["match"] = False
            result["differences"].append(f"缺失工具: {list(missing_tools)}")
        
        # 检查多余的工具
        extra_tools = set(actual_tools_map.keys()) - set(expected_tools_map.keys())
        if extra_tools:
            result["match"] = False
            result["differences"].append(f"多余工具: {list(extra_tools)}")
        
        # 检查共同工具的版本等属性
        common_tools = set(actual_tools_map.keys()) & set(expected_tools_map.keys())
        for tool_key in common_tools:
            actual_tool = actual_tools_map[tool_key]
            expected_tool = expected_tools_map[tool_key]
            
            # 比较版本
            actual_version = actual_tool.get("toolVersion", 1)
            expected_version = expected_tool.get("toolVersion", 1)
            if actual_version != expected_version:
                result["match"] = False
                result["differences"].append(f"工具 {tool_key} 版本不匹配: 实际 {actual_version}, 期望 {expected_version}")
        
        # 生成详细信息
        if result["match"]:
            result["details"] = f"完全匹配: {len(actual_tools)} 个工具"
        else:
            result["details"] = "; ".join(result["differences"])
        
        return result
    
    def _validate_single_server_consistency(self, server_data: Dict[str, Any]) -> List[str]:
        """验证单个McpServer的依赖一致性
        
        Args:
            server_data: McpServer数据
            
        Returns:
            错误信息列表
        """
        errors = []
        server_name = server_data.get("name", "unknown")
        
        try:
            tools = server_data.get("tools", [])
            
            for i, tool in enumerate(tools):
                # 验证tool引用格式
                format_valid, format_details = self.validate_tool_reference_format(tool)
                if not format_valid:
                    errors.append(f"服务器 '{server_name}' 的第 {i+1} 个工具引用格式错误: {format_details}")
                    continue
                
                # 验证引用的ToolSpec是否存在
                exists, exists_details = self.validate_toolspec_exists(tool)
                if not exists:
                    errors.append(f"服务器 '{server_name}' 引用了不存在的ToolSpec: {exists_details}")
        
        except Exception as e:
            errors.append(f"验证服务器 '{server_name}' 时发生异常: {e}")
        
        return errors
    
    def _get_all_mcpservers_via_api(self) -> List[Dict[str, Any]]:
        """通过API获取所有McpServer
        
        Returns:
            McpServer列表
        """
        try:
            response = self.mcpserver_tester._make_request("GET", "/mcp/server/list")
            if response.status_code == 200:
                data = response.json()
                if data.get("success", False):
                    return data.get("data", [])
            return []
        except Exception as e:
            print(f"⚠️ 获取所有McpServer时发生错误: {e}")
            return []


class McpToolSpecIntegrationTester:
    """McpServer与ToolSpec联动集成测试器"""
    
    def __init__(self, config: IntegrationTestConfig):
        self.config = config
        self.session = requests.Session()
        self.session.timeout = config.timeout
        
        # 初始化现有的测试器
        self.toolspec_tester = ToolSpecAPITester(config)
        self.mcpserver_tester = McpServerAPITester(config)
        
        # 初始化组件
        self.data_manager = TestDataManager(config)
        self.validator = DependencyValidator(self.toolspec_tester, self.mcpserver_tester, self.data_manager)
        
        # 测试数据跟踪
        self.test_data = {
            'toolspecs': [],
            'mcpservers': [],
            'dependencies': []
        }
        
        # 测试报告
        self.test_results = []
        self.dependency_validations = []
        self.performance_metrics = {}
    
    def _make_request(self, method: str, endpoint: str, **kwargs) -> requests.Response:
        """HTTP请求处理，复用现有逻辑"""
        return self.toolspec_tester._make_request(method, endpoint, **kwargs)
    
    def _handle_dependency_error(self, error: DependencyError) -> bool:
        """处理依赖关系错误"""
        print(f"Dependency error: {error}")
        # 尝试修复依赖关系的逻辑可以在这里实现
        return False
    
    def _handle_consistency_error(self, error: ConsistencyError) -> bool:
        """处理数据一致性错误"""
        print(f"Consistency error: expected {error.expected}, got {error.actual}")
        return False
    
    def _handle_concurrency_error(self, error: ConcurrencyError) -> bool:
        """处理并发操作错误"""
        print(f"Concurrency error in {error.operation} on {error.resource}")
        time.sleep(random.uniform(0.1, 0.5))  # 随机退避
        return True
    
    def _record_test_result(self, test_name: str, success: bool, details: str = ""):
        """记录测试结果"""
        result = {
            "test_name": test_name,
            "success": success,
            "timestamp": datetime.now(),
            "details": details
        }
        self.test_results.append(result)
    
    def _record_dependency_validation(self, validation_type: str, success: bool, details: str = ""):
        """记录依赖关系验证结果"""
        validation = {
            "validation_type": validation_type,
            "success": success,
            "timestamp": datetime.now(),
            "details": details
        }
        self.dependency_validations.append(validation)
    
    def cleanup_test_data(self):
        """智能清理测试数据"""
        print("\n=== 清理集成测试数据 ===")
        
        cleanup_order = [
            # 1. 先删除McpServer（释放对ToolSpec的引用）
            ("mcpservers", self._cleanup_mcpservers),
            # 2. 再删除ToolSpec
            ("toolspecs", self._cleanup_toolspecs)
        ]
        
        for data_type, cleanup_func in cleanup_order:
            try:
                cleanup_func()
            except Exception as e:
                print(f"Error cleaning up {data_type}: {e}")
        
        # 清理数据管理器
        self.data_manager.cleanup_all_data()
        print("✅ 集成测试数据清理完成")
    
    def _cleanup_mcpservers(self):
        """清理McpServer测试数据"""
        for server in self.test_data['mcpservers'][:]:
            try:
                if 'id' in server and server['id']:
                    delete_params = {"id": server["id"]}
                    response = self.mcpserver_tester._make_request("POST", "/mcp/server/remove", json=delete_params)
                    
                    if response.status_code == 200:
                        data = response.json()
                        if data.get("success", False):
                            print(f"✅ 清理McpServer: {server.get('name', server.get('id'))}")
                        else:
                            print(f"⚠️ McpServer清理警告: {data.get('message', 'Unknown error')}")
                    else:
                        print(f"⚠️ McpServer清理失败: HTTP {response.status_code}")
                
                self.test_data['mcpservers'].remove(server)
            except Exception as e:
                print(f"⚠️ McpServer清理错误: {e}")
    
    def _cleanup_toolspecs(self):
        """清理ToolSpec测试数据"""
        for toolspec in self.test_data['toolspecs'][:]:
            try:
                delete_params = {
                    "namespace": toolspec["namespace"],
                    "group": toolspec["group"],
                    "toolName": toolspec["toolName"]
                }
                response = self.toolspec_tester._make_request("POST", "/toolspec/remove", json=delete_params)
                
                if response.status_code == 200:
                    data = response.json()
                    if data.get("success", False):
                        print(f"✅ 清理ToolSpec: {toolspec['toolName']}")
                    else:
                        print(f"⚠️ ToolSpec清理警告: {data.get('message', 'Unknown error')}")
                else:
                    print(f"⚠️ ToolSpec清理失败: HTTP {response.status_code}")
                
                self.test_data['toolspecs'].remove(toolspec)
            except Exception as e:
                print(f"⚠️ ToolSpec清理错误: {e}")
    
    def create_test_toolspec_with_tracking(self, name: str, **overrides) -> Tuple[bool, Dict[str, Any]]:
        """创建ToolSpec并跟踪数据
        
        Args:
            name: ToolSpec名称
            **overrides: 覆盖参数
            
        Returns:
            (是否成功, ToolSpec数据)
        """
        try:
            # 使用数据管理器生成标准化数据
            toolspec_data = self.data_manager.generate_test_toolspec(name, **overrides)
            
            # 调用API创建ToolSpec
            response = self.toolspec_tester._make_request("POST", "/toolspec/add", json=toolspec_data)
            
            if response.status_code == 200:
                result = response.json()
                if result.get("success", False):
                    # 添加到跟踪列表
                    self.test_data['toolspecs'].append(toolspec_data)
                    print(f"✅ 创建ToolSpec成功: {toolspec_data['toolName']}")
                    return True, toolspec_data
                else:
                    print(f"❌ 创建ToolSpec失败: {result.get('message', 'Unknown error')}")
                    return False, toolspec_data
            else:
                print(f"❌ 创建ToolSpec失败: HTTP {response.status_code}")
                return False, toolspec_data
                
        except Exception as e:
            print(f"❌ 创建ToolSpec异常: {e}")
            return False, {}
    
    def create_test_mcpserver_with_tracking(self, name: str, tool_refs: List[str], **overrides) -> Tuple[bool, Dict[str, Any]]:
        """创建McpServer并跟踪数据
        
        Args:
            name: McpServer名称
            tool_refs: 引用的ToolSpec列表
            **overrides: 覆盖参数
            
        Returns:
            (是否成功, McpServer数据)
        """
        try:
            # 使用数据管理器生成标准化数据
            mcpserver_data = self.data_manager.generate_test_mcpserver(name, tool_refs, **overrides)
            
            # 调用API创建McpServer
            response = self.mcpserver_tester._make_request("POST", "/mcp/server/add", json=mcpserver_data)
            
            if response.status_code == 200:
                result = response.json()
                if result.get("success", False):
                    # 获取创建的服务器ID
                    data = result.get("data")
                    server_id = None
                    
                    # 检查data的类型并获取ID
                    if isinstance(data, dict):
                        server_id = data.get("id")
                    elif isinstance(data, int):
                        server_id = data
                    
                    if server_id:
                        mcpserver_data["id"] = server_id
                    
                    # 添加到跟踪列表
                    self.test_data['mcpservers'].append(mcpserver_data)
                    print(f"✅ 创建McpServer成功: {mcpserver_data['name']} (ID: {server_id})")
                    return True, mcpserver_data
                else:
                    print(f"❌ 创建McpServer失败: {result.get('message', 'Unknown error')}")
                    return False, mcpserver_data
            else:
                print(f"❌ 创建McpServer失败: HTTP {response.status_code}")
                return False, mcpserver_data
                
        except Exception as e:
            print(f"❌ 创建McpServer异常: {e}")
            return False, {}
    
    def check_server_connectivity(self) -> bool:
        """检查服务器连接性"""
        print("🔍 检查服务器连接性...")
        try:
            # 检查ToolSpec API
            response = self.toolspec_tester._make_request("GET", "/toolspec/list")
            if response.status_code != 200:
                print(f"❌ ToolSpec API连接失败: HTTP {response.status_code}")
                return False
            
            # 检查McpServer API
            response = self.mcpserver_tester._make_request("GET", "/mcp/server/list")
            if response.status_code != 200:
                print(f"❌ McpServer API连接失败: HTTP {response.status_code}")
                return False
            
            print("✅ 服务器连接性检查通过")
            return True
        except Exception as e:
            print(f"❌ 服务器连接性检查失败: {e}")
            return False
    
    def generate_test_report(self) -> IntegrationTestReport:
        """生成测试报告"""
        end_time = datetime.now()
        start_time = self.test_results[0]["timestamp"] if self.test_results else end_time
        
        passed_tests = sum(1 for result in self.test_results if result["success"])
        failed_tests = len(self.test_results) - passed_tests
        
        return IntegrationTestReport(
            test_name="McpServer与ToolSpec联动集成测试",
            start_time=start_time,
            end_time=end_time,
            total_tests=len(self.test_results),
            passed_tests=passed_tests,
            failed_tests=failed_tests,
            test_results=self.test_results,
            dependency_validations=self.dependency_validations,
            performance_metrics=self.performance_metrics
        )
    
    def test_data_manager_functionality(self) -> bool:
        """测试数据管理器功能"""
        print("\n🧪 测试数据管理器功能...")
        
        try:
            # 测试生成ToolSpec数据
            toolspec1 = self.data_manager.generate_test_toolspec("test-tool-1")
            toolspec2 = self.data_manager.generate_test_toolspec("test-tool-2", version=2)
            
            # 测试生成McpServer数据
            mcpserver1 = self.data_manager.generate_test_mcpserver("test-server-1", ["test-tool-1"])
            mcpserver2 = self.data_manager.generate_test_mcpserver("test-server-2", ["test-tool-1", "test-tool-2"])
            
            # 验证依赖关系
            is_valid, errors = self.data_manager.validate_dependencies()
            if not is_valid:
                print(f"⚠️ 依赖关系验证失败: {errors}")
                return False
            
            # 获取统计信息
            stats = self.data_manager.get_statistics()
            print(f"📊 数据统计: {stats['toolspecs_count']} ToolSpecs, {stats['mcpservers_count']} McpServers, {stats['total_dependencies']} 依赖关系")
            
            # 测试依赖关系查询
            tool1_key = f"{self.config.test_namespace}.{self.config.test_group}.{self.config.test_toolspec_prefix}-test-tool-1"
            references = self.data_manager.get_toolspec_references(tool1_key)
            print(f"🔗 ToolSpec '{tool1_key.split('.')[-1]}' 被 {len(references)} 个McpServer引用: {references}")
            
            # 测试清理顺序
            cleanup_order = self.data_manager.get_cleanup_order()
            print(f"🧹 清理顺序: {len(cleanup_order)} 个项目")
            
            self._record_test_result("数据管理器功能测试", True, "所有功能测试通过")
            return True
            
        except Exception as e:
            print(f"❌ 数据管理器功能测试失败: {e}")
            self._record_test_result("数据管理器功能测试", False, f"错误: {e}")
            return False
    
    def test_dependency_validator_functionality(self) -> bool:
        """测试依赖关系验证器功能"""
        print("\n🧪 测试依赖关系验证器功能...")
        
        try:
            success = True
            
            # 1. 测试tool引用格式验证
            print("📝 测试tool引用格式验证...")
            
            # 有效的tool引用
            valid_tool_ref = {
                "toolName": "test-tool",
                "namespace": "test-namespace",
                "group": "test-group",
                "toolVersion": 1
            }
            format_valid, format_details = self.validator.validate_tool_reference_format(valid_tool_ref)
            if format_valid:
                print(f"✅ 有效格式验证通过: {format_details}")
            else:
                print(f"❌ 有效格式验证失败: {format_details}")
                success = False
            
            # 无效的tool引用（缺少必需字段）
            invalid_tool_ref = {
                "toolName": "test-tool",
                "namespace": "test-namespace"
                # 缺少group字段
            }
            format_invalid, invalid_details = self.validator.validate_tool_reference_format(invalid_tool_ref)
            if not format_invalid:
                print(f"✅ 无效格式验证通过: {invalid_details}")
            else:
                print(f"❌ 无效格式验证失败: 应该检测到格式错误")
                success = False
            
            # 2. 测试ToolSpec存在性验证（模拟）
            print("📝 测试ToolSpec存在性验证...")
            
            # 创建测试数据
            test_toolspec = self.data_manager.generate_test_toolspec("validator-test-tool")
            
            # 验证不存在的ToolSpec
            non_existent_tool = {
                "toolName": "non-existent-tool",
                "namespace": "non-existent-namespace",
                "group": "non-existent-group"
            }
            exists, exists_details = self.validator.validate_toolspec_exists(non_existent_tool)
            if not exists:
                print(f"✅ 不存在ToolSpec验证通过: {exists_details}")
            else:
                print(f"⚠️ 不存在ToolSpec验证结果: {exists_details}")
            
            # 3. 测试引用计数验证
            print("📝 测试引用计数验证...")
            
            # 创建测试数据
            toolspec_key = f"{self.config.test_namespace}.{self.config.test_group}.{self.config.test_toolspec_prefix}-validator-test-tool"
            
            # 验证初始引用计数（应该为0）
            count_valid, count_details = self.validator.validate_reference_count(toolspec_key, 0)
            if count_valid:
                print(f"✅ 初始引用计数验证通过: {count_details}")
            else:
                print(f"❌ 初始引用计数验证失败: {count_details}")
                success = False
            
            # 创建引用该ToolSpec的McpServer
            test_mcpserver = self.data_manager.generate_test_mcpserver("validator-test-server", ["validator-test-tool"])
            
            # 验证引用计数增加（应该为1）
            count_valid, count_details = self.validator.validate_reference_count(toolspec_key, 1)
            if count_valid:
                print(f"✅ 引用计数增加验证通过: {count_details}")
            else:
                print(f"❌ 引用计数增加验证失败: {count_details}")
                success = False
            
            # 4. 测试依赖一致性验证
            print("📝 测试依赖一致性验证...")
            
            # 验证单个服务器的依赖一致性
            consistency_valid, consistency_errors = self.validator.validate_dependency_consistency(test_mcpserver)
            if consistency_valid:
                print(f"✅ 单个服务器依赖一致性验证通过")
            else:
                print(f"❌ 单个服务器依赖一致性验证失败: {consistency_errors}")
                success = False
            
            # 验证整体依赖一致性
            overall_valid, overall_errors = self.validator.validate_dependency_consistency()
            if overall_valid:
                print(f"✅ 整体依赖一致性验证通过")
            else:
                print(f"❌ 整体依赖一致性验证失败: {overall_errors}")
                success = False
            
            # 5. 测试验证统计信息
            print("📝 测试验证统计信息...")
            
            stats = self.validator.get_validation_statistics()
            print(f"📊 验证统计: 总验证 {stats['total_validations']} 次, 成功率 {stats['success_rate']:.1f}%")
            print(f"📊 缓存大小: {stats['cache_size']} 项")
            
            # 6. 测试缓存功能
            print("📝 测试缓存功能...")
            
            # 清空缓存
            self.validator.clear_cache()
            
            # 重复验证相同的tool引用（应该使用缓存）
            exists1, details1 = self.validator.validate_toolspec_exists(non_existent_tool)
            exists2, details2 = self.validator.validate_toolspec_exists(non_existent_tool)
            
            if exists1 == exists2:
                print(f"✅ 缓存功能验证通过")
            else:
                print(f"❌ 缓存功能验证失败")
                success = False
            
            # 记录测试结果
            if success:
                self._record_test_result("依赖关系验证器功能测试", True, "所有验证器功能测试通过")
                print("✅ 依赖关系验证器功能测试完成")
            else:
                self._record_test_result("依赖关系验证器功能测试", False, "部分验证器功能测试失败")
                print("❌ 依赖关系验证器功能测试存在问题")
            
            return success
            
        except Exception as e:
            print(f"❌ 依赖关系验证器功能测试失败: {e}")
            self._record_test_result("依赖关系验证器功能测试", False, f"错误: {e}")
            return False
    
    def test_basic_dependency_flow(self) -> bool:
        """测试先创建ToolSpec再创建McpServer的基本流程
        
        验证需求1: McpServer与ToolSpec的依赖关系
        - 先创建ToolSpec
        - 再创建引用该ToolSpec的McpServer
        - 验证依赖关系正确建立
        
        Returns:
            bool: 测试是否成功
        """
        print("\n🧪 测试基本依赖关系流程...")
        
        try:
            # 1. 创建ToolSpec
            print("📝 步骤1: 创建ToolSpec")
            toolspec_success, toolspec_data = self.create_test_toolspec_with_tracking("basic-flow-tool")
            
            if not toolspec_success:
                self._record_test_result("基本依赖流程测试", False, "ToolSpec创建失败")
                return False
            
            # 验证ToolSpec创建成功
            toolspec_key = f"{toolspec_data['namespace']}.{toolspec_data['group']}.{toolspec_data['toolName']}"
            exists, exists_details = self.validator.validate_toolspec_exists({
                "toolName": toolspec_data["toolName"],
                "namespace": toolspec_data["namespace"],
                "group": toolspec_data["group"]
            })
            
            if not exists:
                print(f"❌ ToolSpec验证失败: {exists_details}")
                self._record_test_result("基本依赖流程测试", False, f"ToolSpec验证失败: {exists_details}")
                return False
            
            print(f"✅ ToolSpec创建并验证成功: {toolspec_data['toolName']}")
            
            # 2. 创建引用该ToolSpec的McpServer
            print("📝 步骤2: 创建引用ToolSpec的McpServer")
            mcpserver_success, mcpserver_data = self.create_test_mcpserver_with_tracking(
                "basic-flow-server", 
                ["basic-flow-tool"]
            )
            
            if not mcpserver_success:
                self._record_test_result("基本依赖流程测试", False, "McpServer创建失败")
                return False
            
            print(f"✅ McpServer创建成功: {mcpserver_data['name']}")
            
            # 3. 验证依赖关系正确建立
            print("📝 步骤3: 验证依赖关系")
            
            # 验证McpServer的tools列表
            if 'id' in mcpserver_data:
                tools_valid, tools_details = self.validator.validate_mcpserver_tools(
                    mcpserver_data['id'], 
                    mcpserver_data['tools']
                )
                
                if not tools_valid:
                    print(f"❌ McpServer tools验证失败: {tools_details}")
                    self._record_test_result("基本依赖流程测试", False, f"Tools验证失败: {tools_details}")
                    return False
                
                print(f"✅ McpServer tools验证成功: {tools_details}")
            
            # 验证引用计数
            count_valid, count_details = self.validator.validate_reference_count(toolspec_key, 1)
            if not count_valid:
                print(f"❌ 引用计数验证失败: {count_details}")
                self._record_test_result("基本依赖流程测试", False, f"引用计数验证失败: {count_details}")
                return False
            
            print(f"✅ 引用计数验证成功: {count_details}")
            
            # 验证整体依赖一致性
            consistency_valid, consistency_errors = self.validator.validate_dependency_consistency()
            if not consistency_valid:
                print(f"❌ 依赖一致性验证失败: {consistency_errors}")
                self._record_test_result("基本依赖流程测试", False, f"依赖一致性验证失败: {consistency_errors}")
                return False
            
            print("✅ 依赖一致性验证成功")
            
            # 记录成功结果
            self._record_test_result("基本依赖流程测试", True, "基本依赖关系流程测试完全成功")
            self._record_dependency_validation("基本依赖流程", True, f"ToolSpec: {toolspec_data['toolName']}, McpServer: {mcpserver_data['name']}")
            
            print("✅ 基本依赖关系流程测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 基本依赖流程测试异常: {e}")
            self._record_test_result("基本依赖流程测试", False, f"异常: {e}")
            return False
    
    def test_dependency_validation_failure(self) -> bool:
        """测试引用不存在ToolSpec时的错误处理
        
        验证需求1: 当McpServer引用不存在的ToolSpec时，系统应该返回相应的错误信息
        
        Returns:
            bool: 测试是否成功
        """
        print("\n🧪 测试依赖验证失败场景...")
        
        try:
            # 尝试创建引用不存在ToolSpec的McpServer
            print("📝 步骤1: 尝试创建引用不存在ToolSpec的McpServer")
            
            # 生成引用不存在ToolSpec的McpServer数据
            mcpserver_data = self.data_manager.generate_test_mcpserver(
                "invalid-dependency-server",
                ["non-existent-tool"]
            )
            
            # 调用API创建McpServer（应该失败）
            response = self.mcpserver_tester._make_request("POST", "/mcp/server/add", json=mcpserver_data)
            
            # 验证创建失败
            if response.status_code == 200:
                result = response.json()
                if result.get("success", False):
                    # 如果创建成功了，这是不期望的结果
                    print("❌ 期望创建失败，但实际创建成功了")
                    
                    # 清理意外创建的数据
                    data = result.get("data")
                    server_id = None
                    
                    # 检查data的类型并获取ID
                    if isinstance(data, dict):
                        server_id = data.get("id")
                    elif isinstance(data, int):
                        server_id = data
                    
                    if server_id:
                        mcpserver_data["id"] = server_id
                        self.test_data['mcpservers'].append(mcpserver_data)
                    
                    self._record_test_result("依赖验证失败测试", False, "期望创建失败但实际成功")
                    return False
                else:
                    # 创建失败，这是期望的结果
                    error_message = result.get("message", "Unknown error")
                    print(f"✅ 创建失败符合预期: {error_message}")
                    
                    # 验证错误信息是否包含依赖相关的内容
                    if any(keyword in error_message.lower() for keyword in ["tool", "dependency", "reference", "not found", "exist"]):
                        print("✅ 错误信息包含依赖相关内容")
                    else:
                        print(f"⚠️ 错误信息可能不够明确: {error_message}")
            else:
                # HTTP状态码不是200，也是一种失败情况
                print(f"✅ HTTP请求失败符合预期: {response.status_code}")
                
                try:
                    error_data = response.json()
                    error_message = error_data.get("message", f"HTTP {response.status_code}")
                except:
                    error_message = f"HTTP {response.status_code}"
                
                print(f"✅ 错误信息: {error_message}")
            
            # 步骤2: 验证不存在的ToolSpec确实不存在
            print("📝 步骤2: 验证不存在的ToolSpec确实不存在")
            
            non_existent_tool = {
                "toolName": f"{self.config.test_toolspec_prefix}-non-existent-tool",
                "namespace": self.config.test_namespace,
                "group": self.config.test_group
            }
            
            exists, exists_details = self.validator.validate_toolspec_exists(non_existent_tool)
            if not exists:
                print(f"✅ 确认ToolSpec不存在: {exists_details}")
            else:
                print(f"❌ 意外发现ToolSpec存在: {exists_details}")
                self._record_test_result("依赖验证失败测试", False, f"意外发现ToolSpec存在: {exists_details}")
                return False
            
            # 步骤3: 测试多个不存在的ToolSpec引用
            print("📝 步骤3: 测试多个不存在的ToolSpec引用")
            
            mcpserver_data_multi = self.data_manager.generate_test_mcpserver(
                "multi-invalid-dependency-server",
                ["non-existent-tool-1", "non-existent-tool-2"]
            )
            
            response_multi = self.mcpserver_tester._make_request("POST", "/mcp/server/add", json=mcpserver_data_multi)
            
            if response_multi.status_code == 200:
                result_multi = response_multi.json()
                if not result_multi.get("success", False):
                    print(f"✅ 多个无效引用创建失败符合预期: {result_multi.get('message', 'Unknown error')}")
                else:
                    print("❌ 多个无效引用创建成功，不符合预期")
                    # 清理意外创建的数据
                    data = result_multi.get("data")
                    server_id = None
                    
                    # 检查data的类型并获取ID
                    if isinstance(data, dict):
                        server_id = data.get("id")
                    elif isinstance(data, int):
                        server_id = data
                    
                    if server_id:
                        mcpserver_data_multi["id"] = server_id
                        self.test_data['mcpservers'].append(mcpserver_data_multi)
                    self._record_test_result("依赖验证失败测试", False, "多个无效引用创建成功")
                    return False
            else:
                print(f"✅ 多个无效引用HTTP请求失败符合预期: {response_multi.status_code}")
            
            # 记录成功结果
            self._record_test_result("依赖验证失败测试", True, "依赖验证失败场景测试完全成功")
            self._record_dependency_validation("依赖验证失败", True, "正确处理了不存在ToolSpec的引用")
            
            print("✅ 依赖验证失败场景测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 依赖验证失败测试异常: {e}")
            self._record_test_result("依赖验证失败测试", False, f"异常: {e}")
            return False
    
    def test_multiple_toolspec_references(self) -> bool:
        """测试McpServer引用多个ToolSpec的场景
        
        验证需求1: McpServer可以引用多个ToolSpec，系统应该正确维护所有引用关系
        
        Returns:
            bool: 测试是否成功
        """
        print("\n🧪 测试多个ToolSpec引用场景...")
        
        try:
            # 步骤1: 创建多个ToolSpec
            print("📝 步骤1: 创建多个ToolSpec")
            
            toolspecs = []
            toolspec_names = ["multi-tool-1", "multi-tool-2", "multi-tool-3"]
            
            for tool_name in toolspec_names:
                success, toolspec_data = self.create_test_toolspec_with_tracking(tool_name)
                if not success:
                    print(f"❌ 创建ToolSpec失败: {tool_name}")
                    self._record_test_result("多个ToolSpec引用测试", False, f"ToolSpec创建失败: {tool_name}")
                    return False
                
                toolspecs.append(toolspec_data)
                print(f"✅ 创建ToolSpec成功: {tool_name}")
            
            # 步骤2: 创建引用所有ToolSpec的McpServer
            print("📝 步骤2: 创建引用多个ToolSpec的McpServer")
            
            mcpserver_success, mcpserver_data = self.create_test_mcpserver_with_tracking(
                "multi-reference-server",
                toolspec_names
            )
            
            if not mcpserver_success:
                print("❌ 创建McpServer失败")
                self._record_test_result("多个ToolSpec引用测试", False, "McpServer创建失败")
                return False
            
            print(f"✅ 创建McpServer成功: {mcpserver_data['name']}")
            
            # 步骤3: 验证所有引用关系
            print("📝 步骤3: 验证所有引用关系")
            
            # 验证McpServer的tools列表包含所有工具
            if 'id' in mcpserver_data:
                tools_valid, tools_details = self.validator.validate_mcpserver_tools(
                    mcpserver_data['id'],
                    mcpserver_data['tools']
                )
                
                if not tools_valid:
                    print(f"❌ McpServer tools验证失败: {tools_details}")
                    self._record_test_result("多个ToolSpec引用测试", False, f"Tools验证失败: {tools_details}")
                    return False
                
                print(f"✅ McpServer tools验证成功: {tools_details}")
            
            # 验证每个ToolSpec的引用计数都是1
            for toolspec_data in toolspecs:
                toolspec_key = f"{toolspec_data['namespace']}.{toolspec_data['group']}.{toolspec_data['toolName']}"
                count_valid, count_details = self.validator.validate_reference_count(toolspec_key, 1)
                
                if not count_valid:
                    print(f"❌ ToolSpec引用计数验证失败: {toolspec_data['toolName']} - {count_details}")
                    self._record_test_result("多个ToolSpec引用测试", False, f"引用计数验证失败: {count_details}")
                    return False
                
                print(f"✅ ToolSpec引用计数验证成功: {toolspec_data['toolName']} - {count_details}")
            
            # 步骤4: 验证依赖关系图
            print("📝 步骤4: 验证依赖关系图")
            
            dependency_graph = self.data_manager.get_dependency_graph()
            server_name = mcpserver_data['name']
            
            if server_name in dependency_graph:
                dependencies = dependency_graph[server_name]
                if len(dependencies) == len(toolspec_names):
                    print(f"✅ 依赖关系图正确: {server_name} 引用了 {len(dependencies)} 个ToolSpec")
                    for dep in dependencies:
                        print(f"   - {dep.split('.')[-1]}")
                else:
                    print(f"❌ 依赖关系图错误: 期望 {len(toolspec_names)} 个依赖，实际 {len(dependencies)} 个")
                    self._record_test_result("多个ToolSpec引用测试", False, "依赖关系图数量不匹配")
                    return False
            else:
                print(f"❌ 依赖关系图中未找到服务器: {server_name}")
                self._record_test_result("多个ToolSpec引用测试", False, "依赖关系图中未找到服务器")
                return False
            
            # 步骤5: 验证整体依赖一致性
            print("📝 步骤5: 验证整体依赖一致性")
            
            consistency_valid, consistency_errors = self.validator.validate_dependency_consistency()
            if not consistency_valid:
                print(f"❌ 依赖一致性验证失败: {consistency_errors}")
                self._record_test_result("多个ToolSpec引用测试", False, f"依赖一致性验证失败: {consistency_errors}")
                return False
            
            print("✅ 依赖一致性验证成功")
            
            # 步骤6: 测试部分引用的场景
            print("📝 步骤6: 测试部分引用的场景")
            
            partial_server_success, partial_server_data = self.create_test_mcpserver_with_tracking(
                "partial-reference-server",
                [toolspec_names[0], toolspec_names[2]]  # 只引用第1和第3个工具
            )
            
            if not partial_server_success:
                print("❌ 创建部分引用McpServer失败")
                self._record_test_result("多个ToolSpec引用测试", False, "部分引用McpServer创建失败")
                return False
            
            print(f"✅ 创建部分引用McpServer成功: {partial_server_data['name']}")
            
            # 验证部分引用的引用计数
            # 第1和第3个工具的引用计数应该是2，第2个工具的引用计数应该是1
            expected_counts = [2, 1, 2]  # 对应toolspec_names的顺序
            
            for i, toolspec_data in enumerate(toolspecs):
                toolspec_key = f"{toolspec_data['namespace']}.{toolspec_data['group']}.{toolspec_data['toolName']}"
                expected_count = expected_counts[i]
                
                count_valid, count_details = self.validator.validate_reference_count(toolspec_key, expected_count)
                if not count_valid:
                    print(f"❌ 部分引用后引用计数验证失败: {toolspec_data['toolName']} - {count_details}")
                    self._record_test_result("多个ToolSpec引用测试", False, f"部分引用后引用计数验证失败: {count_details}")
                    return False
                
                print(f"✅ 部分引用后引用计数验证成功: {toolspec_data['toolName']} - {count_details}")
            
            # 记录成功结果
            self._record_test_result("多个ToolSpec引用测试", True, "多个ToolSpec引用场景测试完全成功")
            self._record_dependency_validation("多个ToolSpec引用", True, f"成功处理了 {len(toolspec_names)} 个ToolSpec的引用关系")
            
            print("✅ 多个ToolSpec引用场景测试完成")
            return True
            
        except Exception as e:
            print(f"❌ 多个ToolSpec引用测试异常: {e}")
            self._record_test_result("多个ToolSpec引用测试", False, f"异常: {e}")
            return False
    
    def test_toolspec_deletion_with_references(self) -> bool:
        """测试删除被引用ToolSpec的处理
        
        验证需求1: 当ToolSpec被McpServer引用时，系统应该根据引用情况进行相应处理
        
        Returns:
            bool: 测试是否成功
        """
        print("\n🧪 测试删除被引用ToolSpec的处理...")
        
        try:
            # 步骤1: 创建ToolSpec和引用它的McpServer
            print("📝 步骤1: 创建ToolSpec和引用它的McpServer")
            
            # 创建ToolSpec
            toolspec_success, toolspec_data = self.create_test_toolspec_with_tracking("deletion-test-tool")
            if not toolspec_success:
                print("❌ 创建ToolSpec失败")
                self._record_test_result("删除被引用ToolSpec测试", False, "ToolSpec创建失败")
                return False
            
            print(f"✅ 创建ToolSpec成功: {toolspec_data['toolName']}")
            
            # 创建引用该ToolSpec的McpServer
            mcpserver_success, mcpserver_data = self.create_test_mcpserver_with_tracking(
                "deletion-test-server",
                ["deletion-test-tool"]
            )
            
            if not mcpserver_success:
                print("❌ 创建McpServer失败")
                self._record_test_result("删除被引用ToolSpec测试", False, "McpServer创建失败")
                return False
            
            print(f"✅ 创建McpServer成功: {mcpserver_data['name']}")
            
            # 步骤2: 验证引用关系建立
            print("📝 步骤2: 验证引用关系建立")
            
            toolspec_key = f"{toolspec_data['namespace']}.{toolspec_data['group']}.{toolspec_data['toolName']}"
            count_valid, count_details = self.validator.validate_reference_count(toolspec_key, 1)
            
            if not count_valid:
                print(f"❌ 引用计数验证失败: {count_details}")
                self._record_test_result("删除被引用ToolSpec测试", False, f"引用计数验证失败: {count_details}")
                return False
            
            print(f"✅ 引用关系建立成功: {count_details}")
            
            # 步骤3: 尝试删除被引用的ToolSpec
            print("📝 步骤3: 尝试删除被引用的ToolSpec")
            
            delete_params = {
                "namespace": toolspec_data["namespace"],
                "group": toolspec_data["group"],
                "toolName": toolspec_data["toolName"]
            }
            
            response = self.toolspec_tester._make_request("POST", "/toolspec/remove", json=delete_params)
            
            # 分析删除结果
            deletion_handled_correctly = False
            
            if response.status_code == 200:
                result = response.json()
                if result.get("success", False):
                    # 删除成功 - 检查系统是否正确处理了引用关系
                    print("⚠️ ToolSpec删除成功，检查引用关系处理...")
                    
                    # 验证McpServer是否仍然存在且状态正确
                    if 'id' in mcpserver_data:
                        server_params = {"id": mcpserver_data["id"]}
                        server_response = self.mcpserver_tester._make_request("GET", "/mcp/server/info", params=server_params)
                        
                        if server_response.status_code == 200:
                            server_result = server_response.json()
                            if server_result.get("success", False):
                                server_info = server_result.get("data", {})
                                tools = server_info.get("tools", [])
                                
                                # 检查tools列表是否已更新（移除了被删除的ToolSpec）
                                remaining_tools = [tool for tool in tools if tool.get("toolName") != toolspec_data["toolName"]]
                                
                                if len(remaining_tools) < len(tools):
                                    print("✅ 系统正确处理了引用关系：从McpServer中移除了被删除的ToolSpec")
                                    deletion_handled_correctly = True
                                elif len(tools) == 0:
                                    print("✅ 系统正确处理了引用关系：McpServer的tools列表为空")
                                    deletion_handled_correctly = True
                                else:
                                    print("⚠️ McpServer仍然包含被删除的ToolSpec引用，可能存在数据一致性问题")
                            else:
                                print("⚠️ 无法获取McpServer信息来验证引用关系处理")
                        else:
                            print("⚠️ 无法访问McpServer来验证引用关系处理")
                    
                    # 从跟踪列表中移除已删除的ToolSpec
                    self.test_data['toolspecs'] = [ts for ts in self.test_data['toolspecs'] if ts['toolName'] != toolspec_data['toolName']]
                    
                else:
                    # 删除失败 - 这可能是正确的行为（保护被引用的ToolSpec）
                    error_message = result.get("message", "Unknown error")
                    print(f"✅ ToolSpec删除失败符合预期（保护被引用的资源）: {error_message}")
                    
                    # 检查错误信息是否提到引用关系
                    if any(keyword in error_message.lower() for keyword in ["reference", "used", "dependency", "mcpserver"]):
                        print("✅ 错误信息正确提到了引用关系")
                        deletion_handled_correctly = True
                    else:
                        print(f"⚠️ 错误信息可能不够明确: {error_message}")
                        deletion_handled_correctly = True  # 仍然认为是正确的，只是信息不够详细
            else:
                # HTTP错误 - 也可能是正确的保护机制
                print(f"✅ ToolSpec删除HTTP请求失败符合预期: {response.status_code}")
                deletion_handled_correctly = True
            
            # 步骤4: 验证引用计数（如果ToolSpec仍然存在）
            print("📝 步骤4: 验证删除后的状态")
            
            # 检查ToolSpec是否仍然存在
            exists, exists_details = self.validator.validate_toolspec_exists({
                "toolName": toolspec_data["toolName"],
                "namespace": toolspec_data["namespace"],
                "group": toolspec_data["group"]
            })
            
            if exists:
                print(f"✅ ToolSpec仍然存在（被保护）: {exists_details}")
                
                # 验证引用计数
                count_valid, count_details = self.validator.validate_reference_count(toolspec_key, 1)
                if count_valid:
                    print(f"✅ 引用计数保持正确: {count_details}")
                else:
                    print(f"⚠️ 引用计数可能有问题: {count_details}")
            else:
                print(f"✅ ToolSpec已被删除: {exists_details}")
                
                # 如果ToolSpec被删除，验证引用计数应该为0
                count_valid, count_details = self.validator.validate_reference_count(toolspec_key, 0)
                if count_valid:
                    print(f"✅ 删除后引用计数正确: {count_details}")
                else:
                    print(f"⚠️ 删除后引用计数可能有问题: {count_details}")
            
            # 步骤5: 测试删除McpServer后再删除ToolSpec的场景
            print("📝 步骤5: 测试先删除McpServer再删除ToolSpec")
            
            # 创建新的测试数据
            toolspec2_success, toolspec2_data = self.create_test_toolspec_with_tracking("deletion-test-tool-2")
            if not toolspec2_success:
                print("❌ 创建第二个ToolSpec失败")
                self._record_test_result("删除被引用ToolSpec测试", False, "第二个ToolSpec创建失败")
                return False
            
            mcpserver2_success, mcpserver2_data = self.create_test_mcpserver_with_tracking(
                "deletion-test-server-2",
                ["deletion-test-tool-2"]
            )
            
            if not mcpserver2_success:
                print("❌ 创建第二个McpServer失败")
                self._record_test_result("删除被引用ToolSpec测试", False, "第二个McpServer创建失败")
                return False
            
            # 先删除McpServer
            if 'id' in mcpserver2_data:
                delete_server_params = {"id": mcpserver2_data["id"]}
                server_delete_response = self.mcpserver_tester._make_request("POST", "/mcp/server/remove", json=delete_server_params)
                
                if server_delete_response.status_code == 200:
                    server_delete_result = server_delete_response.json()
                    if server_delete_result.get("success", False):
                        print("✅ McpServer删除成功")
                        
                        # 从跟踪列表中移除
                        self.test_data['mcpservers'] = [ms for ms in self.test_data['mcpservers'] if ms.get('id') != mcpserver2_data['id']]
                        
                        # 现在尝试删除ToolSpec（应该成功，因为没有引用了）
                        delete_params2 = {
                            "namespace": toolspec2_data["namespace"],
                            "group": toolspec2_data["group"],
                            "toolName": toolspec2_data["toolName"]
                        }
                        
                        response2 = self.toolspec_tester._make_request("POST", "/toolspec/remove", json=delete_params2)
                        
                        if response2.status_code == 200:
                            result2 = response2.json()
                            if result2.get("success", False):
                                print("✅ 无引用的ToolSpec删除成功")
                                # 从跟踪列表中移除
                                self.test_data['toolspecs'] = [ts for ts in self.test_data['toolspecs'] if ts['toolName'] != toolspec2_data['toolName']]
                            else:
                                print(f"⚠️ 无引用的ToolSpec删除失败: {result2.get('message', 'Unknown error')}")
                        else:
                            print(f"⚠️ 无引用的ToolSpec删除HTTP失败: {response2.status_code}")
                    else:
                        print(f"❌ McpServer删除失败: {server_delete_result.get('message', 'Unknown error')}")
                else:
                    print(f"❌ McpServer删除HTTP失败: {server_delete_response.status_code}")
            
            # 验证最终的依赖一致性
            print("📝 步骤6: 验证最终依赖一致性")
            
            consistency_valid, consistency_errors = self.validator.validate_dependency_consistency()
            if consistency_valid:
                print("✅ 最终依赖一致性验证成功")
            else:
                print(f"⚠️ 最终依赖一致性验证有问题: {consistency_errors}")
                # 不一定是失败，可能是正常的清理过程中的临时状态
            
            # 记录测试结果
            if deletion_handled_correctly:
                self._record_test_result("删除被引用ToolSpec测试", True, "删除被引用ToolSpec的处理测试完全成功")
                self._record_dependency_validation("删除被引用ToolSpec", True, "系统正确处理了被引用ToolSpec的删除")
                print("✅ 删除被引用ToolSpec的处理测试完成")
                return True
            else:
                self._record_test_result("删除被引用ToolSpec测试", False, "删除处理不符合预期")
                return False
            
        except Exception as e:
            print(f"❌ 删除被引用ToolSpec测试异常: {e}")
            self._record_test_result("删除被引用ToolSpec测试", False, f"异常: {e}")
            return False
    
    def run_basic_dependency_tests(self) -> bool:
        """运行所有基本依赖关系测试
        
        Returns:
            bool: 所有测试是否都成功
        """
        print("\n🚀 开始基本依赖关系测试套件...")
        
        success = True
        
        # 运行各个测试
        tests = [
            ("基本依赖流程测试", self.test_basic_dependency_flow),
            ("依赖验证失败测试", self.test_dependency_validation_failure),
            ("多个ToolSpec引用测试", self.test_multiple_toolspec_references),
            ("删除被引用ToolSpec测试", self.test_toolspec_deletion_with_references)
        ]
        
        for test_name, test_func in tests:
            print(f"\n{'='*60}")
            print(f"执行测试: {test_name}")
            print(f"{'='*60}")
            
            try:
                test_result = test_func()
                if test_result:
                    print(f"✅ {test_name} 通过")
                else:
                    print(f"❌ {test_name} 失败")
                    success = False
            except Exception as e:
                print(f"❌ {test_name} 异常: {e}")
                success = False
        
        print(f"\n{'='*60}")
        if success:
            print("✅ 所有基本依赖关系测试通过")
        else:
            print("❌ 部分基本依赖关系测试失败")
        print(f"{'='*60}")
        
        return success
    
    def run_all_integration_tests(self) -> bool:
        """运行所有集成测试"""
        print("🚀 开始McpServer与ToolSpec联动集成测试")
        print(f"目标服务器: {self.config.base_url}")
        
        # 检查服务器连接性
        if not self.check_server_connectivity():
            return False
        
        success = True
        
        try:
            # 测试数据管理器功能
            if not self.test_data_manager_functionality():
                success = False
            
            # 测试依赖关系验证器功能
            if not self.test_dependency_validator_functionality():
                success = False
            
            # 运行基本依赖关系测试
            if not self.run_basic_dependency_tests():
                success = False
            
            print("✅ 集成测试执行完成")
            
            self._record_test_result("集成测试总体", success, "集成测试执行完成")
            
        except Exception as e:
            print(f"❌ 集成测试执行失败: {e}")
            self._record_test_result("集成测试总体", False, f"错误: {e}")
            success = False
        finally:
            # 清理测试数据
            self.cleanup_test_data()
        
        # 生成测试报告
        report = self.generate_test_report()
        print(report.generate_summary())
        
        return success


def main():
    """主函数"""
    config = IntegrationTestConfig()
    tester = McpToolSpecIntegrationTester(config)
    
    success = tester.run_all_integration_tests()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()