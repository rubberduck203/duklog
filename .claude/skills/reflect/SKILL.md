---
name: reflect
description: Post-implementation retrospective. Run after completing any non-trivial task to extract patterns, surprises, and lessons and persist them to rules and memory without waiting for user feedback. Use proactively after creating a PR or finishing a significant feature.
---

# Reflect

After completing a significant implementation, ask these questions about the work just done and update the knowledge base with anything that should fire proactively next time.

## Process

1. **Review what was just built**: run `git diff main...HEAD` to survey the full change
2. **Ask the five questions** (answer each briefly before deciding whether to save):
   - **Pattern**: Did I invent or repeat a pattern that should be a named rule so I don't have to re-derive it?
   - **Surprise**: Did anything behave unexpectedly — a crate API, a Ratatui layout quirk, a storage edge case? Would a note have saved time?
   - **Struggle**: Did I spend more than one round-trip on something that should have been obvious? What would make it obvious next time?
   - **Missed opportunity**: Is there something I *didn't* do that I would have done if I'd noticed it earlier (refactoring, test coverage, doc gap)?
   - **Stale knowledge**: Did anything in memory/rules turn out to be wrong or outdated?
3. **Classify and save** each finding using the same destinations as `learn-from-feedback`:
   - Coding pattern → `.claude/skills/coding-standards/SKILL.md`
   - Domain fact → `.claude/rules/domain.md`
   - Testing practice → `.claude/rules/testing.md`
   - Workflow change → `CLAUDE.md` or relevant skill
   - Project-specific detail → `memory/MEMORY.md`
   - Stale entry → edit or remove the outdated line in place
4. **Summarize**: Report what was saved and where (or "Nothing new to record" if nothing qualified)

## Rules

- Only save findings that would have changed behavior on this task — not observations that are already obvious
- Be conservative: one clear bullet per finding, no speculative generalizations
- Don't duplicate existing entries — check before writing
- "Nothing new to record" is a valid and common outcome; do not force findings
