# About

I do have a real need for `duklog`, but this project is an [Upsyde AI](upsyde.ai) research project.

In my day to day work I "pair program" with various AI assistants.
I wanted to know if I could step back from the code and focus on the requirements and architecture if I put certain guardrails in place.
Would those guardrails be sufficient? 
If not, what more might I want to add?

I also hadn't really had the time to dive into the more Claude specific things like subagents, skills, hooks, etc. and I wanted to be able to bring knowledge about these things to my clients.

I started with a `cargo new` project, a README description of the application I _wished_ I had, and a list of tools I believed I wanted to use.

1. Code coverage to ensure the agent was writing enough tests
2. Mutation testing to ensure the agent was writing tests that _fail_ when the implementation is broken
3. Hooks to ensure tests/lints get run frequently instead of relying on the agent deciding to run them
4. Property based testing to help ensure input spaces are covered by tests
5. A dedicated `code-review` "adversarial" sub-agent to force a refactor cycle prior to me reviewing any PRs.
6. A `learn-from-feedback` skill to help ensure the agent is updating its own instructions based on user feedback
7. Links to various documents about the problem domain.

Claude was able to create an implentation plan, including setting itself up with the agents, skills, hooks, etc. and largely implement this project with little feedback from me.
I was honestly surprised at how well this approach worked. 
I did review all of the code, but had little to say about most of it.
This allowed me to step back and be involved at the _architectural_ level, where I _did_ have to "convince" claude of taking a different approach a few times. 
Once, downright instructing it that it _would_ take my approach. 
Which is an odd experience, but ultimately, exactly the level of abstraction I _wanted_ to be at in this project.

The hard part of software development has never been generating code, it's been susing out requirements and creating the right architecture to meet those requirements.
The guardrails and tools above seem to have accomplished what I had hoped they would.

That said, this is perhaps not the _most_ cost effective strategy. 
This blows through the pro plan's 4-5 hour block of usage in about 15 minutes.

I did work with the agent to optimize the `.claude` directory for token usage, but it's still a little costly.
I took large breaks during the day because it was a day off really and we got about half way through the project plan for about $40 + the $20 monthly fee.
I would expect a day's development to easily cost $100+ (and that's without doing anything like _teams_ of agents.)
Is that worth it? Probably. It's about what my time costs an hour, so it would be equivalent to paying for an extra hour of work a day and getting an awful lot more in return for it.

As I now have to return to billable work, I will attempt to set off a task in the morning, then another after the usage reset moving forward.

---

Around [PR #23](https://github.com/rubberduck203/duklog/pull/23) I felt the need to get involved a little bit with the code.  
The agent really struggled with the change and went through multiple rounds with the code reviewer.  
I believe this is because some of the code files were becoming too large.  
I intervened and had it stop to do a refactoring to extract out modules and functions, but am currently unsure of how to prevent the situation from happenging to begin with.
Perhaps some tweaks to the `code-review` agent? I'm not sure exactly what kind right now though.

---

Below are the original notes I gave to claude to get the project started.

```text
# references

## tools i want to use

Mutation testing
https://mutants.rs/cautions.html
Code coverage
https://github.com/taiki-e/cargo-llvm-cov
Property based testing (QuickCheck)
https://github.com/BurntSushi/quickcheck

TUI
https://ratatui.rs/ (edited) 

## domain references

ADIF parsing
https://docs.rs/difa/0.1.1/difa/

POTA logging
https://docs.pota.app/docs/activator_reference/submitting_logs.html

https://docs.pota.app/docs/rules.html#logging-requirements

https://docs.pota.app/docs/activator_reference/ADIF_for_POTA_reference.html

ADIF reference
https://www.adif.org/

https://www.adif.org/316/ADIF_316.htm



# about the data

There are three categories of data. 


Per log: will not change after the log is created (unless an error was made in creation). Ex: station and operator callsign, POTA park, ARRL section, Maidenhead locator, etc. 
Slow moving: Things like band and mode. They may change during operation, but not often. They're still part of the QSO record, but don't need entered every time because it would be redundant. 
Fast moving: Actual contact details that change for each record, other operator’s callsign, signal report sent/rcvd, etc. 

# coding guidance

Dependency documentation can be retrieved locally by building it with cargo. 

We care about not just test coverage, but good tests. Use the code coverage and mutation tools to ensure all code is covered and that the tests fail if bugs are introduced into production code. Tests should be deterministic and fast. 

Project documentation is of upmost importance, both code documentation and dev/user markdown documentation. No item is complete without updating the docs.  (edited) 
Chris McClellan  [11:07 AM]
Rust style guide

Prefer a functional approach. This means

prefer iterators over loops (map/reduce)
prefer expressions over return
Prefer match over if let -> return
```
