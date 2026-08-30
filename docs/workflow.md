# Workflow

## Branches

- **`develop`** — the default branch and primary integration point. All work
  lands here first.
- **`main`** — stable backup. Advances only when `develop` is deliberately
  merged into it (a manual decision, no schedule). Every such merge is tagged.
- **`feature/*`** / **`increment/N-*`** — cut from `develop`, one per unit of
  work, squash-merged back into `develop` via pull request, then deleted.

Release branches are not used yet. They will be introduced the first time
PeekOS has a real release to stabilise, and at that point they *will* merge
back into `develop` and `main`.

## Day to day

    git switch develop && git pull
    git switch -c increment/3-ci
    # ...work, commit...
    git push -u origin increment/3-ci
    gh pr create --base develop
    # CI runs, review the diff, then:
    gh pr merge --squash --delete-branch

## Promoting `develop` to `main`

When `develop` reaches a state worth preserving:

    gh pr create --base main --head develop --title "Merge develop -> main (v0.0.x)"
    gh pr merge --merge            # a merge commit, never a squash
    git switch main && git pull
    git tag -a v0.0.x -m "<what this snapshot is>"
    git push origin v0.0.x

Annotated tags `vMAJOR.MINOR.PATCH` on `main` are the permanent rollback
points. To work from one:

    git switch -c hotfix/<name> v0.0.x

## Protections

- `main` and `develop` both require a pull request — no direct pushes.
- Force-pushes and branch deletion are blocked on both.
- `develop` keeps a linear history (squash merges); `main` records `develop`
  merges as merge commits.
- CI must pass before a merge (wired up in increment 3).

## Commits

Imperative subject (~50 chars), blank line, body explaining *why*, wrapped at
~72 columns.
