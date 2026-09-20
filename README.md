# Static Asset Platform

Rust workspace for an agent-facing static site deployment platform.

## Current Scope

Implemented so far:

- site registration and artifact upload use cases
- artifact activation flow for selecting the active site version
- HTTP control plane endpoints for site registration, artifact upload, activation, and runtime inspection
- gateway static file serving for the active site version

## Environment Variables

- `MCP_SERVER_ADDR`
- `GATEWAY_ADDR`
- `STATIC_ROOT`

Defaults:

- `MCP_SERVER_ADDR=127.0.0.1:3000`
- `GATEWAY_ADDR=127.0.0.1:4000`
- `STATIC_ROOT=./data/static`

## Control Plane Endpoints

- `POST /sites`
- `POST /artifacts`
- `POST /sites/{site_id}/activate`
- `GET /sites/{site_id}/runtime`

## Gateway Endpoints

- `GET /sites/{site_id}`
- `GET /sites/{site_id}/*asset_path`

The gateway resolves files from the active artifact version:

```text
{STATIC_ROOT}/{storage_key}
```
