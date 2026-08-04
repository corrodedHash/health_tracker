#!/usr/bin/env python3
"""Import exercise sessions from a CSV (and optional GPX files) into health-tracker.

Two auth modes (both require --token):
  --token TOKEN   import with an existing API token directly (create one in the
                  web UI under "API tokens", or pass the bot's service token).
  --token TOKEN --link
                  replicate the bot's account-link flow: mint a link with the
                  given token, the user confirms it in a browser, and the script
                  polls for a freshly issued per-user token to import with.

Dates are preserved from the CSV: every row's `started_at` is sent verbatim
(parsed, then converted to UTC), so nothing is stored as "today".

CSV columns (header names are case-insensitive, aliases accepted):

  date         Required. "2026-01-15", "2026-01-15 08:30", RFC3339, ...
               Naive values are interpreted in the local timezone; date-only
               values use --default-time (default 12:00).
  kind         Required. weight | core | running | custom
  duration_min Optional. Minutes (float). Required for weight/core/custom and
               for running rows without a GPX file (ignored when a GPX file is
               given; the server computes it from the file).
  notes        Optional. Free text.
  quality      Optional. 1..5.
  weight_kg    Required for kind=weight (converted to grams server-side).
  sets         Required for kind=weight.
  distance_km  Optional for kind=running without a GPX file (converted to meters).
  gpx          Optional for kind=running. Path to a GPX file, resolved against
               --gpx-dir or the CSV's directory. When present the file is
               uploaded via POST /api/runs/gpx; date/duration/distance are taken
               from the file itself, so those CSV columns are ignored.

Example:
  csv_import.py --api-base http://localhost:3000 --token $TOKEN workouts.csv
  csv_import.py --api-base http://localhost:3000 --link workouts.csv
"""

import argparse
import csv
import json
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

KINDS = {"weight", "core", "running", "custom"}

HEADER_ALIASES = {
    "date": ("date", "started_at", "start", "day"),
    "kind": ("kind", "type", "exercise"),
    "duration_min": ("duration_min", "duration", "minutes"),
    "notes": ("notes", "note"),
    "quality": ("quality",),
    "weight_kg": ("weight_kg", "weight", "load_kg"),
    "sets": ("sets", "set"),
    "distance_km": ("distance_km", "distance", "km"),
    "gpx": ("gpx", "gpx_file", "file"),
}


def column_map(headers):
    """Map normalized CSV headers to canonical column names."""
    mapping = {}
    for header in headers:
        key = header.strip().lower()
        for canonical, aliases in HEADER_ALIASES.items():
            if key in aliases:
                mapping[canonical] = header
                break
    return mapping


def http_request(base, method, path, token=None, body=None, raw=None, content_type=None):
    url = base.rstrip("/") + path
    headers = {}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    if raw is not None:
        data = raw
    elif body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    else:
        data = None
    if content_type and content_type not in headers.values():
        headers["Content-Type"] = content_type

    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, _decode_json(resp.read())
    except urllib.error.HTTPError as err:
        return err.code, _decode_json(err.read())


def _decode_json(data: bytes) -> dict[str, Any]:
    if not data:
        return {}
    try:
        return json.loads(data)
    except json.JSONDecodeError:
        return {"error": data.decode(errors="replace")}


def run_link_flow(base, mint_token, poll_seconds, timeout):
    status, link = http_request(base, "POST", "/api/links", token=mint_token)
    if status != 200:
        die(f"failed to create link ({status}): {link}")
    code, url = link["code"], link["url"]
    print(f"Open this URL in a browser and confirm the link:\n  {url}\n", file=sys.stderr)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        status, resp = http_request(base, "GET", f"/api/links/{code}", token=mint_token)
        if status != 200:
            die(f"failed to poll link ({status}): {resp}")
        state = resp.get("status")
        if state == "accepted" and resp.get("token"):
            print("Link accepted; using freshly issued token.", file=sys.stderr)
            return resp["token"]
        if state == "expired":
            die("link expired before confirmation")
        time.sleep(poll_seconds)
    die("timed out waiting for link confirmation")


def parse_date(raw, default_time):
    txt = (raw or "").strip()
    if not txt:
        raise ValueError("empty date")
    if txt.endswith("Z"):
        txt = txt[:-1] + "+00:00"
    date_only = ":" not in txt

    parsed = None
    for fmt in ("%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S",
                "%Y-%m-%d %H:%M", "%Y-%m-%d"):
        try:
            parsed = datetime.strptime(txt, fmt)
            break
        except ValueError:
            continue
    if parsed is None:
        try:
            parsed = datetime.fromisoformat(txt)
        except ValueError as exc:
            raise ValueError(f"unrecognized date format: {raw!r}") from exc

    if parsed.tzinfo is None:
        if date_only:
            hours, minutes = (int(x) for x in default_time.split(":"))
            parsed = parsed.replace(hour=hours, minute=minutes, second=0, microsecond=0)
        parsed = parsed.astimezone()
    return parsed.astimezone(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def opt(value):
    """Return None for empty cells."""
    return value.strip() if value is not None and value.strip() else None


def parse_float(raw, field, row_number):
    if raw is None:
        raise ValueError(f"row {row_number}: missing {field}")
    try:
        return float(raw)
    except ValueError as exc:
        raise ValueError(f"row {row_number}: {field} is not a number: {raw!r}") from exc


def parse_int(raw, field, row_number):
    if raw is None:
        raise ValueError(f"row {row_number}: missing {field}")
    try:
        return int(float(raw))
    except ValueError as exc:
        raise ValueError(f"row {row_number}: {field} is not a number: {raw!r}") from exc


def resolve_gpx_path(raw, csv_dir, gpx_dir):
    path = Path(raw)
    if not path.is_absolute():
        base = Path(gpx_dir) if gpx_dir else csv_dir
        path = base / path
    if not path.is_file():
        raise ValueError(f"gpx file not found: {path}")
    return path


def build_rows(reader, mapping, args, csv_dir):
    """Return (import_rows, gpx_tasks, skipped) where import_rows are JSON payloads
    and gpx_tasks are (row_number, gpx_path) uploads."""
    import_rows = []
    gpx_tasks = []
    skipped = []

    def cell(row, canonical):
        header = mapping.get(canonical)
        if header is None:
            return None
        return opt(row[header])

    for row_number, row in enumerate(reader, start=2):  # 1-indexed incl. header
        try:
            kind = (cell(row, "kind") or "").lower()
            if kind not in KINDS:
                raise ValueError(f"row {row_number}: unknown kind {kind!r} (expected one of {', '.join(sorted(KINDS))})")

            quality = cell(row, "quality")
            quality = parse_int(quality, "quality", row_number) if quality else None
            notes = cell(row, "notes")

            def started_at():
                try:
                    return parse_date(cell(row, "date"), args.default_time)
                except ValueError as exc:
                    raise ValueError(f"row {row_number}: {exc}") from exc

            if kind == "running":
                gpx_raw = cell(row, "gpx")
                if gpx_raw:
                    gpx_path = resolve_gpx_path(gpx_raw, csv_dir, args.gpx_dir)
                    gpx_tasks.append((row_number, gpx_path))
                    continue
                duration_min = parse_float(cell(row, "duration_min"), "duration_min", row_number)
                distance_km = parse_float(cell(row, "distance_km"), "distance_km", row_number)
                import_rows.append({
                    "kind": "running",
                    "started_at": started_at(),
                    "duration_secs": duration_min * 60,
                    "notes": notes,
                    "quality": quality,
                    "distance_m": int(round(distance_km * 1000)),
                })
            else:
                duration_min = parse_float(cell(row, "duration_min"), "duration_min", row_number)
                payload = {
                    "kind": kind,
                    "started_at": started_at(),
                    "duration_secs": duration_min * 60,
                    "notes": notes,
                    "quality": quality,
                }
                if kind == "weight":
                    weight_kg = parse_float(cell(row, "weight_kg"), "weight_kg", row_number)
                    sets = parse_int(cell(row, "sets"), "sets", row_number)
                    payload["weight_g"] = int(round(weight_kg * 1000))
                    payload["sets"] = sets
                import_rows.append(payload)
        except ValueError as exc:
            skipped.append((row_number, str(exc)))

    return import_rows, gpx_tasks, skipped


def upload_gpx(base, token, row_number, gpx_path):
    raw = gpx_path.read_bytes()
    status, resp = http_request(
        base, "POST", "/api/runs/gpx", token=token,
        raw=raw, content_type="application/gpx+xml",
    )
    if status != 200:
        return f"row {row_number}: GPX upload failed ({status}): {resp}"
    return f"row {row_number}: created running session {resp['id']} (started {resp['started_at']})"


def import_sessions(base, token, rows, batch_size):
    lines = []
    for start in range(0, len(rows), batch_size):
        batch = rows[start:start + batch_size]
        status, resp = http_request(base, "POST", "/api/import/sessions", token=token, body=batch)
        if status != 200:
            lines.append(f"batch failed ({status}): {resp}")
            continue
        # resp["results"][i] corresponds to batch[i] (0-based within the batch).
        for i, result in enumerate(resp.get("results", [])):
            csv_row = start + i + 2  # CSV line number
            if "error" in result:
                lines.append(f"row {csv_row}: {result['error']}")
            else:
                lines.append(f"row {csv_row}: created {batch[i]['kind']} session {result['id']} "
                             f"(started {result['started_at']}, created {result['created_at']})")
    return lines


def main():
    parser = argparse.ArgumentParser(
        prog="csv_import.py",
        description="Import workout CSV (and GPX runs) into health-tracker.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="See the module docstring above the CSV column reference.",
    )
    parser.add_argument("csv", help="path to the CSV file")
    parser.add_argument("--api-base", default="http://localhost:3000",
                        help="web API base URL (default: %(default)s)")
    parser.add_argument("--token",
                        help="existing API token to import with (also used to mint "
                             "the link when --link is set)")
    parser.add_argument("--link", action="store_true",
                        help="run the bot account-link flow to obtain a fresh per-user "
                             "token, then import with that")
    parser.add_argument("--link-poll-seconds", type=float, default=3.0,
                        help="poll interval while waiting for link confirmation")
    parser.add_argument("--link-timeout", type=float, default=300.0,
                        help="max seconds to wait for link confirmation")
    parser.add_argument("--gpx-dir", help="base dir for relative gpx paths (default: CSV dir)")
    parser.add_argument("--default-time", default="12:00",
                        help="time (HH:MM, local) applied to date-only rows (default: %(default)s)")
    parser.add_argument("--batch-size", type=int, default=100,
                        help="rows per /api/import/sessions request (default: %(default)s)")
    parser.add_argument("--delimiter", default=",",
                        help="CSV delimiter (default: ',')")
    parser.add_argument("--dry-run", action="store_true",
                        help="parse and validate rows without calling the API")
    args = parser.parse_args()

    if not args.dry_run and not args.token:
        parser.error("--token is required (unless --dry-run)")

    csv_path = Path(args.csv)
    if not csv_path.is_file():
        die(f"CSV file not found: {csv_path}")
    csv_dir = csv_path.parent

    with csv_path.open(newline="", encoding="utf-8-sig") as fh:
        reader = csv.DictReader(fh, delimiter=args.delimiter)
        if reader.fieldnames is None:
            die("CSV is empty")
        mapping = column_map(reader.fieldnames)
        import_rows, gpx_tasks, skipped = build_rows(reader, mapping, args, csv_dir)

    print(f"parsed {len(import_rows)} session row(s), {len(gpx_tasks)} gpx upload(s), "
          f"{len(skipped)} invalid row(s)")

    if skipped:
        print("skipped rows:")
        for row_number, error in skipped:
            print(f"  {error}")

    if args.dry_run:
        print("dry-run: no API calls made")
        for payload in import_rows:
            print(f"  would create {payload['kind']} at {payload['started_at']} "
                  f"(duration {payload['duration_secs']}s)")
        for row_number, gpx_path in gpx_tasks:
            print(f"  would upload GPX {gpx_path} (row {row_number})")
        return

    if args.link:
        token = run_link_flow(args.api_base, args.token, args.link_poll_seconds, args.link_timeout)
    else:
        token = args.token

    lines = []
    for row_number, gpx_path in gpx_tasks:
        lines.append(upload_gpx(args.api_base, token, row_number, gpx_path))
    lines.extend(import_sessions(args.api_base, token, import_rows, args.batch_size))

    for line in lines:
        print(line)

    failed = sum(1 for line in lines if "failed" in line or "error" in line)
    if failed:
        print(f"\n{failed} item(s) failed; {len(import_rows) + len(gpx_tasks) - failed} succeeded.",
              file=sys.stderr)
        sys.exit(1)


def die(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(1)


if __name__ == "__main__":
    main()
