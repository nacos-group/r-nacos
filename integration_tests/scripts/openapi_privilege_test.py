#!/usr/bin/env python3
"""
OpenAPI Privilege Integration Test

Validates that open-api (HTTP /nacos/v1, /nacos/v2 and gRPC) honors the
TokenSession namespace privilege (namespace_privilege) and edit privilege
(can_edit) the same way the console does.

Prerequisites
=============
Case 4-9 need a r-nacos instance started with open-api auth enabled:

    RNACOS_ENABLE_OPEN_API_AUTH=true

Case 10 needs a default instance (auth disabled) to verify the
backward-compatible behavior.

Both instances must have been initialized with the default admin account
(admin/admin) and must expose:
- open-api http port      (default 18848 / 28848)
- open-api grpc port      (default 19848 / 29848, http + 1000)
- console web port        (default 20848 / 30848, http + 2000)

Use `run_openapi_privilege_test.py` to start/stop the instances and run
this script automatically.

Test matrix (users created through the console api):
- admin        (init admin) : Manager, all namespaces, editable
- dev_limited  (created)    : Developer, whitelist [""] (default namespace), editable
- dev_empty    (created)    : Developer, privilege enabled with empty whitelist, editable
- visitor_user (created)    : Visitor, all namespaces, read only
"""

import base64
import json
import os
import queue
import sys
import threading
import time
import uuid

import grpc
import requests

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

AUTH_HTTP_BASE = os.environ.get("OPENAPI_PRIV_AUTH_HTTP", "http://127.0.0.1:18848")
AUTH_CONSOLE_BASE = os.environ.get("OPENAPI_PRIV_AUTH_CONSOLE", "http://127.0.0.1:20848")
AUTH_GRPC_ADDR = os.environ.get("OPENAPI_PRIV_AUTH_GRPC", "127.0.0.1:19848")

NOAUTH_HTTP_BASE = os.environ.get("OPENAPI_PRIV_NOAUTH_HTTP", "http://127.0.0.1:28848")
NOAUTH_CONSOLE_BASE = os.environ.get("OPENAPI_PRIV_NOAUTH_CONSOLE", "http://127.0.0.1:30848")

ADMIN_USER = os.environ.get("OPENAPI_PRIV_ADMIN_USER", "admin")
ADMIN_PASSWORD = os.environ.get("OPENAPI_PRIV_ADMIN_PASSWORD", "admin")

TIMEOUT = 10

CONSOLE_API_PREFIX = "/rnacos/api/console/v2"

# test users: username -> (password, roles, namespacePrivilegeParam or None)
# the default namespace id is "" (empty string) in the namespace list
TEST_USERS = {
    "dev_limited": (
        "dev_limited",
        "1",
        {"whitelistIsAll": False, "whitelist": [""]},
    ),
    "dev_empty": (
        "dev_empty",
        "1",
        {"whitelistIsAll": False},
    ),
    "visitor_user": (
        "visitor_user",
        "2",
        None,
    ),
}


def unique_id(prefix):
    return f"{prefix}_{int(time.time() * 1000)}"


# ---------------------------------------------------------------------------
# Check harness
# ---------------------------------------------------------------------------


class Check:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.failures = []

    def check(self, name, cond, detail=""):
        if cond:
            self.passed += 1
            print(f"  ✅ {name}")
        else:
            self.failed += 1
            self.failures.append(f"{name} {detail}")
            print(f"  ❌ {name} {detail}")
        return cond

    def sub(self, other: "Check"):
        self.passed += other.passed
        self.failed += other.failed
        self.failures.extend(other.failures)


# ---------------------------------------------------------------------------
# Console client (user/namespace management, console auth)
# ---------------------------------------------------------------------------


class ConsoleClient:
    def __init__(self, console_base):
        self.console_base = console_base
        self.session = requests.Session()

    def login(self, username, password):
        # console login requires base64 encoded password
        resp = self.session.post(
            f"{self.console_base}{CONSOLE_API_PREFIX}/login/login",
            data={
                "username": username,
                "password": base64.b64encode(password.encode()).decode(),
            },
            timeout=TIMEOUT,
        )
        resp.raise_for_status()
        data = resp.json()
        if not data.get("success"):
            raise RuntimeError(f"console login failed: {data}")
        token = data["data"]["token"]
        self.session.headers["Token"] = token
        return token

    def add_user(self, username, password, roles, namespace_privilege_param):
        param = {
            "username": username,
            "password": password,
            "roles": roles,
        }
        if namespace_privilege_param is not None:
            param["namespacePrivilegeParam"] = namespace_privilege_param
        resp = self.session.post(
            f"{self.console_base}{CONSOLE_API_PREFIX}/user/add",
            json=param,
            timeout=TIMEOUT,
        )
        resp.raise_for_status()
        return resp.json()

    def remove_user(self, username):
        resp = self.session.post(
            f"{self.console_base}{CONSOLE_API_PREFIX}/user/remove",
            params={"username": username},
            timeout=TIMEOUT,
        )
        return resp

    def add_namespace(self, namespace_id, namespace_name):
        resp = self.session.post(
            f"{self.console_base}{CONSOLE_API_PREFIX}/namespaces/add",
            json={"namespaceId": namespace_id, "namespaceName": namespace_name},
            timeout=TIMEOUT,
        )
        resp.raise_for_status()
        return resp.json()

    def remove_namespace(self, namespace_id):
        resp = self.session.post(
            f"{self.console_base}{CONSOLE_API_PREFIX}/namespaces/remove",
            json={"namespaceId": namespace_id},
            timeout=TIMEOUT,
        )
        return resp

    def list_namespaces(self):
        resp = self.session.get(
            f"{self.console_base}{CONSOLE_API_PREFIX}/namespaces/list",
            timeout=TIMEOUT,
        )
        resp.raise_for_status()
        return resp.json()


# ---------------------------------------------------------------------------
# Open-api HTTP client (token auth)
# ---------------------------------------------------------------------------


class OpenApiClient:
    """open-api http client for one user (accessToken auth)"""

    def __init__(self, http_base, username=None, password=None):
        self.http_base = http_base
        self.username = username
        self.password = password
        self.token = None
        self.session = requests.Session()

    def login(self):
        if self.username is None:
            return None
        resp = self.session.post(
            f"{self.http_base}/nacos/v1/auth/users/login",
            data={"username": self.username, "password": self.password},
            timeout=TIMEOUT,
        )
        resp.raise_for_status()
        data = resp.json()
        token = data.get("accessToken")
        if not token or token == "AUTH_DISABLED":
            raise RuntimeError(f"open-api login failed: {data}")
        self.token = token
        return token

    def _auth_headers(self, extra=None):
        headers = {"accessToken": self.token or ""}
        if extra:
            headers.update(extra)
        return headers

    # -- config center ----------------------------------------------------

    def publish_config(self, data_id, group="DEFAULT_GROUP", tenant="", content="hello"):
        return self.session.post(
            f"{self.http_base}/nacos/v1/cs/configs",
            data={
                "dataId": data_id,
                "group": group,
                "tenant": tenant,
                "content": content,
            },
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def query_config(self, data_id, group="DEFAULT_GROUP", tenant=""):
        return self.session.get(
            f"{self.http_base}/nacos/v1/cs/configs",
            params={"dataId": data_id, "group": group, "tenant": tenant},
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def search_config(self, tenant, search="blur", data_id="", group=""):
        return self.session.get(
            f"{self.http_base}/nacos/v1/cs/configs",
            params={
                "tenant": tenant,
                "search": search,
                "dataId": data_id,
                "group": group,
                "pageNo": 1,
                "pageSize": 20,
            },
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def delete_config(self, data_id, group="DEFAULT_GROUP", tenant=""):
        return self.session.delete(
            f"{self.http_base}/nacos/v1/cs/configs",
            params={"dataId": data_id, "group": group, "tenant": tenant},
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def listen_configs(self, items, timeout_ms=20000):
        """items: list of (data_id, group, tenant, md5)"""
        body = "".join(
            f"{data_id}\x02{group}\x02{md5}\x02{tenant}\x01"
            for data_id, group, tenant, md5 in items
        )
        return self.session.post(
            f"{self.http_base}/nacos/v1/cs/configs/listener",
            data={"Listening-Configs": body},
            headers=self._auth_headers({"Long-Pulling-Timeout": str(timeout_ms)}),
            timeout=TIMEOUT,
        )

    # -- namespace --------------------------------------------------------

    def list_namespaces_v1(self):
        return self.session.get(
            f"{self.http_base}/nacos/v1/console/namespaces",
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def list_namespaces_v2(self):
        return self.session.get(
            f"{self.http_base}/nacos/v2/console/namespace/list",
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def add_namespace_v1(self, namespace_id, name="priv-test"):
        return self.session.post(
            f"{self.http_base}/nacos/v1/console/namespaces",
            data={
                "customNamespaceId": namespace_id,
                "namespaceName": name,
            },
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def remove_namespace_v1(self, namespace_id):
        return self.session.delete(
            f"{self.http_base}/nacos/v1/console/namespaces",
            data={"namespaceId": namespace_id},
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    # -- naming -----------------------------------------------------------

    def register_instance(
        self, service_name, ip="127.0.0.1", port=8080, namespace_id="public"
    ):
        return self.session.post(
            f"{self.http_base}/nacos/v1/ns/instance",
            params={
                "serviceName": service_name,
                "ip": ip,
                "port": port,
                "weight": 1,
                "namespaceId": namespace_id,
            },
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def beat_instance(self, service_name, ip="127.0.0.1", port=8080, namespace_id="public"):
        return self.session.put(
            f"{self.http_base}/nacos/v1/ns/instance/beat",
            params={
                "serviceName": service_name,
                "ip": ip,
                "port": port,
                "namespaceId": namespace_id,
            },
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def instance_list(self, service_name, namespace_id="public"):
        return self.session.get(
            f"{self.http_base}/nacos/v1/ns/instance/list",
            params={"serviceName": service_name, "namespaceId": namespace_id},
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )

    def service_list(self, namespace_id="public", page_no=1, page_size=100):
        return self.session.get(
            f"{self.http_base}/nacos/v1/ns/service/list",
            params={
                "namespaceId": namespace_id,
                "pageNo": page_no,
                "pageSize": page_size,
            },
            headers=self._auth_headers(),
            timeout=TIMEOUT,
        )


# ---------------------------------------------------------------------------
# Minimal protobuf codec for the nacos grpc payload
#
# Payload {
#   Metadata metadata = 2;  // Metadata { type=3, headers=7(map), clientIp=8 }
#   Any body = 3;           // Any { type_url = 1, value = 2 }
# }
# ---------------------------------------------------------------------------


def _encode_varint(value):
    out = bytearray()
    while True:
        b = value & 0x7F
        value >>= 7
        if value:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def _encode_tag(field, wire_type):
    return _encode_varint((field << 3) | wire_type)


def _encode_ld(field, data: bytes):
    return _encode_tag(field, 2) + _encode_varint(len(data)) + data


def _decode_varint(buf, pos):
    result = 0
    shift = 0
    while True:
        b = buf[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        if not (b & 0x80):
            return result, pos
        shift += 7


def _decode_fields(buf: bytes):
    """return {field_number: [values...]}"""
    fields = {}
    pos = 0
    while pos < len(buf):
        tag, pos = _decode_varint(buf, pos)
        field, wire_type = tag >> 3, tag & 0x7
        if wire_type == 2:
            length, pos = _decode_varint(buf, pos)
            fields.setdefault(field, []).append(buf[pos : pos + length])
            pos += length
        elif wire_type == 0:
            value, pos = _decode_varint(buf, pos)
            fields.setdefault(field, []).append(value)
        elif wire_type == 5:
            fields.setdefault(field, []).append(buf[pos : pos + 4])
            pos += 4
        elif wire_type == 1:
            fields.setdefault(field, []).append(buf[pos : pos + 8])
            pos += 8
        else:
            raise ValueError(f"unsupported wire type {wire_type}")
    return fields


def encode_payload(metadata_type, body_str, headers):
    meta = _encode_ld(3, metadata_type.encode())
    for key, value in headers.items():
        entry = _encode_ld(1, key.encode()) + _encode_ld(2, str(value).encode())
        meta += _encode_ld(7, entry)
    body = _encode_ld(1, b"") + _encode_ld(2, body_str.encode())
    return _encode_ld(2, meta) + _encode_ld(3, body)


def decode_payload(raw: bytes):
    meta_type = ""
    body_json = {}
    fields = _decode_fields(raw)
    if 2 in fields:
        meta_fields = _decode_fields(fields[2][0])
        if 3 in meta_fields:
            meta_type = meta_fields[3][0].decode()
    if 3 in fields:
        body_fields = _decode_fields(fields[3][0])
        if 2 in body_fields:
            body_json = json.loads(body_fields[2][0].decode() or "{}")
    return meta_type, body_json


class GrpcClient:
    """raw nacos grpc client with a persistent bi-stream connection"""

    def __init__(self, grpc_addr):
        self.grpc_addr = grpc_addr
        self.channel = grpc.insecure_channel(grpc_addr)
        self.request_call = self.channel.unary_unary(
            "/Request/request",
            request_serializer=lambda p: p,
            response_deserializer=lambda p: p,
        )
        self.bi_call = self.channel.stream_stream(
            "/BiRequestStream/requestBiStream",
            request_serializer=lambda p: p,
            response_deserializer=lambda p: p,
        )
        self._send_queue = queue.Queue()
        self._response_queue = queue.Queue()
        self._closed = threading.Event()
        self._start_bi_stream()

    def _start_bi_stream(self):
        def request_iter():
            while True:
                item = self._send_queue.get()
                if item is None:
                    return
                yield item

        def response_reader(response_iter):
            try:
                for payload in response_iter:
                    self._response_queue.put(decode_payload(payload))
            except Exception:  # noqa: BLE001 - connection closed
                pass

        self._resp_iter = self.bi_call(request_iter())
        threading.Thread(
            target=response_reader, args=(self._resp_iter,), daemon=True
        ).start()
        setup_body = json.dumps(
            {
                "clientVersion": "Nacos:Python:2.0",
                "tenant": "",
                "labels": {},
                "requestId": uuid.uuid4().hex,
            }
        )
        self._send_queue.put(encode_payload("ConnectionSetupRequest", setup_body, {}))
        # wait for the connection to be registered server-side
        time.sleep(0.5)

    def request(self, request_type, body, token=""):
        headers = {"accessToken": token, "requestId": uuid.uuid4().hex}
        raw = self.request_call(
            encode_payload(request_type, json.dumps(body), headers), timeout=TIMEOUT
        )
        return decode_payload(raw)

    def close(self):
        self._closed.set()
        self._send_queue.put(None)
        try:
            self._resp_iter.cancel()
        except Exception:  # noqa: BLE001
            pass
        self.channel.close()


# ---------------------------------------------------------------------------
# test fixture: users + namespaces + grpc clients
# ---------------------------------------------------------------------------


class Fixture:
    def __init__(self, check: Check):
        self.check = check
        self.console = ConsoleClient(AUTH_CONSOLE_BASE)
        self.openapi = {}  # username -> OpenApiClient
        self.grpc = {}  # username -> GrpcClient
        self.namespaces = []
        self.created_users = []
        self.data_id = unique_id("openapi-priv-test")
        self.grpc_data_id = unique_id("openapi-priv-grpc-test")
        self.listen_data_id = unique_id("listen-priv-test")
        self.service_name = unique_id("openapi-priv-svc")
        self.grpc_service_name = unique_id("openapi-priv-grpc-svc")

    # -- setup / teardown --------------------------------------------------

    def setup(self):
        self.console.login(ADMIN_USER, ADMIN_PASSWORD)
        for username, (password, roles, privilege_param) in TEST_USERS.items():
            self.console.remove_user(username)
            result = self.console.add_user(username, password, roles, privilege_param)
            ok = self.check.check(
                f"setup: create user {username} (roles={roles})", result.get("success"), str(result)
            )
            if ok:
                self.created_users.append(username)
        # ns-x used as the "no permission" namespace
        self.console.remove_namespace("ns-x")
        result = self.console.add_namespace("ns-x", "priv-test-ns-x")
        self.check.check("setup: create namespace ns-x", result.get("success"), str(result))
        self.namespaces.append("ns-x")

        for username, (password, _roles, _p) in TEST_USERS.items():
            client = OpenApiClient(AUTH_HTTP_BASE, username, password)
            client.login()
            self.openapi[username] = client
        self.openapi["admin"] = OpenApiClient(AUTH_HTTP_BASE, ADMIN_USER, ADMIN_PASSWORD)
        self.openapi["admin"].login()
        for username in list(TEST_USERS) + ["admin"]:
            self.grpc[username] = GrpcClient(AUTH_GRPC_ADDR)

    def teardown(self):
        # best effort cleanup, errors are ignored
        admin = self.openapi.get("admin")
        if admin is not None:
            try:
                admin.delete_config(self.data_id)
            except Exception:  # noqa: BLE001
                pass
            try:
                admin.delete_config(self.listen_data_id, tenant="public")
            except Exception:  # noqa: BLE001
                pass
            try:
                admin.delete_config(self.listen_data_id, tenant="ns-x")
            except Exception:  # noqa: BLE001
                pass
        for client in self.grpc.values():
            try:
                client.close()
            except Exception:  # noqa: BLE001
                pass
        for namespace_id in reversed(self.namespaces):
            try:
                self.console.remove_namespace(namespace_id)
            except Exception:  # noqa: BLE001
                print(f"  (warn) failed to remove namespace {namespace_id}")
        for username in self.created_users:
            try:
                self.console.remove_user(username)
            except Exception:  # noqa: BLE001
                print(f"  (warn) failed to remove user {username}")


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------


def assert_403(check: Check, name, resp, expect_error_code=None):
    ok = check.check(f"{name} -> 403", resp.status_code == 403, f"got {resp.status_code}: {resp.text[:120]}")
    if ok and expect_error_code is not None:
        try:
            body = resp.json()
        except ValueError:
            body = {}
        check.check(
            f"{name} errorCode=={expect_error_code}",
            body.get("errorCode") == expect_error_code,
            f"got {body}",
        )


def grpc_error_code403(check: Check, name, result, expect_error_code=None):
    meta_type, body = result
    ok = check.check(
        f"{name} -> ErrorResponse(403)",
        meta_type == "ErrorResponse" and body.get("errorCode") == 403,
        f"got {meta_type}: {body}",
    )
    if ok and expect_error_code is not None:
        check.check(
            f"{name} message contains {expect_error_code}",
            expect_error_code in (body.get("message") or ""),
            f"got message: {body.get('message')}",
        )


# ---------------------------------------------------------------------------
# 用例4: HTTP 配置中心接口数据权限与编辑权限
# ---------------------------------------------------------------------------


def case4_http_config_privilege(check: Check, fx: Fixture):
    print("\n=== 用例4: HTTP 配置中心接口数据权限与编辑权限 ===")
    dev_empty = fx.openapi["dev_empty"]
    dev_limited = fx.openapi["dev_limited"]
    visitor = fx.openapi["visitor_user"]
    admin = fx.openapi["admin"]
    data_id = fx.data_id

    resp = dev_empty.publish_config(data_id, content="dev-empty")
    assert_403(check, "dev_empty publish public", resp, "NO_NAMESPACE_PERMISSION")

    resp = visitor.publish_config(data_id, content="visitor")
    assert_403(check, "visitor publish public (all privilege but read only)", resp, "NO_PERMISSION")

    resp = dev_limited.publish_config(data_id, content="hello")
    check.check("dev_limited publish public -> 200 true", resp.status_code == 200 and resp.text == "true", f"got {resp.status_code}: {resp.text}")

    resp = dev_limited.publish_config(data_id, tenant="ns-x", content="nsx")
    assert_403(check, "dev_limited publish ns-x", resp, "NO_NAMESPACE_PERMISSION")

    resp = dev_limited.query_config(data_id)
    check.check("dev_limited query public -> content", resp.status_code == 200 and resp.text == "hello", f"got {resp.status_code}: {resp.text[:80]}")

    resp = dev_limited.query_config(data_id, tenant="ns-x")
    assert_403(check, "dev_limited query ns-x", resp, "NO_NAMESPACE_PERMISSION")

    resp = dev_limited.search_config("ns-x")
    assert_403(check, "dev_limited search ns-x (blur)", resp, "NO_NAMESPACE_PERMISSION")

    resp = dev_limited.search_config("public")
    ok = check.check("dev_limited search public -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        items = resp.json().get("pageItems", [])
        check.check(
            "dev_limited search public contains published config",
            any(item.get("dataId") == data_id for item in items),
            f"got {[item.get('dataId') for item in items]}",
        )

    resp = visitor.query_config(data_id)
    check.check("visitor query public -> 200 (read allowed)", resp.status_code == 200 and resp.text == "hello", f"got {resp.status_code}: {resp.text[:80]}")

    resp = visitor.delete_config(data_id)
    assert_403(check, "visitor delete public (read only)", resp, "NO_PERMISSION")

    resp = admin.query_config(data_id)
    check.check("admin query public -> content", resp.status_code == 200 and resp.text == "hello", f"got {resp.status_code}: {resp.text[:80]}")

    resp = admin.delete_config(data_id)
    check.check("admin delete public -> 200 true", resp.status_code == 200 and resp.text == "true", f"got {resp.status_code}: {resp.text}")

    resp = admin.query_config(data_id)
    check.check("admin query after delete -> 404", resp.status_code == 404, f"got {resp.status_code}")

    # tenant 默认值: 不传 tenant 等价于 public
    resp = dev_limited.publish_config(data_id, content="hello-default")
    check.check("dev_limited publish without tenant -> 200 true", resp.status_code == 200 and resp.text == "true", f"got {resp.status_code}: {resp.text}")
    resp = dev_empty.publish_config(data_id, content="dev-empty-default")
    assert_403(check, "dev_empty publish without tenant", resp, "NO_NAMESPACE_PERMISSION")
    admin.delete_config(data_id)


# ---------------------------------------------------------------------------
# 用例5: HTTP 注册中心接口数据权限
# ---------------------------------------------------------------------------


def case5_http_naming_privilege(check: Check, fx: Fixture):
    print("\n=== 用例5: HTTP 注册中心接口数据权限 ===")
    dev_empty = fx.openapi["dev_empty"]
    dev_limited = fx.openapi["dev_limited"]
    visitor = fx.openapi["visitor_user"]
    admin = fx.openapi["admin"]
    service = fx.service_name

    resp = dev_empty.register_instance(service)
    assert_403(check, "dev_empty register instance public", resp, "NO_NAMESPACE_PERMISSION")

    resp = dev_limited.register_instance(service, port=18080)
    check.check("dev_limited register instance public -> ok", resp.status_code == 200 and resp.text == "ok", f"got {resp.status_code}: {resp.text}")

    resp = dev_limited.register_instance(service, port=18080, namespace_id="ns-x")
    assert_403(check, "dev_limited register instance ns-x", resp, "NO_NAMESPACE_PERMISSION")

    resp = dev_limited.instance_list(service)
    ok = check.check("dev_limited instance_list public -> 200", resp.status_code == 200, f"got {resp.status_code}: {resp.text[:80]}")
    if ok:
        hosts = resp.json().get("hosts") or []
        check.check(
            "dev_limited instance_list public contains instance",
            any(h.get("ip") == "127.0.0.1" and h.get("port") == 18080 for h in hosts),
            f"got {hosts}",
        )

    resp = dev_limited.instance_list(service, namespace_id="ns-x")
    assert_403(check, "dev_limited instance_list ns-x", resp, "NO_NAMESPACE_PERMISSION")

    resp = dev_limited.service_list("ns-x")
    assert_403(check, "dev_limited service_list ns-x", resp, "NO_NAMESPACE_PERMISSION")

    resp = dev_limited.beat_instance(service, port=18080)
    check.check("dev_limited beat instance public -> 200", resp.status_code == 200, f"got {resp.status_code}: {resp.text[:80]}")

    resp = visitor.register_instance(service, port=18081)
    check.check("visitor register instance public -> ok (naming write ignores can_edit)", resp.status_code == 200 and resp.text == "ok", f"got {resp.status_code}: {resp.text}")

    resp = admin.instance_list(service)
    ok = check.check("admin instance_list public -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        hosts = resp.json().get("hosts") or []
        check.check(
            "admin instance_list public contains both instances",
            len(hosts) >= 2,
            f"got {len(hosts)} hosts",
        )


# ---------------------------------------------------------------------------
# 用例6: HTTP 命名空间接口数据权限
# ---------------------------------------------------------------------------


def case6_http_namespace_privilege(check: Check, fx: Fixture):
    print("\n=== 用例6: HTTP 命名空间接口数据权限 ===")
    dev_empty = fx.openapi["dev_empty"]
    dev_limited = fx.openapi["dev_limited"]
    admin = fx.openapi["admin"]

    def namespace_ids(resp):
        data = resp.json().get("data", [])
        return [item.get("namespace") for item in data]

    resp = admin.list_namespaces_v1()
    ok = check.check("admin list_namespaces_v1 -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        ids = namespace_ids(resp)
        check.check(
            "admin list contains public and ns-x",
            ("ns-x" in ids) and any(i in (None, "") for i in ids),
            f"got {ids}",
        )

    resp = dev_limited.list_namespaces_v1()
    ok = check.check("dev_limited list_namespaces_v1 -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        ids = namespace_ids(resp)
        check.check(
            "dev_limited list only contains public (default ns)",
            all(i in (None, "") for i in ids) and len(ids) >= 1,
            f"got {ids}",
        )

    resp = dev_empty.list_namespaces_v1()
    ok = check.check("dev_empty list_namespaces_v1 -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        ids = namespace_ids(resp)
        check.check("dev_empty list is empty", len(ids) == 0, f"got {ids}")

    resp = dev_limited.list_namespaces_v2()
    ok = check.check("dev_limited list_namespaces_v2 -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        ids = namespace_ids(resp)
        check.check(
            "dev_limited v2 list only contains public",
            all(i in (None, "") for i in ids) and len(ids) >= 1,
            f"got {ids}",
        )

    resp = dev_limited.add_namespace_v1("ns-y")
    assert_403(check, "dev_limited add namespace ns-y", resp, "NO_NAMESPACE_PERMISSION")
    ids = namespace_ids(admin.list_namespaces_v1())
    check.check("ns-y not created by dev_limited", "ns-y" not in ids, f"got {ids}")

    resp = admin.add_namespace_v1("ns-y")
    check.check("admin add namespace ns-y -> 200 true", resp.status_code == 200 and resp.text == "true", f"got {resp.status_code}: {resp.text}")
    fx.namespaces.append("ns-y")

    resp = dev_limited.remove_namespace_v1("ns-y")
    assert_403(check, "dev_limited remove namespace ns-y", resp, "NO_NAMESPACE_PERMISSION")
    ids = namespace_ids(admin.list_namespaces_v1())
    check.check("ns-y still exists after denied removal", "ns-y" in ids, f"got {ids}")

    resp = admin.remove_namespace_v1("ns-y")
    check.check("admin remove namespace ns-y -> 200 true", resp.status_code == 200 and resp.text == "true", f"got {resp.status_code}: {resp.text}")
    fx.namespaces.remove("ns-y")


# ---------------------------------------------------------------------------
# 用例9: HTTP 配置监听权限过滤
# ---------------------------------------------------------------------------


def case9_http_listener_filter(check: Check, fx: Fixture):
    print("\n=== 用例9: HTTP 配置监听权限过滤 ===")
    dev_limited = fx.openapi["dev_limited"]
    admin = fx.openapi["admin"]
    data_id = fx.listen_data_id
    group = "DEFAULT_GROUP"

    resp = admin.publish_config(data_id, group=group, tenant="public", content="pub")
    check.check("admin publish listen config public", resp.status_code == 200, f"got {resp.status_code}: {resp.text}")
    resp = admin.publish_config(data_id, group=group, tenant="ns-x", content="nsx")
    check.check("admin publish listen config ns-x", resp.status_code == 200, f"got {resp.status_code}: {resp.text}")

    items = [
        (data_id, group, "public", "WRONG_MD5"),
        (data_id, group, "ns-x", "WRONG_MD5"),
    ]
    resp = dev_limited.listen_configs(items)
    ok = check.check("dev_limited listener -> 200", resp.status_code == 200, f"got {resp.status_code}: {resp.text[:80]}")
    if ok:
        check.check(
            "dev_limited change set only contains public item",
            data_id in resp.text and "%02ns-x" not in resp.text,
            f"got {resp.text[:200]}",
        )

    resp = admin.listen_configs(items)
    ok = check.check("admin listener -> 200", resp.status_code == 200, f"got {resp.status_code}: {resp.text[:80]}")
    if ok:
        check.check(
            "admin change set contains both items",
            data_id in resp.text and "%02ns-x" in resp.text,
            f"got {resp.text[:200]}",
        )


# ---------------------------------------------------------------------------
# 用例7: gRPC 配置中心数据权限与编辑权限
# ---------------------------------------------------------------------------


def case7_grpc_config_privilege(check: Check, fx: Fixture):
    print("\n=== 用例7: gRPC 配置中心数据权限与编辑权限 ===")
    admin = fx.openapi["admin"]
    data_id = fx.grpc_data_id

    def publish(client, tenant, content="grpc-hello"):
        return fx.grpc[client].request(
            "ConfigPublishRequest",
            {
                "dataId": data_id,
                "group": "DEFAULT_GROUP",
                "tenant": tenant,
                "content": content,
                "requestId": uuid.uuid4().hex,
            },
            token=fx.openapi[client].token,
        )

    def query(client, tenant):
        return fx.grpc[client].request(
            "ConfigQueryRequest",
            {
                "dataId": data_id,
                "group": "DEFAULT_GROUP",
                "tenant": tenant,
                "requestId": uuid.uuid4().hex,
            },
            token=fx.openapi[client].token,
        )

    def remove(client, tenant):
        return fx.grpc[client].request(
            "ConfigRemoveRequest",
            {
                "dataId": data_id,
                "group": "DEFAULT_GROUP",
                "tenant": tenant,
                "requestId": uuid.uuid4().hex,
            },
            token=fx.openapi[client].token,
        )

    meta, body = publish("visitor_user", "public")
    grpc_error_code403(check, "visitor grpc publish public", (meta, body), "NO_PERMISSION")

    meta, body = publish("dev_empty", "public")
    grpc_error_code403(check, "dev_empty grpc publish public", (meta, body), "NO_NAMESPACE_PERMISSION")

    meta, body = publish("dev_limited", "public")
    check.check(
        "dev_limited grpc publish public -> success",
        meta == "ConfigPublishResponse" and body.get("resultCode") == 200,
        f"got {meta}: {body}",
    )

    meta, body = publish("dev_limited", "ns-x")
    grpc_error_code403(check, "dev_limited grpc publish ns-x", (meta, body), "NO_NAMESPACE_PERMISSION")

    meta, body = query("dev_limited", "ns-x")
    grpc_error_code403(check, "dev_limited grpc query ns-x", (meta, body), "NO_NAMESPACE_PERMISSION")

    meta, body = query("visitor_user", "public")
    ok = check.check(
        "visitor grpc query public -> success",
        meta == "ConfigQueryResponse" and body.get("resultCode") == 200,
        f"got {meta}: {body}",
    )
    if ok:
        check.check(
            "visitor grpc query public returns content",
            body.get("content") == "grpc-hello",
            f"got {body.get('content')}",
        )

    meta, body = remove("visitor_user", "public")
    grpc_error_code403(check, "visitor grpc remove public", (meta, body), "NO_PERMISSION")

    resp = admin.query_config(data_id)
    check.check("admin http query after grpc publish -> content", resp.status_code == 200 and resp.text == "grpc-hello", f"got {resp.status_code}: {resp.text[:80]}")

    meta, body = remove("admin", "public")
    check.check(
        "admin grpc remove public -> success",
        meta == "ConfigRemoveResponse" and body.get("resultCode") == 200,
        f"got {meta}: {body}",
    )

    meta, body = query("dev_limited", "public")
    check.check(
        "dev_limited grpc query after delete -> config not exist",
        meta == "ConfigQueryResponse" and "config data not exist" in (body.get("message") or ""),
        f"got {meta}: {body}",
    )


# ---------------------------------------------------------------------------
# 用例8: gRPC 注册中心数据权限
# ---------------------------------------------------------------------------


def case8_grpc_naming_privilege(check: Check, fx: Fixture):
    print("\n=== 用例8: gRPC 注册中心数据权限 ===")
    service = fx.grpc_service_name

    def register(client, namespace):
        return fx.grpc[client].request(
            "InstanceRequest",
            {
                "namespace": namespace,
                "serviceName": service,
                "groupName": "DEFAULT_GROUP",
                "type": "registerInstance",
                "instance": {
                    "ip": "127.0.0.1",
                    "port": 19090,
                    "weight": 1.0,
                    "healthy": True,
                    "enabled": True,
                    "ephemeral": True,
                },
                "requestId": uuid.uuid4().hex,
            },
            token=fx.openapi[client].token,
        )

    def service_list(client, namespace):
        return fx.grpc[client].request(
            "ServiceListRequest",
            {
                "namespace": namespace,
                "groupName": "DEFAULT_GROUP",
                "pageNo": 1,
                "pageSize": 100,
                "requestId": uuid.uuid4().hex,
            },
            token=fx.openapi[client].token,
        )

    meta, body = register("dev_empty", "public")
    grpc_error_code403(check, "dev_empty grpc register public", (meta, body), "NO_NAMESPACE_PERMISSION")

    meta, body = register("dev_limited", "public")
    check.check(
        "dev_limited grpc register public -> success",
        meta == "InstanceResponse" and body.get("resultCode") == 200,
        f"got {meta}: {body}",
    )

    meta, body = register("dev_limited", "ns-x")
    grpc_error_code403(check, "dev_limited grpc register ns-x", (meta, body), "NO_NAMESPACE_PERMISSION")

    meta, body = service_list("dev_limited", "ns-x")
    grpc_error_code403(check, "dev_limited grpc service_list ns-x", (meta, body), "NO_NAMESPACE_PERMISSION")

    def grpc_service_list_contains(client, namespace, name, max_wait_seconds=5):
        """service list is eventually consistent, poll until the service shows up"""
        deadline = time.time() + max_wait_seconds
        last = ("", {})
        while time.time() < deadline:
            last = service_list(client, namespace)
            if last[0] == "ServiceListResponse" and name in (last[1].get("serviceNames") or []):
                return True, last
            time.sleep(0.3)
        return False, last

    meta, body = service_list("admin", "public")
    ok = check.check(
        "admin grpc service_list public -> success",
        meta == "ServiceListResponse" and body.get("resultCode") == 200,
        f"got {meta}: {body}",
    )
    if ok:
        found, (_, body) = grpc_service_list_contains("admin", "public", service)
        names = body.get("serviceNames") or []
        check.check(
            "admin grpc service_list public contains registered service",
            found,
            f"got {names}",
        )


# ---------------------------------------------------------------------------
# 用例10: 鉴权关闭兼容性回归
# ---------------------------------------------------------------------------


def case10_no_auth_regression(check: Check):
    print("\n=== 用例10: 鉴权关闭兼容性回归 (默认实例) ===")
    data_id = unique_id("openapi-noauth-test")
    service = unique_id("openapi-noauth-svc")

    client = OpenApiClient(NOAUTH_HTTP_BASE)

    resp = client.publish_config(data_id, tenant="ns-x", content="noauth")
    check.check("no token publish config ns-x -> 200 true", resp.status_code == 200 and resp.text == "true", f"got {resp.status_code}: {resp.text}")

    resp = client.query_config(data_id, tenant="ns-x")
    check.check("no token query config ns-x -> 200", resp.status_code == 200 and resp.text == "noauth", f"got {resp.status_code}: {resp.text[:80]}")

    resp = client.register_instance(service, namespace_id="ns-x")
    check.check("no token register instance ns-x -> ok", resp.status_code == 200 and resp.text == "ok", f"got {resp.status_code}: {resp.text}")

    resp = client.instance_list(service, namespace_id="ns-x")
    ok = check.check("no token instance_list ns-x -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        hosts = resp.json().get("hosts") or []
        check.check("no token instance_list contains instance", len(hosts) >= 1, f"got {hosts}")

    resp = client.list_namespaces_v1()
    ok = check.check("no token list_namespaces_v1 -> 200", resp.status_code == 200, f"got {resp.status_code}")
    if ok:
        ids = [item.get("namespace") for item in resp.json().get("data", [])]
        check.check(
            "no token namespace list is unfiltered (full privilege)",
            any(i in (None, "") for i in ids),
            f"got {ids}",
        )

    # cleanup
    cleanup = OpenApiClient(NOAUTH_HTTP_BASE)
    cleanup.delete_config(data_id, tenant="ns-x")


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main():
    check = Check()
    fx = Fixture(check)

    print("OpenAPI Privilege Integration Test")
    print(f"  auth instance : http={AUTH_HTTP_BASE} grpc={AUTH_GRPC_ADDR} console={AUTH_CONSOLE_BASE}")
    print(f"  no-auth instance: http={NOAUTH_HTTP_BASE}")

    try:
        fx.setup()
    except Exception as e:  # noqa: BLE001
        print(f"❌ setup failed: {e}")
        fx.teardown()
        return 1

    try:
        case4_http_config_privilege(check, fx)
        case5_http_naming_privilege(check, fx)
        case6_http_namespace_privilege(check, fx)
        case7_grpc_config_privilege(check, fx)
        case8_grpc_naming_privilege(check, fx)
        case9_http_listener_filter(check, fx)
    finally:
        fx.teardown()

    # 用例10 uses the no-auth instance (independent fixture)
    try:
        case10_no_auth_regression(check)
    except requests.RequestException as e:
        check.check("case10: no-auth instance reachable", False, str(e))

    print("\n=== Summary ===")
    print(f"passed: {check.passed}, failed: {check.failed}")
    if check.failures:
        print("failed checks:")
        for failure in check.failures:
            print(f"  - {failure}")
        return 1
    print("✅ all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
