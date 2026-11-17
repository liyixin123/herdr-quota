# Issue tracker: GitHub

Issues and specs live in https://github.com/liyixin123/herdr-quota/issues.
Use the `gh` CLI from this repository.

## Conventions

- Before the first issue creation in a session, verify the target with
  `gh repo view --json nameWithOwner`. It must be `liyixin123/herdr-quota`.
- Create: `gh issue create --title "..." --body-file <file>`.
  For multiline bodies, write the exact Markdown to a temporary file.
- Read: `gh issue view <number> --json number,title,body,labels,comments`.
- List: `gh issue list --state open --json number,title,body,labels`.
  Add label and state filters as needed.
- Comment: `gh issue comment <number> --body-file <file>`.
- Apply or remove labels:
  `gh issue edit <number> --add-label "..." --remove-label "..."`.
  Use the mapping in `triage-labels.md`.
- Close: post the explanation as a comment, then run
  `gh issue close <number>`. Post comments separately even when already closed.
- Link a sub-issue: `gh issue edit <parent> --add-sub-issue <child>`.
  If unavailable, put `Part of #<parent>` at the top of the child body
  and add the child to the parent's task list.
- Record dependencies:
  `gh issue edit <child> --add-blocked-by <blocker>`.
  If unavailable, put `Blocked by: #<number>, #<number>` at the top
  of the child body. An issue is unblocked when every blocker is closed.
- Count open native blockers:
  `gh api repos/liyixin123/herdr-quota/issues/<number> --jq .issue_dependencies_summary.blocked_by`.

## Pull requests as a triage surface

PRs as a request surface: no.

## Skill operations

When a skill says "publish to the issue tracker", create a GitHub issue.
When it says "fetch the relevant ticket", read the referenced GitHub issue.

## Wayfinding operations

- Map: one issue labelled `wayfinder:map`, containing Notes,
  Decisions-so-far, and Fog.
- Child ticket: a sub-issue of the map, labelled `wayfinder:<type>`,
  where type is research, prototype, grilling, or task.
  Create these labels when first needed.
- Frontier: inspect the map's open children in map order.
  The first child with no open blocker and no assignee is next.
- Claim: assign the ticket to the driving developer before working.
  Use `gh issue edit <number> --add-assignee @me` when appropriate.
- Resolve: comment with the answer, close the child, then append
  a concise finding and a link to the map's Decisions-so-far.
