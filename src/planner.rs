use std::path::Path;
use serde::Serialize;
use regex::Regex;

use crate::LoomError;

/// A work plan derived from a brief.
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct WorkPlan {
    pub brief: String,
    pub steps: Vec<WorkStep>,
    pub validation: Vec<String>,
    pub agent_assignments: Vec<AgentAssignment>,
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct WorkStep {
    pub order: usize,
    pub description: String,
    pub kind: StepKind,
    pub files: Vec<String>,
    pub depends_on: Vec<usize>,
    pub validation_command: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Create,
    Modify,
    Test,
    Document,
    Review,
    Deploy,
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct AgentAssignment {
    pub agent: String,
    pub steps: Vec<usize>,
    pub claims: Vec<String>,
}

/// Analyze a brief and project context to generate a work plan.
pub fn plan_from_brief(
    repo: &Path,
    brief: &str,
    agents: &[String],
) -> Result<WorkPlan, LoomError> {
    let mut steps = Vec::new();
    let mut order = 1;

    // Detect project type for context
    let project_type = detect_project_type(repo);

    // Parse the brief for action keywords
    let keywords = extract_keywords(brief);

    // Generate steps based on keywords and project context
    if keywords.contains(&"add") || keywords.contains(&"create") || keywords.contains(&"new") {
        steps.push(WorkStep {
            order,
            description: format!("Create new files for: {}", summarize(brief, 60)),
            kind: StepKind::Create,
            files: guess_files_from_brief(brief, &project_type),
            depends_on: vec![],
            validation_command: build_command(&project_type),
        });
        order += 1;
    }

    if keywords.contains(&"fix") || keywords.contains(&"bug") || keywords.contains(&"repair") {
        steps.push(WorkStep {
            order,
            description: format!("Fix: {}", summarize(brief, 60)),
            kind: StepKind::Modify,
            files: guess_files_from_brief(brief, &project_type),
            depends_on: vec![],
            validation_command: test_command(&project_type),
        });
        order += 1;
    }

    if keywords.contains(&"refactor") || keywords.contains(&"update") || keywords.contains(&"modify") {
        steps.push(WorkStep {
            order,
            description: format!("Modify: {}", summarize(brief, 60)),
            kind: StepKind::Modify,
            files: guess_files_from_brief(brief, &project_type),
            depends_on: vec![],
            validation_command: build_command(&project_type),
        });
        order += 1;
    }

    // Always add test step if there are code changes
    if !steps.is_empty() {
        let code_steps: Vec<usize> = steps.iter().map(|s| s.order).collect();
        steps.push(WorkStep {
            order,
            description: "Run tests to validate changes".to_string(),
            kind: StepKind::Test,
            files: vec![],
            depends_on: code_steps,
            validation_command: test_command(&project_type),
        });
        order += 1;
    }

    // Add review step
    steps.push(WorkStep {
        order,
        description: "Review changes with rivet before commit".to_string(),
        kind: StepKind::Review,
        files: vec![],
        depends_on: vec![order - 1],
        validation_command: Some("rivet check".to_string()),
    });

    // Generate validation commands
    let mut validation = Vec::new();
    if let Some(cmd) = build_command(&project_type) {
        validation.push(cmd);
    }
    if let Some(cmd) = test_command(&project_type) {
        validation.push(cmd);
    }
    validation.push("rivet check".to_string());

    // Assign agents if provided
    let agent_assignments = if agents.is_empty() {
        vec![]
    } else {
        assign_agents(agents, &steps)
    };

    Ok(WorkPlan {
        brief: brief.to_string(),
        steps,
        validation,
        agent_assignments,
    })
}

/// Analyze the current project state and suggest next actions.
pub fn suggest_next(repo: &Path) -> Result<Vec<String>, LoomError> {
    let mut suggestions = Vec::new();
    let project_type = detect_project_type(repo);

    // Check for dirty git state
    if let Ok(output) = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let dirty_count = text.lines().count();
            if dirty_count > 0 {
                suggestions.push(format!("{dirty_count} uncommitted change(s). Run `rivet check` before committing."));
            }
        }
    }

    // Check for failing tests
    if let Some(cmd) = test_command(&project_type) {
        suggestions.push(format!("Run `{cmd}` to validate current state."));
    }

    // Check for outdated deps
    if repo.join("Cargo.toml").exists() || repo.join("package.json").exists() {
        suggestions.push("Run `quarry audit` to check dependency health.".to_string());
    }

    // Check for missing evidence
    if !repo.join(".agent-witness").exists() {
        suggestions.push("No witness evidence found. Record test runs with `witness run`.".to_string());
    }

    // Check for missing atlas index
    if !repo.join(".agent-atlas").exists() {
        suggestions.push("No atlas index. Run `atlas scan` to build the dependency graph.".to_string());
    }

    if suggestions.is_empty() {
        suggestions.push("Project looks clean. Check `trail summary` for recent work history.".to_string());
    }

    Ok(suggestions)
}

/// Generate latch commands from a plan.
pub fn emit_latch_commands(plan: &WorkPlan) -> Vec<String> {
    let mut commands = Vec::new();

    commands.push("latch init".to_string());

    for assignment in &plan.agent_assignments {
        for claim in &assignment.claims {
            commands.push(format!(
                "latch claim acquire --actor {} --path {} --ttl 2h",
                assignment.agent, claim
            ));
        }
    }

    for step in &plan.steps {
        commands.push(format!(
            "latch task add --title \"{}\" --actor {}",
            step.description,
            plan.agent_assignments.first()
                .map(|a| a.agent.as_str())
                .unwrap_or("agent")
        ));
    }

    commands
}

fn detect_project_type(repo: &Path) -> String {
    if repo.join("Cargo.toml").exists() { "rust".to_string() }
    else if repo.join("package.json").exists() { "node".to_string() }
    else if repo.join("pyproject.toml").exists() { "python".to_string() }
    else if repo.join("go.mod").exists() { "go".to_string() }
    else { "unknown".to_string() }
}

fn extract_keywords(brief: &str) -> Vec<&str> {
    let action_words = ["add", "create", "new", "fix", "bug", "repair",
        "refactor", "update", "modify", "delete", "remove", "test", "deploy"];
    let lower = brief.to_lowercase();
    action_words.iter()
        .filter(|w| lower.contains(**w))
        .copied()
        .collect()
}

fn guess_files_from_brief(brief: &str, project_type: &str) -> Vec<String> {
    let re = Regex::new(r"(?:src/\S+|lib/\S+|tests?/\S+|\S+\.\w{1,4})").unwrap();
    let mut files: Vec<String> = re.find_iter(brief).map(|m| m.as_str().to_string()).collect();

    if files.is_empty() {
        // Suggest likely files based on project type
        match project_type {
            "rust" => files.push("src/".to_string()),
            "node" => files.push("src/".to_string()),
            "python" => files.push("src/".to_string()),
            _ => {}
        }
    }

    files
}

fn build_command(project_type: &str) -> Option<String> {
    match project_type {
        "rust" => Some("cargo check".to_string()),
        "node" => Some("npm run build".to_string()),
        "python" => Some("python -m py_compile".to_string()),
        "go" => Some("go build ./...".to_string()),
        _ => None,
    }
}

fn test_command(project_type: &str) -> Option<String> {
    match project_type {
        "rust" => Some("cargo test".to_string()),
        "node" => Some("npm test".to_string()),
        "python" => Some("pytest".to_string()),
        "go" => Some("go test ./...".to_string()),
        _ => None,
    }
}

fn assign_agents(agents: &[String], steps: &[WorkStep]) -> Vec<AgentAssignment> {
    // Simple round-robin assignment
    agents.iter().enumerate().map(|(i, agent)| {
        let my_steps: Vec<usize> = steps.iter()
            .filter(|s| s.order % agents.len() == i % agents.len())
            .map(|s| s.order)
            .collect();

        let claims: Vec<String> = steps.iter()
            .filter(|s| my_steps.contains(&s.order))
            .flat_map(|s| s.files.clone())
            .collect();

        AgentAssignment {
            agent: agent.clone(),
            steps: my_steps,
            claims,
        }
    }).collect()
}

fn summarize(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
