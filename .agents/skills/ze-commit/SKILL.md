---
name: ze-commit
description: Draft, review, and create Conventional Commits with plain English subjects that explain what becomes possible or what gets fixed. Use when preparing commit messages or committing changes.
---

# Ze Commit

Write commit messages in English. Make the subject understandable to a person who has not read the diff: explain the capability, corrected behavior, or practical benefit of the change. Put technical details in the optional body.

Follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/), with the writing rules below.

## Understand the change

- Read applicable repository instructions and commit validation settings. Use established types, scopes, and length limits where configured, while keeping the message in English and the subject plain and readable.
- Inspect the actual changes and their context. When committing, review the staged diff; it is the source of truth for what the commit contains. Do not describe unrelated unstaged work.
- Identify what someone can do after the change, what problem stops happening, or what becomes easier. For internal maintenance, describe the benefit to developers without inventing a product feature.
- Keep each commit focused on a coherent change. Separate unrelated changes when authorized; do not split a feature from the tests or documentation needed to understand it.

## Format the message

```text
type(scope): plain English outcome

- Optional explanation of why the change matters and how it works.

Optional footers.
```

The scope, body, and footers are optional. Use a lowercase type:

- `feat` for a new capability.
- `fix` for corrected behavior.
- `perf` for improved speed or resource use.
- `refactor` for restructuring without changing behavior.
- `docs`, `test`, `build`, `ci`, or `chore` for their respective kinds of work, following repository conventions.

Choose a short scope that names the affected area when it helps the reader. Omit it when it adds no useful information. Do not label internal maintenance as a feature just to make the subject sound more useful.

Mark incompatible changes with `!` immediately before the colon, a `BREAKING CHANGE:` footer, or both. Explain what becomes incompatible and how affected users can adapt. If using `!` without that footer, make the breaking change clear in the subject.

## Write the subject

- Describe the result, using simple words and an imperative verb such as `let`, `keep`, `restore`, `prevent`, or `reduce`.
- Name the concrete capability or corrected behavior. Avoid vague subjects such as `fix bugs`, `update logic`, or `improve handling`.
- Keep class names, function names, file paths, algorithms, and implementation mechanisms out of the description. A scope may retain an established technical area name such as `runtime`.
- Keep the subject concise without losing its meaning. Do not impose an arbitrary length limit when the repository has none. Omit the final period.
- Describe only what the change supports. Do not promise faster performance, broader platform support, or new capabilities without evidence.

Examples of weak subjects and better alternatives, assuming the diff supports each result:

- Weak: `feat(runtime): implement runtime export`
  Better: `feat(runtime): let authors export runtime for remote hosting`
- Weak: `fix(cards): fix serialization`
  Better: `fix(cards): keep card descriptions after reopening a workspace`
- Weak: `perf(board): memoize board selectors`
  Better: `perf(board): open large workspaces faster`
- Weak: `refactor(storage): extract persistence adapter`
  Better: `refactor(storage): make workspace saving easier to maintain`

## Add details when useful

Separate the body from the subject with a blank line. Write the body as a bulleted list using `- `, with one point per item. Use the body for motivation, previous and new behavior, implementation details, compatibility notes, or relevant verification results. Keep it in English and avoid repeating the subject. Omit the body when the subject is enough.

For example:

```text
fix(cards): keep card descriptions after reopening a workspace

- Include the description in the saved card data and restore it when loading
  the workspace. Previously, only the title survived a restart.
- Add a regression test for saving and reopening a card with a description.
```

Separate footers from preceding text with a blank line. Preserve required trailer syntax and include issue references or verification claims only when supported by the task and actual work.

## Create the commit when requested

For a message-only request, return the message without changing Git state. When asked to commit, include only the intended files or hunks, review the final staged diff, and create the commit with the prepared message. Preserve unrelated work. Do not amend existing commits, rewrite history, or push unless authorized.

Before finishing, confirm that the message matches the committed diff and that the commit succeeded. Report the commit identifier and subject. If a hook fails, address failures within the task's scope and retry; do not bypass hooks or claim success while the commit is still blocked.
