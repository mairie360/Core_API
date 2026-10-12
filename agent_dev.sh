#!/usr/bin/env bash
#
# Dev loop for an AI coding agent (or a human): runs the compose stack detached, keeps
# `docker compose watch` syncing the sources in the background, and gives short commands to see what
# the code does (logs, health, SQL, Redis, authenticated HTTP calls) without attaching to a terminal.
#
#   ./agent_dev.sh up               build + start the stack detached, wait for healthy, start the watcher
#   ./agent_dev.sh wait [seconds]   wait until the app answers its readiness URL again (after an edit)
#   ./agent_dev.sh status           containers, watcher and readiness at a glance
#   ./agent_dev.sh logs [svc] [N]   last N log lines of a service (default: the app, 80)
#   ./agent_dev.sh sql "<query>"    run SQL in the database container
#   ./agent_dev.sh redis <cmd...>   run a redis-cli command
#   ./agent_dev.sh token [sub] [role]   print an HS256 JWT signed with the stack secret (default: admin, sub=1)
#   ./agent_dev.sh http METHOD PATH [json-body]   call the app with that token, print status then body
#   ./agent_dev.sh restart [svc]    restart one service (default: the app)
#   ./agent_dev.sh down [-v]        stop the watcher and the stack (-v also drops the volumes)
#
# Stack-local overrides live in docker-compose.agent.yml (database and Redis published on 127.0.0.1).
# Exit codes are those of the underlying command, so the agent can branch on them.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

# --- per-repository settings ------------------------------------------------------------------------
APP_SERVICE="core"
APP_URL="http://localhost:3000"
READY_PATH="/ready"
NEEDS_NODE_AUTH_TOKEN="false"
# ------------------------------------------------------------------------------------------------------

DB_USER="${DB_USER:-postgres}"
DB_NAME="${DB_NAME:-mairie_360_database}"
# Well-known dev value of the compose files (JWT_ALLOW_WEAK_SECRET=true there). Override for another stack.
JWT_SECRET="${JWT_SECRET:-b\"secret\"}"
STATE_DIR=".agent"
WATCH_PID="$STATE_DIR/watch.pid"
WATCH_LOG="$STATE_DIR/watch.log"

compose() { docker compose -f docker-compose.yml -f docker-compose.agent.yml "$@"; }

die() { echo "agent_dev: $*" >&2; exit 1; }

watcher_running() { [ -f "$WATCH_PID" ] && kill -0 "$(cat "$WATCH_PID")" 2>/dev/null; }

probe() { curl -s -o /dev/null -w '%{http_code}' --max-time 3 "$APP_URL$READY_PATH" 2>/dev/null || true; }

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }

make_token() {
    local sub="${1:-1}" role="${2:-admin}" header payload signature
    header="$(printf '%s' '{"alg":"HS256","typ":"JWT"}' | b64url)"
    payload="$(printf '{"sub":"%s","role":"%s","exp":%s}' "$sub" "$role" "$(( $(date +%s) + 4 * 3600 ))" | b64url)"
    signature="$(printf '%s.%s' "$header" "$payload" | openssl dgst -sha256 -hmac "$JWT_SECRET" -binary | b64url)"
    printf '%s.%s.%s\n' "$header" "$payload" "$signature"
}

cmd_up() {
    if [ "$NEEDS_NODE_AUTH_TOKEN" = "true" ] && [ -z "${NODE_AUTH_TOKEN:-}" ]; then
        die "NODE_AUTH_TOKEN is not set (token with read:packages, needed to install @mairie360/* packages)"
    fi
    mkdir -p "$STATE_DIR"
    compose up -d --build --wait --wait-timeout "${WAIT_TIMEOUT:-600}" || {
        echo "agent_dev: the stack did not become healthy, last logs of $APP_SERVICE:" >&2
        compose logs --no-color --tail 60 "$APP_SERVICE" >&2 || true
        exit 1
    }
    if watcher_running; then
        echo "watcher already running (pid $(cat "$WATCH_PID"))"
    else
        # `watch --no-up` only syncs/rebuilds on file changes; the containers are already up.
        nohup docker compose -f docker-compose.yml -f docker-compose.agent.yml watch --no-up \
            >"$WATCH_LOG" 2>&1 &
        echo $! >"$WATCH_PID"
        echo "watcher started (pid $(cat "$WATCH_PID"), log $WATCH_LOG)"
    fi
    cmd_status
}

cmd_wait() {
    local timeout="${1:-300}" deadline code
    deadline=$(( $(date +%s) + timeout ))
    # Give the watcher a moment to sync the edit and the app to start rebuilding.
    sleep 2
    while [ "$(date +%s)" -lt "$deadline" ]; do
        code="$(probe)"
        if [ "$code" = "200" ]; then echo "ready ($APP_URL$READY_PATH -> 200)"; return 0; fi
        sleep 2
    done
    echo "not ready after ${timeout}s (last status: ${code:-none}), last logs of $APP_SERVICE:" >&2
    compose logs --no-color --tail 40 "$APP_SERVICE" >&2 || true
    return 1
}

cmd_status() {
    compose ps --format 'table {{.Service}}\t{{.State}}\t{{.Health}}\t{{.Ports}}'
    if watcher_running; then echo "watcher: running (pid $(cat "$WATCH_PID"))"; else echo "watcher: stopped"; fi
    echo "$APP_SERVICE readiness: HTTP $(probe)"
}

cmd_logs() {
    local svc="${1:-$APP_SERVICE}" lines="${2:-80}"
    compose logs --no-color --tail "$lines" "$svc"
}

cmd_sql() {
    [ $# -ge 1 ] || die 'usage: sql "<query>"'
    compose exec -T postgres psql -U "$DB_USER" -d "$DB_NAME" -v ON_ERROR_STOP=1 -P pager=off -c "$1"
}

cmd_redis() {
    [ $# -ge 1 ] || die "usage: redis <command...>"
    compose exec -T redis redis-cli "$@"
}

cmd_http() {
    [ $# -ge 2 ] || die "usage: http METHOD PATH [json-body]"
    local method="$1" path="$2" body="${3:-}" out status
    local args=(-s -X "$method" "$APP_URL$path" -H "Authorization: Bearer $(make_token)" -w '\n%{http_code}')
    [ -n "$body" ] && args+=(-H 'Content-Type: application/json' --data "$body")
    out="$(curl "${args[@]}")" || die "request failed (is the stack up? ./agent_dev.sh status)"
    status="${out##*$'\n'}"
    echo "HTTP $status"
    printf '%s\n' "${out%$'\n'*}"
}

cmd_restart() { compose restart "${1:-$APP_SERVICE}"; }

cmd_down() {
    if watcher_running; then
        kill "$(cat "$WATCH_PID")" 2>/dev/null || true
    fi
    rm -f "$WATCH_PID"
    if [ "${1:-}" = "-v" ]; then compose down -v; else compose down; fi
}

action="${1:-}"
shift || true
case "$action" in
    up) cmd_up ;;
    wait) cmd_wait "$@" ;;
    status) cmd_status ;;
    logs) cmd_logs "$@" ;;
    sql) cmd_sql "$@" ;;
    redis) cmd_redis "$@" ;;
    token) make_token "$@" ;;
    http) cmd_http "$@" ;;
    restart) cmd_restart "$@" ;;
    down) cmd_down "$@" ;;
    *) sed -n '3,20p' "$0" | sed 's/^# \{0,1\}//'; [ -z "$action" ] || exit 2 ;;
esac
