"""Verify two real APISIX gateways against one disposable Valkey counter store."""

import http.client
import json
import os
import pathlib
import sys
import time
import urllib.parse
import uuid

GATEWAYS = sys.argv[2:]
LIMIT = 8
BLOCKED_IP = "203.0.113.101"
ORIGIN = "http://localhost"


def request(index, ip, path="/api/v1/account", headers=(), method="GET"):
    target = urllib.parse.urlsplit(GATEWAYS[index % len(GATEWAYS)])
    connection = http.client.HTTPConnection(target.hostname, target.port, timeout=5)
    connection.putrequest(method, path)
    connection.putheader("CF-Connecting-IP", ip)
    for name, value in headers:
        connection.putheader(name, value)
    connection.endheaders()
    response = connection.getresponse()
    body = response.read()
    status = response.status
    response_headers = {name.lower(): value for name, value in response.getheaders()}
    connection.close()
    return status, response_headers, json.loads(body) if body else None


def edge_error(result, status=429):
    code, headers, body = result
    assert code == status, (code, body)
    assert body["error"]["code"] == ("edge_rate_limited" if status == 429 else "invalid_input"), body
    assert str(uuid.UUID(body["request_id"])) == headers["x-request-id"]
    assert headers["cache-control"] == "no-store"
    assert "x-ratelimit-limit" not in headers
    assert "x-ratelimit-remaining" not in headers
    if status == 429:
        assert int(headers["retry-after"]) > 0
    return headers


def consume(ip, variants=None, extra=()):
    variants = variants or [ip]
    for index in range(LIMIT):
        status, _, body = request(index, variants[index % len(variants)], headers=extra)
        assert status == 401, (status, body)
    edge_error(request(1, ip))


def check():
    for index in range(LIMIT):
        status, _, body = request(index, BLOCKED_IP, headers=[
            ("X-RL-Client-IP", f"192.0.2.{index}"),
            ("Hirune-Edge-Client-IP", f"192.0.2.{index}"),
        ])
        assert status == 401, (status, body)
    headers = edge_error(request(1, BLOCKED_IP, headers=[("Origin", ORIGIN), ("X-Request-ID", "caller-chosen")]))
    assert headers["access-control-allow-origin"] == ORIGIN
    assert headers["access-control-allow-credentials"] == "true"
    assert "retry-after" in headers["access-control-expose-headers"].lower()
    assert "access-control-allow-origin" not in edge_error(request(0, BLOCKED_IP, headers=[("Origin", "https://untrusted.example")]))
    assert request(1, "203.0.113.102")[0] == 401
    consume("203.0.113.103", ["203.0.113.103", "::ffff:203.0.113.103"])
    consume("2001:db8::42", ["2001:db8::42", "2001:0DB8:0:0:0:0:0:42"])
    consume("203.0.113.104", extra=[("X-Real-Client-IP", "invalid"), ("X-Internal-Secret", "incorrect")])
    for index in range(LIMIT):
        assert request(index, f"198.51.100.{index + 1}", headers=[
            ("X-Internal-Secret", os.environ["INTERNAL_PROXY_SECRET"]),
            ("X-Real-Client-IP", "203.0.113.105"),
        ])[0] == 401
    edge_error(request(1, "198.51.100.99", headers=[
        ("X-Internal-Secret", os.environ["INTERNAL_PROXY_SECRET"]),
        ("X-Real-Client-IP", "203.0.113.105"),
    ]))
    for value in ["invalid", "", "192.0.2.1, 192.0.2.2", "010.0.0.1", "fe80::1%eth0"]:
        edge_error(request(0, value), 400)
    edge_error(request(1, "203.0.113.110", headers=[("CF-Connecting-IP", "203.0.113.111")]), 400)
    edge_error(request(0, "203.0.113.112", headers=[
        ("X-Internal-Secret", os.environ["INTERNAL_PROXY_SECRET"]),
        ("X-Real-Client-IP", "192.0.2.1"), ("X-Real-Client-IP", "192.0.2.2"),
    ]), 400)
    assert request(1, "203.0.113.113", headers=[
        ("X-Internal-Secret", os.environ["INTERNAL_PROXY_SECRET"]),
        ("X-Internal-Secret", os.environ["INTERNAL_PROXY_SECRET"]),
        ("X-Real-Client-IP", "invalid"),
    ])[0] == 401
    for path in ["/health/live", "/health/ready"]:
        for method in ["GET", "HEAD"]:
            assert request(0, BLOCKED_IP, path=path, method=method)[0] == 204
    edge_error(request(0, BLOCKED_IP, path="/health/ready", method="POST"))
    edge_error(request(1, BLOCKED_IP, path="/health/ready/extra"))
    print("Two gateways share one IP allowance; spoofed headers, equivalent IP spellings, SSR trust, duplicate rejection, CORS, typed errors, and health isolation passed.")


def after_gateway_restart():
    for index in range(2):
        edge_error(request(index, BLOCKED_IP))
    print("Gateway replacement preserved the shared edge allowance.")


def outage():
    state = json.loads(pathlib.Path("/tmp/account.json").read_text())
    for index in range(2):
        assert request(index, BLOCKED_IP)[0] == 401
        status, headers, body = request(index, "203.0.113.180", path="/api/v1/external/organization", headers=[
            ("Authorization", "Bearer " + state["key"]), ("Origin", ORIGIN),
        ])
        assert status == 429 and body["error"]["code"] == "api_quota_exceeded", (status, body)
        assert str(uuid.UUID(body["request_id"])) == headers["x-request-id"]
        assert int(headers["retry-after"]) > 0
        assert "retry-after" in headers["access-control-expose-headers"].lower()
        assert request(index, BLOCKED_IP, path="/health/ready")[0] == 204
    time.sleep(5.2)
    after_gateway_restart()
    print("Valkey timeout relaxed only edge enforcement; PostgreSQL quota stayed enforced and shared edge enforcement recovered.")


def after_valkey_restart():
    for index in range(2):
        assert request(index, BLOCKED_IP)[0] == 401
    # Old pooled sockets can fail once before gateways reconnect to the new Pod.
    for index in range(64):
        result = request(index, "203.0.113.190")
        if result[0] == 429:
            edge_error(result)
            break
        assert result[0] == 401, result
    else:
        raise AssertionError("edge enforcement did not resume after Valkey replacement")
    print("Replacing disposable Valkey intentionally reset the allowance; shared enforcement resumed.")


{
    "check": check,
    "after-gateway-restart": after_gateway_restart,
    "outage": outage,
    "after-valkey-restart": after_valkey_restart,
}[sys.argv[1]]()
