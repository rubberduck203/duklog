---
name: create-pr
description: Create a pull request after code review
disable-model-invocation: false
allowed-tools: Bash, Read, Grep, Glob, Task
---

# Create PR Workflow

1. **Run the `code-review` subagent** to review this branch. Fix all blockers before proceeding.
2. Run `git status` and `git diff main...HEAD` to understand all changes
3. Run `git log main..HEAD --oneline` to see all commits on this branch
4. Push the branch: `git push -u origin HEAD`
5. Create the PR with `gh pr create`:
   - Title: short, under 70 characters
   - Body: Summary bullets, test plan checklist, and the Claude Code footer
6. Return the PR URL

Do NOT create the PR if the code-review subagent reports blockers.
