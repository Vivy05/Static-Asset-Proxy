# Static Asset Platform

Rust workspace for an agent-facing static site deployment platform.

## Current Scope

Implemented so far:

- Rust workspace with app and crate boundaries
- tenant and user ownership model
- site registration and deployment use cases
- async repository seam
- PostgreSQL metadata repository skeleton
- Docker PostgreSQL setup and initial migration

The platform targets direct static site deployment and current-state metadata management without internal build, CI, or version rollback responsibilities.
