"""Create load users through the deployed API and worker mail flow."""

import json
import os
import pathlib

from probe import request, verified_account

ACCOUNTS = pathlib.Path("/tmp/workflow-accounts.json")


def seed():
    count = 20 if os.environ.get("K6_PROFILE") == "extended" else 2
    users = []
    for index in range(count):
        client_ip = f"198.51.100.{30 + index}"
        owner, session, credentials = verified_account(client_ip)
        commenter, commenter_session, _ = verified_account(f"198.51.100.{130 + index}")
        headers = {"Authorization": "Bearer " + session["token"], "CF-Connecting-IP": client_ip}
        organizations = [request("POST", "/api/v1/organizations", {"name": f"Workflow {index} {scope}"},
                                 headers=headers, expected=201)[0] for scope in ["primary", "secondary"]]
        organization_path = "/api/v1/organizations/" + organizations[0]["id"]
        request("PUT", organization_path + "/members/" + commenter["id"], headers=headers, expected=204)
        client_path = organization_path + "/api-clients"
        client, _ = request("POST", client_path, {"name": "Workflow partner", "scopes": ["organizations:read", "posts:read"]},
                            headers=headers, expected=201)
        keys = [request("POST", client_path + "/" + client["id"] + "/keys", {"name": name, "expires_in_days": 1},
                        headers=headers, expected=201)[0]["credential"] for name in ["Primary", "Rotation"]]
        revocable, _ = request("POST", "/api/v1/auth/login", credentials, headers={"CF-Connecting-IP": client_ip})
        users.append({"account_id": owner["id"], "token": session["token"],
                      "commenter_token": commenter_session["token"], "revocable_token": revocable["token"],
                      "revocable_session_id": revocable["session_id"],
                      "organization_ids": [organization["id"] for organization in organizations],
                      "api_client_id": client["id"], "api_keys": keys})
    ACCOUNTS.write_text(json.dumps(users))
    ACCOUNTS.chmod(0o600)
    print(f"Created {count} isolated workflow users through signup, mail verification, and login.")


def accounts():
    # The Rust fixture captures this output in memory and passes it through a Secret.
    print(ACCOUNTS.read_text())
