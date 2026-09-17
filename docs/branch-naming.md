# Branch naming

This document fixes the branch naming convention. The prefix mirrors the merge commit type, and the identifier points at the tracked unit: a grove node (work item or goal) or a release version.

## Convention

| Branch | Purpose |
| --- | --- |
| `release/vX.Y.Z` | Release line work, including the post-release transparency commit-back. |
| `feat/(W\|G)-NNN` | Features carried by a work item or a goal. |
| `fix/(W\|G)-NNN` | Bug fixes carried by a work item or a goal. |
| `refactor/(W\|G)-NNN` | Behavior-preserving refactors. |
| `docs/(W\|G)-NNN` | Documentation work carried by a node. |
| `chore/(W\|G)-NNN` | Tracked chores, maintenance with a node. |
| `chore/version-vX.Y.Z` | Version bumps preparing a release. |
| `dependabot/*` | Dependabot's own format, never renamed. |
