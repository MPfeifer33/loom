use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::LoomError;

#[derive(Parser, Debug)]
#[command(name = "loom", version, about = "Agent work planner — briefs to execution graphs")]
pub struct Cli {
    /// Project root override
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn resolve_repo(&self) -> Result<PathBuf, LoomError> {
        if let Some(ref repo) = self.repo {
            return Ok(repo.clone());
        }
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }
        std::env::current_dir().map_err(LoomError::Io)
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Plan work from a brief description
    Plan {
        /// What needs to be done
        #[arg(long)]
        brief: String,
        /// Agents available for work
        #[arg(long, num_args = 1..)]
        agents: Option<Vec<String>>,
    },
    /// Analyze the project and suggest what to work on next
    Next,
    /// Emit latch commands for a plan (dry run)
    Emit {
        /// Plan file (JSON) to convert to latch commands
        plan: PathBuf,
    },
}
