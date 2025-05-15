use anyhow::{anyhow, Context as _, Result};
use assistant_tool::{ActionLog, Tool};
use gpui::{App, Entity, Task};
use language_model::LanguageModelRequestMessage;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use util::command::new_smol_command;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BashToolInput {
    /// The command to execute as a one-liner.
    command: String,
    /// Working directory for the command. This must be one of the root directories of the project.
    cd: String,
    /// Shell to use for executing the command. Defaults to "bash" if not specified.
    #[serde(default = "default_shell")]
    shell: String,
    /// Shell argument for command execution. Defaults to "-c" if not specified.
    #[serde(default = "default_shell_arg")]
    shell_arg: String,
}

fn default_shell() -> String {
    "bash".to_string()
}

fn default_shell_arg() -> String {
    "-c".to_string()
}

pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> String {
        "shell".to_string()
    }

    fn description(&self) -> String {
        include_str!("./bash_tool/description.md").to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        let schema = schemars::schema_for!(BashToolInput);
        serde_json::to_value(&schema).unwrap()
    }

    fn run(
        self: Arc<Self>,
        input: serde_json::Value,
        _messages: &[LanguageModelRequestMessage],
        project: Entity<Project>,
        _action_log: Entity<ActionLog>,
        cx: &mut App,
    ) -> Task<Result<String>> {
        let input: BashToolInput = match serde_json::from_value(input) {
            Ok(input) => input,
            Err(err) => return Task::ready(Err(anyhow!(err))),
        };

        let Some(worktree) = project.read(cx).worktree_for_root_name(&input.cd, cx) else {
            return Task::ready(Err(anyhow!("Working directory not found in the project")));
        };
        let working_directory = worktree.read(cx).abs_path();

        cx.spawn(|_| async move {
            // Add 2>&1 to merge stderr into stdout for proper interleaving.
            let command = format!("({}) 2>&1", input.command);

            let output = new_smol_command(&input.shell)
                .arg(&input.shell_arg)
                .arg(&command)
                .current_dir(working_directory)
                .output()
                .await
                .context(format!("Failed to execute {} command", input.shell))?;

            let output_string = String::from_utf8_lossy(&output.stdout).to_string();

            if output.status.success() {
                if output_string.is_empty() {
                    Ok("Command executed successfully.".to_string())
                } else {
                    Ok(output_string)
                }
            } else {
                Ok(format!(
                    "{} command failed with exit code {}\n{}",
                    input.shell,
                    output.status.code().unwrap_or(-1),
                    &output_string
                ))
            }
        })
    }
}
