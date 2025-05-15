Executes a shell command and returns the combined output.

This tool spawns a shell process (bash by default, but configurable), combines stdout and stderr into one interleaved stream as they are produced (preserving the order of writes), and captures that stream into a string which is returned.

Make sure you use the `cd` parameter to navigate to one of the root directories of the project. NEVER do it as part of the `command` itself, otherwise it will error.

You can specify which shell to use with the `shell` parameter (defaults to "bash") and the shell argument with the `shell_arg` parameter (defaults to "-c").

Remember that each invocation of this tool will spawn a new shell process, so you can't rely on any state from previous invocations.
