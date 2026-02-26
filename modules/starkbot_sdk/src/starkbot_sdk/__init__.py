"""StarkBot SDK — shared helpers for Python modules."""

from starkbot_sdk.responses import success, error, status_response
from starkbot_sdk.app import create_app

__all__ = [
    "success",
    "error",
    "status_response",
    "create_app",
]

# TUI dashboard support available via: from starkbot_sdk.tui import StarkbotDashboard
# Requires optional dependency group: pip install starkbot-sdk[tui]
