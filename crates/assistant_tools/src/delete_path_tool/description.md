Deletes the file or directory (and the directory's contents, recursively) at the specified path in the project, and returns confirmation of the deletion.

By default, this tool requires confirmation before performing any deletion for safety. Set `require_confirmation: false` to proceed with deletion. You can also control whether files are moved to trash (default) or permanently deleted by setting `use_trash: true` or `use_trash: false` respectively.
