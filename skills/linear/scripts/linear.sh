#!/usr/bin/env bash
# Linear CLI for Starkbot — talks to Linear GraphQL API
# Usage: linear.sh <action> [json_args]
set -euo pipefail

ACTION="${1:-help}"
ARGS="${2:-{\}}"

API="https://api.linear.app/graphql"

if [[ -z "${LINEAR_API_KEY:-}" ]]; then
  echo "ERROR: LINEAR_API_KEY is not set. Get one at https://linear.app/settings/api"
  exit 1
fi

# ── helpers ──────────────────────────────────────────────────────────────────

gql() {
  local query="$1"
  local response
  response=$(curl -sS --fail-with-body -X POST "$API" \
    -H "Content-Type: application/json" \
    -H "Authorization: $LINEAR_API_KEY" \
    -d "$query" 2>&1) || {
    echo "ERROR: Linear API request failed"
    echo "$response"
    exit 1
  }

  # Check for GraphQL errors
  local errors
  errors=$(echo "$response" | jq -r '.errors // empty')
  if [[ -n "$errors" && "$errors" != "null" ]]; then
    echo "ERROR: GraphQL error"
    echo "$errors" | jq -r '.[0].message // .[0] // .'
    exit 1
  fi

  echo "$response"
}

arg() {
  echo "$ARGS" | jq -r ".${1} // empty"
}

escape_gql() {
  # Escape string for embedding in GraphQL query
  printf '%s' "$1" | jq -Rs '.'
}

# ── commands ─────────────────────────────────────────────────────────────────

cmd_teams() {
  local resp
  resp=$(gql '{"query":"{ teams { nodes { id key name } } }"}')
  echo "$resp" | jq -r '.data.teams.nodes[] | "\(.key)\t\(.name)\t\(.id)"' | column -t -s$'\t'
}

cmd_my_issues() {
  local resp
  resp=$(gql '{"query":"{ viewer { assignedIssues(orderBy: updatedAt, first: 50, filter: { state: { type: { nin: [\"completed\",\"canceled\"] } } }) { nodes { identifier title priority state { name } project { name } } } } }"}')
  echo "$resp" | jq -r '.data.viewer.assignedIssues.nodes[] | "\(.identifier)\t[\(.state.name)]\tP\(.priority)\t\(.title)\t\(.project.name // "-")"' | column -t -s$'\t'
}

cmd_my_todos() {
  local resp
  resp=$(gql '{"query":"{ viewer { assignedIssues(orderBy: updatedAt, first: 50, filter: { state: { name: { in: [\"Todo\",\"Backlog\"] } } }) { nodes { identifier title priority state { name } } } } }"}')
  echo "$resp" | jq -r '.data.viewer.assignedIssues.nodes[] | "\(.identifier)\t[\(.state.name)]\tP\(.priority)\t\(.title)"' | column -t -s$'\t'
}

cmd_urgent() {
  local resp
  resp=$(gql '{"query":"{ issues(orderBy: updatedAt, first: 50, filter: { priority: { in: [1,2] }, state: { type: { nin: [\"completed\",\"canceled\"] } } }) { nodes { identifier title priority assignee { name } state { name } team { key } } } }"}')
  echo "$resp" | jq -r '.data.issues.nodes[] | "\(.team.key)-\(.identifier | split("-")[1])\tP\(.priority)\t[\(.state.name)]\t\(.assignee.name // "unassigned")\t\(.title)"' | column -t -s$'\t'
}

cmd_team() {
  local team
  team=$(arg "team")
  if [[ -z "$team" ]]; then
    team="${LINEAR_DEFAULT_TEAM:-}"
    if [[ -z "$team" ]]; then
      echo "ERROR: team key required. Use {\"team\":\"TEAM_KEY\"} or set LINEAR_DEFAULT_TEAM"
      exit 1
    fi
  fi
  local escaped_team
  escaped_team=$(escape_gql "$team")
  local resp
  resp=$(gql "{\"query\":\"{ teams(filter: { key: { eq: ${escaped_team} } }) { nodes { issues(orderBy: updatedAt, first: 50, filter: { state: { type: { nin: [\\\"completed\\\",\\\"canceled\\\"] } } }) { nodes { identifier title priority assignee { name } state { name } } } } } }\"}")
  echo "$resp" | jq -r '.data.teams.nodes[0].issues.nodes[] | "\(.identifier)\t[\(.state.name)]\tP\(.priority)\t\(.assignee.name // "unassigned")\t\(.title)"' | column -t -s$'\t'
}

cmd_project() {
  local name
  name=$(arg "name")
  if [[ -z "$name" ]]; then
    echo "ERROR: project name required. Use {\"name\":\"Project Name\"}"
    exit 1
  fi
  local escaped_name
  escaped_name=$(escape_gql "$name")
  local resp
  resp=$(gql "{\"query\":\"{ projects(filter: { name: { containsIgnoreCase: ${escaped_name} } }, first: 1) { nodes { name issues(orderBy: updatedAt, first: 100) { nodes { identifier title priority assignee { name } state { name } } } } } }\"}")
  local project_name
  project_name=$(echo "$resp" | jq -r '.data.projects.nodes[0].name // "Not found"')
  echo "Project: $project_name"
  echo "---"
  echo "$resp" | jq -r '.data.projects.nodes[0].issues.nodes[] | "\(.identifier)\t[\(.state.name)]\tP\(.priority)\t\(.assignee.name // "unassigned")\t\(.title)"' | column -t -s$'\t'
}

cmd_issue() {
  local id
  id=$(arg "id")
  if [[ -z "$id" ]]; then
    echo "ERROR: issue identifier required. Use {\"id\":\"TEAM-123\"}"
    exit 1
  fi
  local escaped_id
  escaped_id=$(escape_gql "$id")
  local resp
  resp=$(gql "{\"query\":\"{ issue(id: ${escaped_id}) { identifier title description priority priorityLabel state { name } assignee { name } team { key name } project { name } labels { nodes { name } } createdAt updatedAt comments { nodes { body createdAt user { name } } } } }\"}")
  echo "$resp" | jq -r '
    .data.issue |
    "[\(.identifier)] \(.title)",
    "Status: \(.state.name)  Priority: \(.priorityLabel)  Assignee: \(.assignee.name // "unassigned")",
    "Team: \(.team.key) (\(.team.name))  Project: \(.project.name // "-")",
    "Labels: \([ .labels.nodes[].name ] | join(", ") // "-")",
    "Created: \(.createdAt[:10])  Updated: \(.updatedAt[:10])",
    "",
    (.description // "(no description)"),
    "",
    if (.comments.nodes | length) > 0 then
      "--- Comments ---",
      (.comments.nodes[] | "\(.user.name) (\(.createdAt[:10])): \(.body)")
    else "No comments" end'
}

cmd_branch() {
  local id
  id=$(arg "id")
  if [[ -z "$id" ]]; then
    echo "ERROR: issue identifier required. Use {\"id\":\"TEAM-123\"}"
    exit 1
  fi
  local escaped_id
  escaped_id=$(escape_gql "$id")
  local resp
  resp=$(gql "{\"query\":\"{ issue(id: ${escaped_id}) { branchName } }\"}")
  echo "$resp" | jq -r '.data.issue.branchName'
}

cmd_create() {
  local team title description
  team=$(arg "team")
  title=$(arg "title")
  description=$(arg "description")

  if [[ -z "$team" ]]; then
    team="${LINEAR_DEFAULT_TEAM:-}"
  fi
  if [[ -z "$team" ]]; then
    echo "ERROR: team key required. Use {\"team\":\"TEAM_KEY\"} or set LINEAR_DEFAULT_TEAM"
    exit 1
  fi
  if [[ -z "$title" ]]; then
    echo "ERROR: title required. Use {\"title\":\"Issue title\"}"
    exit 1
  fi

  # Resolve team key to team ID
  local escaped_team
  escaped_team=$(escape_gql "$team")
  local team_resp
  team_resp=$(gql "{\"query\":\"{ teams(filter: { key: { eq: ${escaped_team} } }) { nodes { id } } }\"}")
  local team_id
  team_id=$(echo "$team_resp" | jq -r '.data.teams.nodes[0].id // empty')
  if [[ -z "$team_id" ]]; then
    echo "ERROR: Team '$team' not found"
    exit 1
  fi

  local escaped_title escaped_desc
  escaped_title=$(escape_gql "$title")
  escaped_desc=$(escape_gql "${description:-}")

  local mutation
  if [[ -n "$description" ]]; then
    mutation="{\"query\":\"mutation { issueCreate(input: { teamId: \\\"${team_id}\\\", title: ${escaped_title}, description: ${escaped_desc} }) { success issue { identifier title url } } }\"}"
  else
    mutation="{\"query\":\"mutation { issueCreate(input: { teamId: \\\"${team_id}\\\", title: ${escaped_title} }) { success issue { identifier title url } } }\"}"
  fi

  local resp
  resp=$(gql "$mutation")
  echo "$resp" | jq -r '.data.issueCreate.issue | "Created: \(.identifier) — \(.title)\nURL: \(.url)"'
}

cmd_comment() {
  local id body
  id=$(arg "id")
  body=$(arg "body")
  if [[ -z "$id" || -z "$body" ]]; then
    echo "ERROR: id and body required. Use {\"id\":\"TEAM-123\",\"body\":\"Comment text\"}"
    exit 1
  fi

  # Resolve issue identifier to issue ID
  local escaped_id
  escaped_id=$(escape_gql "$id")
  local issue_resp
  issue_resp=$(gql "{\"query\":\"{ issue(id: ${escaped_id}) { id } }\"}")
  local issue_id
  issue_id=$(echo "$issue_resp" | jq -r '.data.issue.id // empty')
  if [[ -z "$issue_id" ]]; then
    echo "ERROR: Issue '$id' not found"
    exit 1
  fi

  local escaped_body
  escaped_body=$(escape_gql "$body")
  local resp
  resp=$(gql "{\"query\":\"mutation { commentCreate(input: { issueId: \\\"${issue_id}\\\", body: ${escaped_body} }) { success comment { id createdAt } } }\"}")
  echo "$resp" | jq -r '.data.commentCreate | if .success then "Comment added successfully" else "Failed to add comment" end'
}

cmd_status() {
  local id status_name
  id=$(arg "id")
  status_name=$(arg "status")
  if [[ -z "$id" || -z "$status_name" ]]; then
    echo "ERROR: id and status required. Use {\"id\":\"TEAM-123\",\"status\":\"progress\"}"
    exit 1
  fi

  # Map friendly names to Linear state names
  local state_name
  case "$status_name" in
    todo)     state_name="Todo" ;;
    progress) state_name="In Progress" ;;
    review)   state_name="In Review" ;;
    done)     state_name="Done" ;;
    blocked)  state_name="Blocked" ;;
    *)        state_name="$status_name" ;;
  esac

  # Resolve issue
  local escaped_id
  escaped_id=$(escape_gql "$id")
  local issue_resp
  issue_resp=$(gql "{\"query\":\"{ issue(id: ${escaped_id}) { id team { id } } }\"}")
  local issue_id team_id
  issue_id=$(echo "$issue_resp" | jq -r '.data.issue.id // empty')
  team_id=$(echo "$issue_resp" | jq -r '.data.issue.team.id // empty')
  if [[ -z "$issue_id" ]]; then
    echo "ERROR: Issue '$id' not found"
    exit 1
  fi

  # Find matching workflow state for the team
  local escaped_state
  escaped_state=$(escape_gql "$state_name")
  local states_resp
  states_resp=$(gql "{\"query\":\"{ workflowStates(filter: { team: { id: { eq: \\\"${team_id}\\\" } }, name: { containsIgnoreCase: ${escaped_state} } }) { nodes { id name } } }\"}")
  local state_id
  state_id=$(echo "$states_resp" | jq -r '.data.workflowStates.nodes[0].id // empty')
  if [[ -z "$state_id" ]]; then
    echo "ERROR: State '$state_name' not found for this team"
    exit 1
  fi

  local resp
  resp=$(gql "{\"query\":\"mutation { issueUpdate(id: \\\"${issue_id}\\\", input: { stateId: \\\"${state_id}\\\" }) { success issue { identifier state { name } } } }\"}")
  echo "$resp" | jq -r '.data.issueUpdate.issue | "\(.identifier) → \(.state.name)"'
}

cmd_assign() {
  local id user
  id=$(arg "id")
  user=$(arg "user")
  if [[ -z "$id" || -z "$user" ]]; then
    echo "ERROR: id and user required. Use {\"id\":\"TEAM-123\",\"user\":\"userName\"}"
    exit 1
  fi

  # Resolve issue
  local escaped_id
  escaped_id=$(escape_gql "$id")
  local issue_resp
  issue_resp=$(gql "{\"query\":\"{ issue(id: ${escaped_id}) { id } }\"}")
  local issue_id
  issue_id=$(echo "$issue_resp" | jq -r '.data.issue.id // empty')
  if [[ -z "$issue_id" ]]; then
    echo "ERROR: Issue '$id' not found"
    exit 1
  fi

  # Find user by display name
  local escaped_user
  escaped_user=$(escape_gql "$user")
  local user_resp
  user_resp=$(gql "{\"query\":\"{ users(filter: { displayName: { containsIgnoreCase: ${escaped_user} } }) { nodes { id name } } }\"}")
  local user_id
  user_id=$(echo "$user_resp" | jq -r '.data.users.nodes[0].id // empty')
  if [[ -z "$user_id" ]]; then
    echo "ERROR: User '$user' not found"
    exit 1
  fi

  local resp
  resp=$(gql "{\"query\":\"mutation { issueUpdate(id: \\\"${issue_id}\\\", input: { assigneeId: \\\"${user_id}\\\" }) { success issue { identifier assignee { name } } } }\"}")
  echo "$resp" | jq -r '.data.issueUpdate.issue | "\(.identifier) → assigned to \(.assignee.name)"'
}

cmd_priority() {
  local id priority_name
  id=$(arg "id")
  priority_name=$(arg "priority")
  if [[ -z "$id" || -z "$priority_name" ]]; then
    echo "ERROR: id and priority required. Use {\"id\":\"TEAM-123\",\"priority\":\"high\"}"
    exit 1
  fi

  local priority_val
  case "$priority_name" in
    none)   priority_val=0 ;;
    urgent) priority_val=1 ;;
    high)   priority_val=2 ;;
    medium) priority_val=3 ;;
    low)    priority_val=4 ;;
    *)
      echo "ERROR: Invalid priority '$priority_name'. Use: urgent, high, medium, low, none"
      exit 1
      ;;
  esac

  # Resolve issue
  local escaped_id
  escaped_id=$(escape_gql "$id")
  local issue_resp
  issue_resp=$(gql "{\"query\":\"{ issue(id: ${escaped_id}) { id } }\"}")
  local issue_id
  issue_id=$(echo "$issue_resp" | jq -r '.data.issue.id // empty')
  if [[ -z "$issue_id" ]]; then
    echo "ERROR: Issue '$id' not found"
    exit 1
  fi

  local resp
  resp=$(gql "{\"query\":\"mutation { issueUpdate(id: \\\"${issue_id}\\\", input: { priority: ${priority_val} }) { success issue { identifier priorityLabel } } }\"}")
  echo "$resp" | jq -r '.data.issueUpdate.issue | "\(.identifier) → priority: \(.priorityLabel)"'
}

cmd_standup() {
  echo "=== Daily Standup ==="
  echo ""

  echo "📋 YOUR TODOS:"
  cmd_my_todos 2>/dev/null || echo "  (none)"
  echo ""

  echo "🚨 URGENT/HIGH PRIORITY:"
  cmd_urgent 2>/dev/null || echo "  (none)"
  echo ""

  echo "🔍 IN REVIEW:"
  local resp
  resp=$(gql '{"query":"{ viewer { assignedIssues(first: 20, filter: { state: { name: { eq: \"In Review\" } } }) { nodes { identifier title } } } }"}')
  echo "$resp" | jq -r '.data.viewer.assignedIssues.nodes[] | "  \(.identifier)  \(.title)"' 2>/dev/null || echo "  (none)"
  echo ""

  echo "✅ RECENTLY COMPLETED (last 7 days):"
  local resp2
  resp2=$(gql '{"query":"{ viewer { assignedIssues(first: 20, orderBy: updatedAt, filter: { state: { type: { eq: \"completed\" } }, updatedAt: { gte: \"'$(date -d '7 days ago' -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -v-7d -u +%Y-%m-%dT%H:%M:%SZ)'"} }) { nodes { identifier title completedAt } } } }"}')
  echo "$resp2" | jq -r '.data.viewer.assignedIssues.nodes[] | "  \(.identifier)  \(.title)  (completed \(.completedAt[:10]))"' 2>/dev/null || echo "  (none)"
}

cmd_projects() {
  local resp
  resp=$(gql '{"query":"{ projects(first: 50, orderBy: updatedAt) { nodes { name state progress teams { nodes { key } } lead { name } issues { nodes { id } } } } }"}')
  echo "$resp" | jq -r '.data.projects.nodes[] | "\(.name)\t\(.state)\t\((.progress * 100) | floor)%\t\(.lead.name // "-")\t\(.issues.nodes | length) issues\t\([.teams.nodes[].key] | join(","))"' | column -t -s$'\t'
}

cmd_help() {
  cat <<'HELP'
Linear CLI — Commands:

  my-issues         Your assigned open issues
  my-todos          Your Todo/Backlog items
  urgent            Urgent/High priority across all teams

  teams             List available teams
  team              Issues for a team (args: team)
  project           Issues in a project (args: name)
  issue             Issue details + comments (args: id)
  branch            Git branch name for issue (args: id)

  create            Create issue (args: team, title, description?)
  comment           Add comment (args: id, body)
  status            Set status (args: id, status)
  assign            Assign issue (args: id, user)
  priority          Set priority (args: id, priority)

  standup           Daily standup summary
  projects          All projects with progress
HELP
}

# ── dispatch ─────────────────────────────────────────────────────────────────

case "$ACTION" in
  teams)      cmd_teams ;;
  my-issues)  cmd_my_issues ;;
  my-todos)   cmd_my_todos ;;
  urgent)     cmd_urgent ;;
  team)       cmd_team ;;
  project)    cmd_project ;;
  issue)      cmd_issue ;;
  branch)     cmd_branch ;;
  create)     cmd_create ;;
  comment)    cmd_comment ;;
  status)     cmd_status ;;
  assign)     cmd_assign ;;
  priority)   cmd_priority ;;
  standup)    cmd_standup ;;
  projects)   cmd_projects ;;
  help|*)     cmd_help ;;
esac
