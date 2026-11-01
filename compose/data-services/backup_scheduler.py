"""Run restart-safe daily backups from successful pgBackRest repository history."""

import datetime as dt
import json
import os
import subprocess
import time

UTC = dt.timezone.utc


def due_backup(now, backups, hour):
    daily = now.replace(hour=hour, minute=0, second=0, microsecond=0)
    if daily > now:
        daily -= dt.timedelta(days=1)
    weekly = daily - dt.timedelta(days=(daily.weekday() + 1) % 7)
    full = max((backup["timestamp"]["stop"] for backup in backups if backup["type"] == "full"), default=0)
    if full < weekly.timestamp():
        return "full"
    latest = max((backup["timestamp"]["stop"] for backup in backups), default=0)
    return None if latest >= daily.timestamp() else "diff"


def run():
    hour = int(os.environ.get("BACKUP_HOUR_UTC", "3"))
    if not 0 <= hour <= 23:
        raise ValueError("BACKUP_HOUR_UTC must be between 0 and 23")
    while True:
        try:
            result = subprocess.run(
                ["pgbackrest", "--stanza=db", "--output=json", "info"],
                check=True, capture_output=True, text=True,
            )
            stanzas = json.loads(result.stdout)
            if len(stanzas) != 1 or stanzas[0]["name"] != "db":
                raise ValueError("Expected the initialized db stanza")
            now = dt.datetime.now(UTC)
            kind = due_backup(now, stanzas[0].get("backup", []), hour)
            if kind:
                subprocess.run(["pgbackrest", "--stanza=db", "--type=" + kind, "backup"], check=True)
                continue
            next_backup = now.replace(hour=hour, minute=0, second=0, microsecond=0)
            if next_backup <= now:
                next_backup += dt.timedelta(days=1)
            # Re-read history at the next slot; signals can stop the container normally.
            time.sleep(max(1, (next_backup - now).total_seconds()))
        except (subprocess.CalledProcessError, ValueError, KeyError):
            print("Backup scheduling failed; retrying in 60 seconds. Check repository and archive health.", flush=True)
            time.sleep(60)


if __name__ == "__main__":
    run()
