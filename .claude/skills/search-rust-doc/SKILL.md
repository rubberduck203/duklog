---
name: search-rust-doc
description: Search Rust crate documentation for a type, trait, function, or method. Usage: /search-rust-doc {crate} {query}
---

# Search Rust Documentation

Search the offline rustdoc output for a crate. Arguments (from `$ARGUMENTS`): the first word is the crate/package name, the rest is the search query.

## Process

Use the Task tool to invoke the `search-rust-doc` subagent with the prompt `Package: {crate}\nQuery: {query}`. Return the agent's findings directly to the user. If the user did not provide both a crate and a query, ask for the missing piece before invoking the agent.

## Examples

```
/search-rust-doc difa TagEncoder
/search-rust-doc ratatui Frame
/search-rust-doc crossterm event::KeyCode
/search-rust-doc ratatui widgets::Block title
```
