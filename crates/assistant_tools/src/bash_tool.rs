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
    /// The shell to use for executing the command. Defaults to "bash" on Unix systems and "cmd" on Windows.
    #[serde(default)]
    shell: Option<String>,
}

pub struct BashTool;

/// Determine the default shell and arguments based on the platform
fn get_default_shell_and_args() -> (&'static str, &'static str) {
    #[cfg(target_os = "windows")]
    {
        ("cmd", "/c")
    }
    #[cfg(not(target_os = "windows"))]
    {
        ("bash", "-c")
    }
}

impl Tool for BashTool {
    fn name(&self) -> String {
        "bash".to_string()
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

        // Get the default shell and arg based on the platform
        let (default_shell, default_arg) = get_default_shell_and_args();

        // Use the specified shell or default to the platform-specific shell
        let shell = input.shell.as_deref().unwrap_or(default_shell);

        // Select the appropriate arg for the shell
        // For customized shells we still use the platform default arg
        let shell_arg = if shell == default_shell {
            default_arg
        } else {
            // If the user specified a custom shell, we'll use the platform default arg
            // This is a simple approach; in a more advanced implementation, we could map
            // specific shells to their appropriate args
            default_arg
        };

        cx.spawn(|_| async move {
            // Add 2>&1 to merge stderr into stdout for proper interleaving.
            let command = format!("({}) 2>&1", input.command);

            let output = new_smol_command(shell)
                .arg(shell_arg)
                .arg(&command)
                .current_dir(working_directory)
                .output()
                .await
                .context(format!("Failed to execute command with shell '{}'", shell))?;

            let output_string = String::from_utf8_lossy(&output.stdout).to_string();

            if output.status.success() {
                if output_string.is_empty() {
                    Ok("Command executed successfully.".to_string())
                } else {
                    Ok(output_string)
                }
            } else {
                Ok(format!(
                    "Command failed with exit code {}\n{}",
                    output.status.code().unwrap_or(-1),
                    &output_string
                ))
            }
        })
    }
}
