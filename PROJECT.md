# PROJECT.md — loom

**What:** Work planner — generates structured work plans from briefs, assigns to agents, emits latch commands.

**Status:** MVP complete, published to github.com/MPfeifer33/loom

## Architecture
- `src/cli.rs` — Clap 4 CLI: `plan` (--brief, --agents), `next`, `emit`
- `src/planner.rs` — WorkPlan with WorkStep (Create/Modify/Test/Document/Review/Deploy), agent assignment via round-robin, keyword extraction, suggest_next() analyzes project state, emit_latch_commands() generates CLI commands
- `src/report.rs` — Plan visualization, next suggestions, latch command output (text + JSON)
- `src/main.rs` — Standard error handling

## Usage
```bash
# Generate a work plan from a brief
loom plan --brief "Add user authentication with JWT"

# Generate with agent assignments
loom plan --brief "Refactor database layer" --agents nix,bjarn

# Get next recommended action for current project state
loom next

# Emit latch coordination commands for a saved plan
loom emit
```

## Design Decisions
- Round-robin agent assignment (simple, fair)
- Keyword extraction from briefs to determine step types
- Integrates with latch for multi-agent coordination
- Plans serialize to JSON for persistence and recall

## Last Updated
June 22, 2026 — Initial MVP
