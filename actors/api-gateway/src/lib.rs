//! API Gateway actor — compiled to wasm32-wasip2 via `wash build`.
//!
//! `cargo check --target wasm32-wasip2` does NOT work for actor crates without
//! vendored WIT deps. Use `wash build --project-path actors/api-gateway` instead.
//! The CI `build-actors` job handles this correctly.
//!
//! Responsibilities (implemented once WIT bindings are generated):
//!   - Serve bundled React 19 UI assets at `/` and `/assets/*`
//!   - Handle OAuth callbacks; set `hostmgr_session` HttpOnly cookie (8h TTL)
//!   - Authenticate REST requests via cookie (UI) or `Authorization: Bearer hm_*` (API key / CLI)
//!   - Translate REST → NATS request-reply for all internal actors
//!
//! Routes:
//!   GET  /                              → serve React SPA shell
//!   GET  /assets/*                      → serve bundled UI static assets
//!   GET  /api/v1/health                 → liveness probe (no auth)
//!   GET  /api/v1/endpoints              → list endpoints
//!   GET  /api/v1/endpoints/:id          → get endpoint detail
//!   POST /api/v1/endpoints/:id/commands → dispatch command
//!   POST /api/v1/discovery/start        → start discovery run
//!   GET  /api/v1/discovery/status       → last run status
//!   GET  /auth/github/authorize         → redirect to GitHub OAuth
//!   GET  /auth/github/callback          → OAuth code exchange; set cookie; redirect /
//!   GET  /auth/google/authorize         → redirect to Google OAuth
//!   GET  /auth/google/callback          → OAuth code exchange; set cookie; redirect /
//!   POST /auth/logout                   → clear cookie; redirect /login
//!   POST /api/v1/agents/bootstrap       → exchange bootstrap token for NKey creds
