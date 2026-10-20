# Third-party skills

Copied unmodified (2026-10-20) for this project only.

| Source | Commit | Skills |
| --- | --- | --- |
| https://github.com/majidmanzarpour/threejs-game-skills (MIT) | 8286774 | all 9 `threejs-*` |
| https://github.com/actionbook/rust-skills (MIT) | 5c40d3a | 33 of 38: everything else in this folder |

Left out of rust-skills on purpose:
- The `UserPromptSubmit` hook (`rust-skill-eval-hook.sh`): it injects mandatory routing text and an emoji banner into nearly every prompt.
- The `actionbook` MCP server (`npx @actionbookdev/mcp@latest`, unpinned) and the plugin's agents and slash commands.
- `core-dynamic-skills`, `core-fix-skill-docs`, `rust-skill-creator`: they write to (and `rm -rf`) the global `~/.claude/skills/`.
- `core-actionbook`, `core-agent-browser`: they only work with the MCP server and the `agent-browser` CLI above.

To update, re-clone at a newer commit, diff, and copy the same set.
