#!/bin/sh
# Start Redis in the background (daemonized) before the backend binary.
# Redis is used as a fast key/value store for agent state tracking.
redis-server --daemonize yes --save "" --appendonly no --loglevel warning

exec /app/stark-backend-bin "$@"
