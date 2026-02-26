"""TUI dashboard for the KV Store module."""

from __future__ import annotations

from typing import Any

from starkbot_sdk.tui import StarkbotDashboard

from rich.console import Group, RenderableType
from rich.panel import Panel
from rich.table import Table
from rich.text import Text


class KVStoreDashboard(StarkbotDashboard):

    def _get_sorted_entries(self) -> list[dict]:
        """Fetch and sort KV entries."""
        try:
            kv_resp = self.api("/rpc/kv", {"action": "list"})
            return sorted(
                kv_resp.get("data", {}).get("entries", []),
                key=lambda e: e["key"],
            )
        except Exception:
            return []

    def _get_entry_count(self) -> int:
        return len(self._get_sorted_entries())

    def build(self, width: int, state: dict | None = None) -> RenderableType:
        entries = self._get_sorted_entries()
        selected = state.get("selected", -1) if state else -1
        scroll = state.get("scroll", 0) if state else 0

        # Clamp selected
        if entries and selected >= len(entries):
            selected = len(entries) - 1

        try:
            status_resp = self.api("/rpc/status")
            uptime = status_resp.get("data", {}).get("uptime_seconds", 0)
        except Exception:
            uptime = 0

        # Format uptime
        mins, secs = divmod(int(uptime), 60)
        hours, mins = divmod(mins, 60)
        if hours:
            uptime_str = f"{hours}h {mins}m {secs}s"
        elif mins:
            uptime_str = f"{mins}m {secs}s"
        else:
            uptime_str = f"{secs}s"

        # Header
        header_text = Text()
        header_text.append("KV Store", style="bold cyan")
        header_text.append("  |  ", style="dim")
        header_text.append(f"{len(entries)}", style="bold green")
        header_text.append(" entries", style="green")
        header_text.append("  |  ", style="dim")
        header_text.append("uptime ", style="dim")
        header_text.append(uptime_str, style="yellow")

        header = Panel(header_text, border_style="bright_blue", padding=(0, 1))

        # Visible window — show up to (height - overhead) rows
        max_visible = max(1, 20)
        visible_entries = entries[scroll : scroll + max_visible]

        # Table
        table = Table(
            show_header=True,
            header_style="bold bright_blue",
            border_style="bright_black",
            expand=True,
            pad_edge=True,
        )
        table.add_column("#", style="dim", width=4)
        table.add_column("Key", style="cyan", ratio=1)
        table.add_column("Value", style="white", ratio=2)

        if entries:
            for i, entry in enumerate(visible_entries):
                row_idx = scroll + i
                val = entry["value"]
                if len(val) > 80:
                    val = val[:77] + "..."
                idx_str = str(row_idx)
                key_str = entry["key"]
                val_str = val
                if row_idx == selected:
                    idx_str = f"[reverse] {idx_str} [/reverse]"
                    key_str = f"[reverse]{key_str}[/reverse]"
                    val_str = f"[reverse]{val_str}[/reverse]"
                table.add_row(idx_str, key_str, val_str)
        else:
            table.add_row("", "[dim]No entries[/dim]", "[dim]—[/dim]")

        # Scroll indicator
        if len(entries) > max_visible:
            scroll_text = Text(
                f"  Showing {scroll + 1}-{min(scroll + max_visible, len(entries))} of {len(entries)}",
                style="dim",
            )
        else:
            scroll_text = Text()

        # Footer with keybindings
        interactive = state is not None
        if interactive:
            footer = Text()
            footer.append("  ↑↓", style="bold white")
            footer.append(" navigate  ", style="dim")
            footer.append("d", style="bold red")
            footer.append(" delete  ", style="dim")
            footer.append("e", style="bold yellow")
            footer.append(" edit  ", style="dim")
            footer.append("a", style="bold green")
            footer.append(" add  ", style="dim")
            footer.append("r", style="bold cyan")
            footer.append(" refresh  ", style="dim")
            footer.append("q", style="bold white")
            footer.append(" quit", style="dim")
        else:
            footer = Text("  q: quit  |  Ctrl+C: exit", style="dim")

        return Group(header, table, scroll_text, footer)

    def actions(self) -> dict[str, Any]:
        return {
            "navigable": True,
            "actions": [
                {
                    "key": "d",
                    "label": "Delete",
                    "action": "delete_selected",
                    "confirm": True,
                },
                {
                    "key": "e",
                    "label": "Edit value",
                    "action": "edit_selected",
                    "prompts": ["New value:"],
                },
                {
                    "key": "a",
                    "label": "Add entry",
                    "action": "add_entry",
                    "prompts": ["Key:", "Value:"],
                },
                {
                    "key": "r",
                    "label": "Refresh",
                    "action": "refresh",
                },
            ],
        }

    def handle_action(
        self, action: str, state: dict, inputs: list[str] | None = None
    ) -> dict[str, Any]:
        entries = self._get_sorted_entries()
        selected = state.get("selected", 0)

        if action == "refresh":
            return {"ok": True}

        if action == "delete_selected":
            if not entries or selected < 0 or selected >= len(entries):
                return {"ok": False, "error": "No entry selected"}
            key = entries[selected]["key"]
            self.api("/rpc/kv", {"action": "delete", "key": key})
            return {"ok": True, "message": f"Deleted {key}"}

        if action == "edit_selected":
            if not entries or selected < 0 or selected >= len(entries):
                return {"ok": False, "error": "No entry selected"}
            if not inputs or len(inputs) < 1:
                return {"ok": False, "error": "New value required"}
            key = entries[selected]["key"]
            self.api("/rpc/kv", {"action": "set", "key": key, "value": inputs[0]})
            return {"ok": True, "message": f"Updated {key}"}

        if action == "add_entry":
            if not inputs or len(inputs) < 2:
                return {"ok": False, "error": "Key and value required"}
            self.api("/rpc/kv", {"action": "set", "key": inputs[0], "value": inputs[1]})
            return {"ok": True, "message": f"Added {inputs[0]}"}

        return {"ok": False, "error": f"Unknown action: {action}"}
