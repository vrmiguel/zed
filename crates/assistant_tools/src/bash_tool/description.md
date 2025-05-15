Executes a shell command one-liner and returns the combined output.

This tool spawns a shell process (bash by default), combines stdout and stderr into one interleaved stream as they are produced (preserving the order of writes), and captures that stream into a string which is returned.

Make sure you use the `cd` parameter to navigate to one of the root directories of the project. NEVER do it as part of the `command` itself, otherwise it will error.

You can specify the shell to use with the `shell` parameter (e.g., "sh", "zsh", "pwsh", "cmd.exe") and the shell arguments with the `shell_args` parameter. By default, it uses "bash" with the "-c" argument.

Remember that each invocation of this tool will spawn a new shell process, so you can't rely on any state from previous invocations.
