---
name: ze-pr
description: Draft, review, create, and update pull requests with plain English outcome-focused titles, a consistent body, honest testing results, and before/after evidence for visual changes. Use when preparing a PR title or description, opening a PR, or updating its presentation.
---

# Ze PR

Write every PR title, body section, media label, and generation credit in English. Explain the change to a reader who has not seen the conversation or diff.

## Understand the PR

- Read applicable repository instructions and inspect the complete diff against the intended base branch. The final diff is the source of truth for the title and body.
- Identify the user's problem, the solution, and the resulting behavior. For maintenance or documentation, explain the benefit to developers or agents without inventing an application feature.
- Gather the checks actually performed and determine whether the diff changes the visible UI, including motion or other behavior over time.
- When the scope changes, rewrite the title and body around the final implementation. Leave out abandoned approaches and conversational history unless they explain a relevant tradeoff.

## Write the title

Use `type(scope): plain English outcome`. Choose a conventional type such as `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, or `chore` that matches the change. Use a short scope when it helps; omit it otherwise. Preserve the repository's applicable title conventions.

Write a simple, human-readable description of what becomes possible, what gets fixed, or what becomes easier. Use an imperative verb such as `let`, `keep`, `restore`, or `prevent`. Keep implementation details, class and function names, file paths, and algorithms out of the description; an established technical area name may remain in the scope. Omit the final period.

Examples, assuming the diff supports the stated outcome:

- Weak: `feat(runtime): implement runtime export`
  Better: `feat(runtime): let authors export runtime for remote hosting`
- Weak: `fix(cards): fix serialization`
  Better: `fix(cards): keep card descriptions after reopening a workspace`
- Weak: `docs(skills): add PR instructions`
  Better: `docs(skills): help agents explain what a PR changes for users`

## Write the body

Use the following sections in this exact order. Begin the body with `## Summary`; place no introduction above it. Include `## UI Changes` only for visual changes. Always include `## Changes` and `## Testing`, then finish with the generation credit.

### Summary

In a short paragraph, explain the user's problem first, then the solution and resulting behavior. Use simple words, name the affected user or task, and describe only outcomes supported by the diff. Keep implementation details in `Changes` when they help reviewers.

Hypothetical ZeDone example:

> Card descriptions disappeared after reopening a workspace, so users could not rely on ZeDone to keep their notes. Workspaces now save and restore descriptions along with card titles, so users can pick up where they left off.

### UI Changes (conditional)

Include this section only when the PR changes the visible UI. Match the evidence to the change:

- Static visual changes: include exactly one image before the change and one image after it.
- Dynamic visual changes, such as animations or transitions: include exactly one short video before the change and one short video after it. Each video's actual encoded resolution must be **1920 x 1080 pixels**, and all relevant text must be readable during playback. Verify the saved videos; setting a browser viewport alone does not establish the recording resolution. Use framing and scale that keep text legible; enlarging an unreadable recording is insufficient.
- If the PR contains both distinct static and dynamic visual changes, include both pairs.

Label each item `Before` or `After` and briefly identify the demonstrated change. Use the same representative data, scenario, viewport, theme, and framing within each pair so the comparison is meaningful. For videos, show the same short interaction from start to finish.

Capture real behavior from the PR's base and final versions. Capture the baseline before editing when practical; otherwise use an isolated checkout without disturbing the user's work. Do not fabricate screenshots, recordings, or claims about observed behavior.

Keep all PR images and videos outside the repository, including intermediate captures and converted files. Use an operating-system temporary directory or an external artifact location. Upload the final media as PR-host attachments or to an authorized external artifact service, and reference the hosted media in the body. Do not add media to Git, a repository asset directory, or another repository branch. Local filesystem paths are not usable PR attachments.

If capture or upload is unavailable, finish the reviewable text and report the specific missing evidence. Do not publish a visual PR with fabricated evidence, broken links, or media placeholders; resume publication once the required media is available.

### Changes

Use a bulleted list of concrete changes. Start each bullet with an imperative verb, such as `Save`, `Restore`, `Share`, or `Preserve`. Include implementation details only when they help a reviewer assess the change. Do not repeat the summary or list every edited file.

Hypothetical ZeDone example:

```markdown
- Save card descriptions together with titles in workspace data.
- Restore descriptions when users reopen a workspace.
- Cover saving and reopening cards with a regression test.
```

### Testing

Describe all checks actually performed before creating the PR, including automated checks, builds, manual scenarios, and media inspection when applicable. Give the command or procedure, its result, and the relevant platform or environment. Distinguish passing checks from failures and blocked checks. Never imply a check passed merely because it was planned or a test file exists.

Record relevant checks that were not run and explain why. If no checks were run, say so and give the reason. Report only completed CI results as completed; label pending CI explicitly. Keep results aligned with the final code and rerun affected checks after further changes when needed.

For ZeDone behavior changes, account for the applicable project requirements: reversible actions, equivalent human and agent access through the shared API, current documentation, Windows and Android parity, and a single authoritative source of data. State what was verified and any remaining gaps; do not claim platform parity from a single-platform check.

### Generation credit

Finish the body with one plain sentence identifying the actual model, application, and harness that generated the PR. Use the current runtime's supplied identity or authoritative configuration; do not copy the example as a fixed signature or guess missing details. When a detail is unavailable, state that it is unavailable in the credit.

Example for the matching runtime only:

```text
Generated by GPT-6.1 Sol in T3 Code using the Codex harness.
```

## Create or update the PR when requested

For a title or body request, return the requested text without changing remote state. When asked to create or update a PR, confirm the repository, base branch, head branch, and existing PR from available context, then use the prepared title, body, and required media. The skill itself does not authorize unrelated commits, pushes, merges, or external uploads.

Use structured tool arguments for multiline bodies. With `gh`, write the exact body to a temporary file outside the repository and pass it with `--body-file` so Markdown and newlines are preserved.

When T3 Code provides PR linking tools, register every PR created or worked on with `link_pull_request` immediately after creation or starting work. Before finishing, use `list_thread_pull_requests` and register any missing PR from this task.

Read back the resulting PR and verify the title, section order, rendered media, testing claims, and final credit. Report its URL and any unresolved limitation. If publication fails, preserve the prepared text and media, report the failure, and avoid creating duplicate PRs when retrying.
