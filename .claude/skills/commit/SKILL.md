---
name: commit
description: Commit changes after running CI checks
---

# Commit Workflow

1. Run `make ci` — all checks must pass before proceeding
2. Run `git status` to see all changes
3. Run `git diff` to review staged and unstaged changes
4. Run `git log --oneline -5` to see recent commit message style
5. Stage relevant files (never stage `.env`, credentials, or secrets)
6. Draft a concise commit message: focus on "why" not "what"
7. Commit with trailer:
   ```
   Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
   ```
8. Run `git status` to verify success

If `make ci` fails, fix the issues first. Do not skip checks.
