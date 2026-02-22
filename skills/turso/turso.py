#!/usr/bin/env python3
"""Turso/libSQL database skill script.

CLI convention: python3 turso.py <action> '<json_args>'

Requires env vars:
  TURSO_DATABASE_URL  — Turso database HTTP URL (e.g. https://mydb-myorg.turso.io)
  TURSO_GROUP_TOKEN   — Turso group auth token
"""

import json
import os
import sys
import urllib.request
import urllib.error


def get_config():
    """Get and validate Turso configuration from env vars."""
    url = os.environ.get("TURSO_DATABASE_URL", "").rstrip("/")
    token = os.environ.get("TURSO_GROUP_TOKEN", "")
    if not url or not token:
        print(
            json.dumps({
                "error": "TURSO_DATABASE_URL and TURSO_GROUP_TOKEN are required. "
                "Set them via Settings > API Keys."
            })
        )
        sys.exit(1)
    return url, token


def pipeline_request(url, token, requests):
    """Send a request to the Turso HTTP pipeline API.

    Args:
        url: Database base URL
        token: Auth token
        requests: List of pipeline request objects
    """
    endpoint = f"{url}/v2/pipeline"
    payload = json.dumps({"requests": requests}).encode()
    req = urllib.request.Request(
        endpoint,
        data=payload,
        headers={
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        body = e.read().decode() if e.fp else ""
        return {"error": f"HTTP {e.code}: {body}"}
    except urllib.error.URLError as e:
        return {"error": f"Connection error: {e.reason}"}


def format_query_result(result):
    """Format a pipeline query result into a clean response."""
    if "error" in result:
        return result

    results = result.get("results", [])
    if not results:
        return {"error": "No results returned from pipeline"}

    first = results[0]
    if "error" in first:
        return {"error": first["error"]}

    resp = first.get("response", {})
    result_type = resp.get("type")

    if result_type == "execute":
        exec_result = resp.get("result", {})
        cols = exec_result.get("cols", [])
        rows_raw = exec_result.get("rows", [])
        affected = exec_result.get("affected_row_count", 0)

        columns = [c.get("name", "") for c in cols]

        rows = []
        for row in rows_raw:
            row_dict = {}
            for i, col_name in enumerate(columns):
                cell = row[i] if i < len(row) else None
                if cell is None:
                    row_dict[col_name] = None
                elif isinstance(cell, dict):
                    row_dict[col_name] = cell.get("value")
                else:
                    row_dict[col_name] = cell
            rows.append(row_dict)

        if rows:
            return {"columns": columns, "rows": rows, "row_count": len(rows)}
        else:
            return {"affected_row_count": affected}

    return {"raw": resp}


# ---------------------------------------------------------------------------
# Actions
# ---------------------------------------------------------------------------


def list_tables(args):
    """List all tables and views in the database."""
    url, token = get_config()
    reqs = [
        {"type": "execute", "stmt": {
            "sql": "SELECT type, name, sql FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY type, name"
        }},
        {"type": "close"},
    ]
    result = pipeline_request(url, token, reqs)
    return format_query_result(result)


def describe_table(args):
    """Describe a table's columns using PRAGMA table_info."""
    table = args.get("table")
    if not table:
        return {"error": "table is required for describe_table"}

    # Validate table name to prevent injection
    if not all(c.isalnum() or c == "_" for c in table):
        return {"error": "Invalid table name"}

    url, token = get_config()
    reqs = [
        {"type": "execute", "stmt": {"sql": f"PRAGMA table_info({table})"}},
        {"type": "close"},
    ]
    result = pipeline_request(url, token, reqs)
    formatted = format_query_result(result)

    # Make PRAGMA output more readable
    if "rows" in formatted:
        columns = []
        for row in formatted["rows"]:
            col = {
                "cid": row.get("cid"),
                "name": row.get("name"),
                "type": row.get("type"),
                "notnull": row.get("notnull"),
                "default": row.get("dflt_value"),
                "primary_key": row.get("pk"),
            }
            columns.append(col)
        return {"table": table, "columns": columns}

    return formatted


def query(args):
    """Execute a read-only SQL query."""
    sql = args.get("sql")
    if not sql:
        return {"error": "sql is required for query"}

    url, token = get_config()
    reqs = [
        {"type": "execute", "stmt": {"sql": sql}},
        {"type": "close"},
    ]
    result = pipeline_request(url, token, reqs)
    return format_query_result(result)


def execute(args):
    """Execute a write SQL statement."""
    sql = args.get("sql")
    if not sql:
        return {"error": "sql is required for execute"}

    url, token = get_config()
    reqs = [
        {"type": "execute", "stmt": {"sql": sql}},
        {"type": "close"},
    ]
    result = pipeline_request(url, token, reqs)
    return format_query_result(result)


# ---------------------------------------------------------------------------
# Main dispatch
# ---------------------------------------------------------------------------

ACTIONS = {
    "list_tables": list_tables,
    "describe_table": describe_table,
    "query": query,
    "execute": execute,
}


def main():
    if len(sys.argv) < 2:
        print(
            json.dumps({
                "error": "Usage: turso.py <action> [json_args]",
                "available_actions": list(ACTIONS.keys()),
            })
        )
        sys.exit(1)

    action = sys.argv[1]
    args = {}
    if len(sys.argv) >= 3:
        try:
            args = json.loads(sys.argv[2])
        except json.JSONDecodeError as e:
            print(json.dumps({"error": f"Invalid JSON args: {e}"}))
            sys.exit(1)

    if action not in ACTIONS:
        print(
            json.dumps({
                "error": f"Unknown action: {action}",
                "available_actions": list(ACTIONS.keys()),
            })
        )
        sys.exit(1)

    try:
        result = ACTIONS[action](args)
        if isinstance(result, (dict, list)):
            print(json.dumps(result, default=str))
        else:
            print(json.dumps({"result": str(result)}))
    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stdout)
        sys.exit(1)


if __name__ == "__main__":
    main()
