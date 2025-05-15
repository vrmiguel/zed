Executes a shell command and returns the combined output.

This tool spawns a shell process, combines stdout and stderr into one interleaved stream as they are produced (preserving the order of writes), and captures that stream into a string which is returned.

By default, it uses:
- `bash` with `-c` argument on Unix-based systems
- `cmd` with `/c` argument on Windows

You can optionally specify a custom shell using the `shell` parameter.

Make sure you use the `cd` parameter to navigate to one of the root directories of the project. NEVER do it as part of the `command` itself, otherwise it will error.

Remember that each invocation of this tool will spawn a new shell process, so you can't rely on any state from previous invocations.
