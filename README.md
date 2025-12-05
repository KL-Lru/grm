## grm

Git CLI tool.

## Concept

`grm` is a CLI tool to manage multiple Git repositories and their worktrees efficiently.
It provides commands to clone, list, and remove repositories in a structured directory layout.
This CLI tool also offers advanced worktree management features, such as sharing files/directories between worktrees.

## Commands

### root

Display the root directory for managing repositories.

```bash
grm root
```

### clone

Clone a Git repository.
Repository path will be `$(grm root)/<host>/<user>/<repo>+<branch>`.
If branch is not specified, the default branch of the repository is used.

```bash
grm clone <repository_url> [-b <branch>]
```

### list

List all managed Git repositories.
All paths are listed relative to the `grm root` directory.
If `--full-path` is specified, full paths are listed.

```bash
grm list [--full-path]
```

### remove

Remove a managed Git repository.

```bash
grm remove <repository_url>
```

### worktree split

Create a new worktree from an existing repository.
Worktree path will be `$(grm root)/<host>/<user>/<repo>+<branch>`.

If `worktree share` has been used, the new worktree will have shared files/directories automatically.

```bash
# in managed repository directory
grm worktree split <branch>
```

If this command is called outside a managed repository directory, it will fail.

### worktree remove

Remove a worktree from a managed repository.

```bash
# in managed repository directory
grm worktree remove <branch>
```

If this command is called outside a managed repository directory, it will fail.

### worktree share

Share a file or directory between all worktrees of a repository.
Internally, this command creates a symbolic link in each worktree pointing to the shared file/directory.
The shared file/directory is stored in `$(grm root)/.shared/<host>/<user>/<repo>/<path>`.

If `worktree split` has been used, the existing worktree will also share the file/directory.
This operation **overwrites** the file/directory in each worktree.

```bash
grm worktree share <path>
```

If path is not in a managed repository, this command will fail.

### worktree unshare

Remove sharing of a file or directory between worktrees of a repository.
Removes all symbolic links. The original file/directory can be restored from `$(grm root)/.shared/<host>/<user>/<repo>/<path>`.

```bash
# (optional)
# keep a copy of the shared file/directory in the worktree
# grm isolate <shared_path>
grm worktree unshare <shared_path>
```

If path is not in a managed repository, this command will fail.
If path is not shared, this command performs no operation.

### worktree isolate

Isolate a worktree from shared files/directories.
This operation removes the symbolic link and copies the shared file/directory from `$(grm root)/.shared/<host>/<user>/<repo>/<path>` to the worktree.

```bash
grm worktree isolate <shared_path>
```

### Configuration

Load Priority order:

1. `~/.grmrc` (TOML format)
2. in `~/.gitconfig` ([grm] section)

| key    | description                                                                                                      | default | env        |
| ------ | ---------------------------------------------------------------------------------------------------------------- | ------- | ---------- |
| `root` | Root directory for managing repositories.<br>If changed, you need to move existing repositories to the new root. | `~/grm` | `GRM_ROOT` |

## Examples

```bash
# Set custom root directory via environment variable
export GRM_ROOT="$HOME/my_grm_root"
# clone a repository to `$GRM_ROOT/github.com/user/repo+main`
grm clone git://github.com/user/repo.git
# move to the repository directory
cd $(grm list --full-path | peco)
# split a new worktree for branch 'feature/awesome'
grm worktree split feature/awesome
# share a file between worktrees
grm worktree share .env
grm worktree share node_modules
# Develop your features!
```
