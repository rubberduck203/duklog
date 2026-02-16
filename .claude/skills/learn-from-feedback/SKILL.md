---
name: learn-from-feedback
description: Process PR feedback, user corrections, or code review findings and update project memory, rules, and standards accordingly. Use proactively after receiving PR review comments, user corrections, or when patterns emerge from code review.
user-invocable: true
argument-hint: "[PR number or description of feedback]"
---

# Learn from Feedback

When receiving feedback (PR comments, user corrections, code review findings), update the project's knowledge base so the same issues don't recur.

## Process

1. **Identify the feedback**: Read the PR comments (`gh pr view $ARGUMENTS --comments`), or parse the user's correction from context
2. **Classify the learning**:
   - **Coding standard** → update `.claude/skills/coding-standards/SKILL.md`
   - **Domain knowledge** → update `.claude/rules/domain.md`
   - **Testing practice** → update `.claude/rules/testing.md`
   - **Workflow/process** → update `CLAUDE.md` (Git Workflow section) or relevant skill
   - **Project-specific pattern** → update auto memory (`MEMORY.md`)
3. **Apply the fix**: If the feedback points to a code issue, fix it
4. **Update the knowledge base**: Write the learning to the appropriate file so it persists
5. **Summarize**: Report what was learned and where it was saved

## Rules

- Be conservative: only save verified, stable patterns — not one-off corrections
- Keep entries concise: one bullet point per learning
- Don't duplicate: check if the learning already exists before adding
- If unsure where a learning belongs, prefer auto memory (lowest commitment)
