---
name: review-pr
description: Review a smart-road pull request against ticket scope, audit gates, cross-track ownership, and automated quality checks. Use when the user asks to review, check, or audit a PR or branch for smart-road.
---

# Review PR — smart-road

Use this skill when the user asks to review, check, or audit a PR for **smart-road**.

## When to activate

- "Review this PR"
- "Check the PR for smart-road"
- "Is A03 ready to merge?"
- `/review-pr` or similar
- User provides a PR number, branch name, or GitHub link

## Procedure

1. **Resolve the target branch**
   - If the user gives a PR number: `gh pr view <N> --json headRefName,title,body,files`
   - If the user gives a branch name: check it out locally if needed
   - If already on the PR branch, continue

2. **Identify the ticket**
   - Parse ticket ID from branch (`feat/A04-*`), PR title, or `docs/pr-messages/`
   - Load the ticket row from `docs/ticket-tracker.md`
   - Load `docs/pr-messages/{TicketID}-*-pr.md` if it exists

3. **Follow the full playbook**
   - Read and execute `.agents/workflows/review-pr.md` end to end
   - Run all automated checks locally (`cargo test`, `cargo clippy`, `cargo fmt --check`, `cargo build`)
   - Verify REQ/AUD traceability against `docs/requirements.md` and `docs/audit.md`
   - Check cross-track file ownership per `docs/SDS.md` §13.1

4. **Optional Bugbot pass**
   - If the user wants deeper code review, or findings are ambiguous, launch one `bugbot` subagent:
     - `readonly: true`
     - `description: "Bugbot"`
     - `subagent_type: "bugbot"`
   - Prompt shape:
     ```text
     Full Repository Path: <absolute path to smart-road>
     Diff: branch changes
     Custom Instructions: Review against AGENTS.md and docs/SDS.md §13. Flag cross-track file edits, SDL2 resource handling, and simulation correctness. Ignore clippy-catchable style nits.
     ```
   - Merge Bugbot findings into the review table; deduplicate.

5. **Return the structured review**
   - Use the output format from `.agents/workflows/review-pr.md` §6
   - Verdict must be **Approve**, **Request changes**, or **Block**
   - Do not fix code unless the user asks

## Quick invocation prompt

Copy and adapt when starting a review session:

```text
Review the smart-road PR on branch {BRANCH_OR_PR}.

Repository: /home/someuan/smart-road
Follow: .agents/workflows/review-pr.md

Steps:
1. Identify ticket ID and load docs/pr-messages artifact + ticket-tracker row
2. Run: cargo test, cargo clippy -- -D warnings, cargo fmt --check, cargo build
3. Verify REQ/AUD traceability from the PR message against the diff
4. Check cross-track file ownership (docs/SDS.md §13.1)
5. Return structured review with verdict and findings table

Do not fix code. Do not merge.
```

## Blocking conditions (always Request changes or Block)

- Failing automated checks
- Missing `docs/pr-messages/{TicketID}-*-pr.md`
- Cross-track file edits without SDS update
- Claimed AUD/REQ coverage not supported by code or evidence
- Ticket scope creep or unrelated changes
- Unapproved new dependencies
