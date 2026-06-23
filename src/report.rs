use crate::planner::WorkPlan;
use crate::LoomError;

pub fn print_plan(plan: &WorkPlan, is_json: bool) -> Result<(), LoomError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "plan": plan,
        }))?);
    } else {
        println!("loom plan: {}", truncate(&plan.brief, 60));
        println!();

        for step in &plan.steps {
            let deps_str = if step.depends_on.is_empty() {
                String::new()
            } else {
                format!(" (after step {})", step.depends_on.iter()
                    .map(|d| d.to_string()).collect::<Vec<_>>().join(", "))
            };

            println!("  {}. [{}] {}{}", step.order, step_kind_label(step.kind), step.description, deps_str);

            if !step.files.is_empty() {
                println!("     Files: {}", step.files.join(", "));
            }
            if let Some(ref cmd) = step.validation_command {
                println!("     Validate: {cmd}");
            }
        }

        if !plan.agent_assignments.is_empty() {
            println!();
            println!("  Agent assignments:");
            for a in &plan.agent_assignments {
                println!("    {} -> steps {:?}, claims {:?}", a.agent, a.steps, a.claims);
            }
        }

        if !plan.validation.is_empty() {
            println!();
            println!("  Validation commands:");
            for cmd in &plan.validation {
                println!("    $ {cmd}");
            }
        }
    }
    Ok(())
}

pub fn print_next(suggestions: &[String], is_json: bool) -> Result<(), LoomError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "suggestions": suggestions,
        }))?);
    } else {
        println!("loom next:");
        println!();
        for (i, s) in suggestions.iter().enumerate() {
            println!("  {}. {s}", i + 1);
        }
    }
    Ok(())
}

pub fn print_latch_commands(commands: &[String], is_json: bool) -> Result<(), LoomError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "commands": commands,
        }))?);
    } else {
        println!("loom emit: latch commands for the plan");
        println!();
        for cmd in commands {
            println!("  $ {cmd}");
        }
    }
    Ok(())
}

fn step_kind_label(kind: crate::planner::StepKind) -> &'static str {
    match kind {
        crate::planner::StepKind::Create => "create",
        crate::planner::StepKind::Modify => "modify",
        crate::planner::StepKind::Test => "test",
        crate::planner::StepKind::Document => "docs",
        crate::planner::StepKind::Review => "review",
        crate::planner::StepKind::Deploy => "deploy",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = s.char_indices()
            .take_while(|(i, _)| *i < max)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!("{}...", &s[..end])
    }
}
