"""Exercise presigned originals and attachment authority through deployed services."""

import json
import pathlib
import secrets
import time
import urllib.error
import urllib.parse
import urllib.request

from probe import request, session_headers, state, verified_account

ARTIFACT = pathlib.Path("/tmp/file-acceptance.json")
ORIGINAL = b"<svg onload='untrusted()'>unchanged original\x00</svg>"


def storage_request(ticket, body=None, extra_headers=None, expected=200):
    address = urllib.parse.urlsplit(ticket["url"])
    assert address.netloc == "storage-public:8081", "client URL did not use the public storage proxy"
    headers = {header["name"]: header["value"] for header in ticket["headers"]}
    assert "authorization" not in {name.lower() for name in headers}, "storage received API credentials"
    if body is not None:
        headers["Content-Length"] = str(len(body))
    headers.update(extra_headers or {})
    call = urllib.request.Request(ticket["url"], data=body, headers=headers, method=ticket["method"])
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    try:
        response = opener.open(call, timeout=10)
    except urllib.error.HTTPError as error:
        response = error
    except (urllib.error.URLError, OSError):
        raise AssertionError("direct storage request failed") from None
    with response:
        assert response.status == expected, f"storage {ticket['method']}: {response.status}, expected {expected}"
        return response.read(), response.headers


def scoped_path(organization_id, file_id=None):
    return f"/api/v1/organizations/{organization_id}/files" + ("/" + file_id if file_id else "")


def reserve(organization_id, payload, filename):
    reserved, _ = request("POST", scoped_path(organization_id),
                          {"filename": filename, "media_type": "image/svg+xml", "byte_size": len(payload)},
                          headers=session_headers(), expected=201)
    return reserved


def upload(organization_id, payload, filename):
    reserved = reserve(organization_id, payload, filename)
    path = scoped_path(organization_id, reserved["id"])
    ticket, _ = request("POST", path + "/upload-url", headers=session_headers())
    _, cors = storage_request({**ticket, "method": "OPTIONS", "headers": []}, extra_headers={
        "Origin": "http://localhost", "Access-Control-Request-Method": "PUT",
        "Access-Control-Request-Headers": "content-type,content-encoding,content-disposition,cache-control,if-none-match"})
    assert cors["Access-Control-Allow-Origin"] == "http://localhost"
    assert "if-none-match" in cors["Access-Control-Allow-Headers"].lower()
    storage_request(ticket, payload)
    storage_request(ticket, payload, expected=412)
    completed, _ = request("POST", path + "/complete-upload", headers=session_headers())
    retried, _ = request("POST", path + "/complete-upload", headers=session_headers())
    assert completed["state"] == "ready" and completed["content_hash"] is None
    assert completed["version"] == retried["version"], "completion retry changed the file"
    return completed


def download(path, payload):
    ticket, _ = request("POST", path + "/download-url", {"method": "GET"}, headers=session_headers())
    data, headers = storage_request(ticket)
    assert data == payload, "storage changed original bytes"
    assert headers["Content-Type"] == "application/octet-stream"
    assert headers["Content-Disposition"].startswith("attachment;")
    assert headers["Cache-Control"] == "no-store" and headers["X-Content-Type-Options"] == "nosniff"
    head, _ = request("POST", path + "/download-url", {"method": "HEAD"}, headers=session_headers())
    _, metadata = storage_request(head)
    assert int(metadata["Content-Length"]) == len(payload)
    if payload:
        ranged, _ = storage_request(ticket, extra_headers={"Range": "bytes=0-7"}, expected=206)
        assert ranged == payload[:8]
    address = urllib.parse.urlsplit(ticket["url"])
    unsigned = urllib.parse.urlunsplit(address._replace(query=""))
    storage_request({"url": unsigned, "method": "GET", "headers": []}, expected=403)
    return ticket


def prepare():
    organization_id = state()["organization_id"]
    marker = "kubeoriginal" + secrets.token_hex(6)
    original = upload(organization_id, ORIGINAL, marker + " 한글.svg")
    pending = reserve(organization_id, b"pending", "pending.bin")
    foreign, _ = request("POST", "/api/v1/organizations", {"name": "Foreign file scope"}, headers=session_headers(), expected=201)
    foreign_file = upload(foreign["id"], b"foreign", "foreign.svg")
    empty = upload(organization_id, b"", "empty.svg")
    download(scoped_path(organization_id, empty["id"]), b"")
    posts = f"/api/v1/organizations/{organization_id}/posts"
    post_body = {"title": marker, "body": "Retained attachment", "state": "published", "expected_version": 0}
    for invalid in [pending["id"], foreign_file["id"]]:
        request("POST", posts, {**post_body, "attachment_ids": [invalid]}, headers=session_headers(), expected=404)
    post, _ = request("POST", posts, {**post_body, "attachment_ids": [original["id"]]}, headers=session_headers(), expected=201)
    outsider, outsider_session, _ = verified_account("198.51.100.220")
    assert outsider["id"] != state()["account_id"]
    request("POST", scoped_path(organization_id, original["id"]) + "/download-url", {"method": "GET"},
            headers={"Authorization": "Bearer " + outsider_session["token"]}, expected=404)
    ARTIFACT.write_text(json.dumps({"organization_id": organization_id, "file_id": original["id"],
                                   "post_id": post["id"], "pending_id": pending["id"], "empty_id": empty["id"],
                                   "foreign_organization_id": foreign["id"], "foreign_file_id": foreign_file["id"], "marker": marker}))
    ARTIFACT.chmod(0o600)
    retained()
    deadline = time.monotonic() + 10
    while True:
        results, _ = request("GET", scoped_path(organization_id) + "/search?" + urllib.parse.urlencode({"q": marker}), headers=session_headers())
        if any(file["id"] == original["id"] for file in results["items"]):
            break
        assert time.monotonic() < deadline, "worker did not publish the filename projection"
        time.sleep(0.2)
    print("Presigned public-proxy PUT/GET/HEAD, zero bytes, replay, CORS, tenant access, attachments, and worker filename search passed.")


def retained():
    artifact = json.loads(ARTIFACT.read_text())
    path = scoped_path(artifact["organization_id"], artifact["file_id"])
    metadata, _ = request("GET", path, headers=session_headers())
    assert metadata["state"] == "ready"
    post, _ = request("GET", f"/api/v1/organizations/{artifact['organization_id']}/posts/{artifact['post_id']}", headers=session_headers())
    assert post["attachment_ids"] == [artifact["file_id"]]
    download(path, ORIGINAL)
    print("Retained post and original remain readable through the gateway and public storage proxy.")


def delete():
    artifact = json.loads(ARTIFACT.read_text())
    path = scoped_path(artifact["organization_id"], artifact["file_id"])
    ticket, _ = request("POST", path + "/download-url", {"method": "GET"}, headers=session_headers())
    for organization_id, file_id in [(artifact["organization_id"], artifact[name]) for name in ["file_id", "pending_id", "empty_id"]] + [(artifact["foreign_organization_id"], artifact["foreign_file_id"])]:
        request("DELETE", scoped_path(organization_id, file_id), headers=session_headers(), expected=204)
    request("POST", path + "/download-url", {"method": "GET"}, headers=session_headers(), expected=404)
    post, _ = request("GET", f"/api/v1/organizations/{artifact['organization_id']}/posts/{artifact['post_id']}", headers=session_headers())
    assert post["attachment_ids"] == [], "deleted file remained attached to its parent"
    # A still-valid read URL must return an actual missing object, not an expiry rejection.
    deadline = time.monotonic() + 15
    while True:
        try:
            storage_request(ticket, expected=404)
            break
        except AssertionError:
            assert time.monotonic() < deadline, "worker did not remove the deleted original"
            time.sleep(0.2)
    print("Deletion denied new URLs, hid parent attachment links, and removed storage bytes through the worker.")
