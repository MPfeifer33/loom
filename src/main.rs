mod cli;
mod planner;
mod report;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = run(&cli);
    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            if cli.is_json() {
                let err_json = serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": e.error_code(),
                        "message": e.to_string(),
                    }
                });
                eprintln!("{}", serde_json::to_string_pretty(&err_json).unwrap_or_else(|_| format!("{{\"ok\":false,\"error\":{{\"message\":\"{e}\"}}}}")));
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(code);
        }
    }
}

fn run(cli: &Cli) -> Result<(), LoomError> {
    let repo = cli.resolve_repo()?;

    match &cli.command {
        Command::Plan { brief, agents } => {
            let agent_list = agents.as_deref().unwrap_or(&[]);
            let plan = planner::plan_from_brief(&repo, brief, agent_list)?;
            report::print_plan(&plan, cli.is_json())
        }
        Command::Next => {
            let suggestions = planner::suggest_next(&repo)?;
            report::print_next(&suggestions, cli.is_json())
        }
        Command::Emit { plan: plan_file } => {
            let content = std::fs::read_to_string(plan_file)?;
            let plan: planner::WorkPlan = serde_json::from_str(&content)?;
            let commands = planner::emit_latch_commands(&plan);
            report::print_latch_commands(&commands, cli.is_json())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoomError {
    #[error("{0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl LoomError {
    pub fn exit_code(&self) -> i32 {
        match self {
            LoomError::Validation(_) => 1,
            LoomError::NotFound(_) => 3,
            LoomError::Io(_) => 2,
            LoomError::Json(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            LoomError::Validation(_) => "validation_error",
            LoomError::NotFound(_) => "not_found",
            LoomError::Io(_) => "io_error",
            LoomError::Json(_) => "json_error",
        }
    }
}
