"""Exercise the real gateway and API inside the isolated k3s fixture."""

import concurrent.futures
from collections import Counter
import json
import os
import pathlib
import secrets
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

BASE = "http://hirune-apisix:9080"
STATE = pathlib.Path("/tmp/account.json")
STOP = pathlib.Path("/tmp/traffic-stop")
DONE = pathlib.Path("/tmp/traffic-result.json")
STARTED = pathlib.Path("/tmp/traffic-started")


def request(method, path, body=None, headers=None, expected=200):
    headers = {"CF-Connecting-IP": "203.0.113.8", **(headers or {})}
    if body is not None:
        headers["Content-Type"] = "application/json"
    call = urllib.request.Request(
        path if path.startswith("http") else BASE + path,
        data=json.dumps(body).encode() if body is not None else None,
        headers=headers, method=method,
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        response = opener.open(call, timeout=5)
    except urllib.error.HTTPError as error:
        response = error
    with response:
        assert response.status == expected, f"{method} {urllib.parse.urlsplit(path).path}: {response.status}, expected {expected}"
        data = response.read()
        return json.loads(data) if data else None, response.headers


def state():
    return json.loads(STATE.read_text())


def session_headers():
    return {"Authorization": "Bearer " + state()["session"]}


def verified_account(client_ip="203.0.113.8"):
    email = "k3s-" + secrets.token_hex(8) + "@example.test"
    password = "k3s acceptance " + secrets.token_urlsafe(24)
    credentials = {"email": email, "password": password, "transport": "bearer"}
    identity_headers = {"CF-Connecting-IP": client_ip}
    request("POST", "/api/v1/auth/signup", {"email": email, "password": password}, headers=identity_headers, expected=202)
    deadline = time.monotonic() + 90
    token = None
    while time.monotonic() < deadline and token is None:
        result, _ = request("GET", "http://mailpit:8025/api/v1/search?" +
                            urllib.parse.urlencode({"query": "to:" + email}))
        for message in result.get("messages", []):
            if "Verify your email" in message["Subject"]:
                detail, _ = request("GET", "http://mailpit:8025/api/v1/message/" + message["ID"])
                token = detail["Text"].split("#token=", 1)[1].split()[0]
        if token is None:
            time.sleep(0.2)
    assert token is not None, "worker did not deliver verification mail"
    request("POST", "/api/v1/auth/verification/confirm", {"token": token}, headers=identity_headers, expected=204)
    login, _ = request("POST", "/api/v1/auth/login", credentials, headers=identity_headers)
    assert login["status"] == "authenticated"
    session = {"Authorization": "Bearer " + login["token"]}
    account, _ = request("GET", "/api/v1/account", headers=session)
    assert account["verified"] is True
    return account, login, credentials


def prepare():
    account, login, credentials = verified_account()
    session = {"Authorization": "Bearer " + login["token"]}
    organization, _ = request("POST", "/api/v1/organizations", {"name": "k3s acceptance"}, headers=session, expected=201)
    client_path = f"/api/v1/organizations/{organization['id']}/api-clients"
    client, _ = request("POST", client_path, {"name": "k3s partner", "scopes": ["organizations:read"]}, headers=session, expected=201)
    issued, _ = request("POST", client_path + "/" + client["id"] + "/keys",
                        {"name": "k3s key", "expires_in_days": 1}, headers=session, expected=201)
    STATE.write_text(json.dumps({"session": login["token"], "client": client["id"],
                                 "key": issued["credential"], "credentials": credentials,
                                 "account_id": account["id"], "organization_id": organization["id"]}))
    STATE.chmod(0o600)
    # APISIX must preserve both the CF header and the secret-authenticated SSR override.
    request("POST", "/api/v1/auth/login", credentials,
            headers={"CF-Connecting-IP": "invalid"}, expected=400)
    request("POST", "/api/v1/auth/login", credentials, headers={
        "CF-Connecting-IP": "invalid", "X-Internal-Secret": os.environ["INTERNAL_PROXY_SECRET"],
        "X-Real-Client-IP": "203.0.113.9"})
    request("POST", "/api/v1/auth/login", credentials, headers={
        "CF-Connecting-IP": "invalid", "X-Internal-Secret": "incorrect",
        "X-Real-Client-IP": "203.0.113.9"}, expected=400)
    print("Signup, worker mail, bearer session, and CF/SSR header forwarding passed.")


def consume_two():
    for remaining in ["2", "1"]:
        _, headers = request("GET", "/api/v1/external/organization",
                             headers={"Authorization": "Bearer " + state()["key"]})
        assert headers["X-Quota-Remaining"] == remaining


def after_rollout():
    request("GET", "/api/v1/account", headers=session_headers())
    _, headers = request("GET", "/api/v1/external/organization",
                         headers={"Authorization": "Bearer " + state()["key"]})
    assert headers["X-Quota-Remaining"] == "0"
    error, _ = request("GET", "/api/v1/external/organization",
                       headers={"Authorization": "Bearer " + state()["key"]}, expected=429)
    assert error["error"]["code"] == "api_quota_exceeded"
    print("Existing session survived API replacement; the shared monthly budget remained exhausted.")


def retained_authority():
    request("GET", "/api/v1/account", headers=session_headers())
    error, _ = request("GET", "/api/v1/external/organization",
                       headers={"Authorization": "Bearer " + state()["key"]}, expected=429)
    assert error["error"]["code"] == "api_quota_exceeded"
    print("Coordinated drained deployment restored the session and retained the exhausted quota.")


def maintenance():
    body, headers = request("OPTIONS", "/api/v1/account", headers={
        "Origin": "http://localhost", "Access-Control-Request-Method": "POST",
        "Access-Control-Request-Headers": "content-type, authorization",
    }, expected=204)
    assert body is None
    assert headers["Access-Control-Allow-Origin"] == "http://localhost"
    assert headers["Access-Control-Allow-Credentials"] == "true"
    assert "POST" in headers["Access-Control-Allow-Methods"]
    assert "authorization" in headers["Access-Control-Allow-Headers"].lower()
    request("OPTIONS", "/api/v1/account", headers={
        "Origin": "http://localhost", "Access-Control-Request-Method": "POST",
        "Access-Control-Request-Headers": "x-untrusted",
    }, expected=503)
    for method in ["GET", "POST", "OPTIONS"]:
        body, headers = request(method, "/api/v1/account", headers={"Origin": "http://localhost"}, expected=503)
        assert body["error"]["code"] == "maintenance"
        assert str(uuid.UUID(body["request_id"])) == headers["X-Request-ID"]
        assert headers["Retry-After"] == "30"
        assert headers["Cache-Control"] == "no-store"
        assert headers["Access-Control-Allow-Origin"] == "http://localhost"
        assert headers["Access-Control-Allow-Credentials"] == "true"
        assert "retry-after" in headers["Access-Control-Expose-Headers"].lower()
    health, _ = request("GET", "/health/gateway")
    assert health["maintenance"] is True
    print("APISIX Service returned typed maintenance and allowed browser preflight with CORS and Retry-After while all API Pods were absent.")


def traffic():
    headers = session_headers()

    def consume():
        completed, errors = 0, 0
        reasons = Counter()
        while not STOP.exists():
            try:
                request("GET", "/api/v1/account", headers=headers)
                completed += 1
                STARTED.touch()
            except AssertionError as error:
                errors += 1
                # This fixed API call reports only its method, path, and status.
                reasons[str(error)] += 1
            except (urllib.error.URLError, TimeoutError, OSError) as error:
                errors += 1
                # Network messages can contain URLs. Record only the exception type.
                reasons[type(getattr(error, "reason", error)).__name__] += 1
            time.sleep(0.02)
        return completed, errors, reasons

    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        results = list(executor.map(lambda _: consume(), range(4)))
    reasons = Counter()
    for _, _, failures in results:
        reasons.update(failures)
    DONE.write_text(json.dumps({"requests": sum(x[0] for x in results), "errors": sum(x[1] for x in results),
                                "error_reasons": dict(reasons)}))


def start_traffic():
    for path in [STOP, DONE, STARTED]:
        path.unlink(missing_ok=True)
    with open("/tmp/traffic.log", "w") as output:
        subprocess.Popen([sys.executable, __file__, "traffic"], stdout=output, stderr=output, start_new_session=True)
    deadline = time.monotonic() + 15
    while not STARTED.exists():
        assert time.monotonic() < deadline, "traffic process did not start"
        time.sleep(0.1)


def stop_traffic():
    STOP.touch()
    deadline = time.monotonic() + 20
    while not DONE.exists():
        assert time.monotonic() < deadline, "traffic process did not finish"
        time.sleep(0.1)
    result = json.loads(DONE.read_text())
    assert result["errors"] == 0 and result["requests"] >= 10, result
    print(f"Continuous authenticated traffic: {result['requests']} successful requests, zero errors.")


def cannot_connect(host, port):
    addresses = socket.getaddrinfo(host, port, type=socket.SOCK_STREAM)
    assert addresses, "service name resolution returned no addresses"
    address = addresses[0][4]
    try:
        with socket.create_connection(address, timeout=3):
            pass
    except (TimeoutError, OSError):
        return
    raise AssertionError(f"private ingress unexpectedly allowed {host}:{port}")


def private_ingress():
    request("GET", "/health/ready", expected=204)
    cannot_connect("hirune-api", 3000)
    cannot_connect("hirune-edge-valkey", 6379)
    cannot_connect("seaweedfs", 8333)
    cannot_connect("meilisearch", 7700)
    print("Tunnel-labelled Pod can reach APISIX and cannot bypass it to the API or Valkey.")


def denied_ingress():
    cannot_connect("hirune-apisix", 9080)
    cannot_connect("hirune-api", 3000)
    cannot_connect("hirune-edge-valkey", 6379)
    cannot_connect("storage-public", 8081)
    print("Untrusted Pod cannot connect to APISIX, the API, or Valkey.")


if __name__ == "__main__":
    import files
    import workflows

    {
        "prepare": prepare, "client-id": lambda: print(state()["client"]),
        "consume-two": consume_two, "after-rollout": after_rollout,
        "retained-authority": retained_authority,
        "maintenance": maintenance,
        "files-prepare": files.prepare, "files-retained": files.retained, "files-delete": files.delete,
        "workflow-seed": workflows.seed, "workflow-accounts": workflows.accounts,
        "traffic": traffic, "start-traffic": start_traffic, "stop-traffic": stop_traffic,
        "private-ingress": private_ingress, "denied-ingress": denied_ingress,
    }[sys.argv[1]]()
