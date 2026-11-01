"""Wait for this workload Pod's network policies before measuring requests."""

from collections import Counter
import json
import socket
import sys
import time
import urllib.error
import urllib.request


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, new_url):
        return None


class NetworkReadinessTimeout(RuntimeError):
    def __init__(self, failures):
        super().__init__("workload network readiness timed out")
        self.failures = dict(failures)


def wait_for_network(gateways, storage_address, timeout_seconds=60):
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    deadline = time.monotonic() + timeout_seconds
    failures = Counter()
    while time.monotonic() < deadline:
        ready = True
        for index, target in enumerate(gateways, start=1):
            label = f"gateway-{index}"
            try:
                with opener.open(target, timeout=2) as response:
                    if response.status != 204:
                        failures[f"{label}:http-{response.status}"] += 1
                        ready = False
            except urllib.error.HTTPError as error:
                failures[f"{label}:http-{error.code}"] += 1
                error.close()
                ready = False
            except (urllib.error.URLError, TimeoutError, OSError) as error:
                reason = type(getattr(error, "reason", error)).__name__
                failures[f"{label}:{reason}"] += 1
                ready = False
        try:
            with socket.create_connection(storage_address, timeout=2):
                pass
        except OSError as error:
            failures[f"storage:{type(error).__name__}"] += 1
            ready = False
        if ready:
            print("K3S_NETWORK_WAIT_RESULT " + json.dumps({"failures": dict(failures)}), flush=True)
            return
        time.sleep(0.2)
    # Error text can contain URLs. Only fixed labels, HTTP statuses, and types leave here.
    raise NetworkReadinessTimeout(failures)


if __name__ == "__main__":
    try:
        wait_for_network(sys.argv[1:], ("storage-public", 8081))
    except NetworkReadinessTimeout as error:
        print("K3S_NETWORK_WAIT_RESULT " + json.dumps({"failures": error.failures}), flush=True)
        sys.exit(1)
