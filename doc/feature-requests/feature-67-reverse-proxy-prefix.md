# Feature 67: Reverse Proxy Subdirectory Support

Read `X-Forwarded-Prefix` header to discover the external base path when behind a reverse proxy that routes through a subdirectory (e.g., `https://example.com/webrpg/`). All absolute paths must be prefixed so the app works correctly whether served at `/` or at a subdirectory.

**Affected absolute paths:**

- Shell HTML head: `/pkg/webrpg.css`, `/favicon.ico`
- Leptos router: `/`, `/login`, `/signup`, `/sessions`, `/game/:id`
- WebSocket endpoint: `/api/ws`
- API endpoints: `/api/media/upload`, `/api/media/{hash}`
- Client-side navigation: all `<a href="...">` links
- Server functions: Leptos RPC endpoint URLs

**Standard headers:**

- `X-Forwarded-Prefix` (nginx, traefik, HAProxy) — the path prefix stripped by the proxy
- Should also support a static `BASE_PATH` env var fallback for non-proxy deployments or when the header isn't available

**Approach considerations:**

- Axum middleware could read the header and inject it into request extensions
- The shell function can read it when rendering HTML to prefix static assets
- A Leptos context (e.g., `BasePath`) could make the prefix available to all components
- The WebSocket URL is constructed client-side and needs the prefix too
- Leptos `<Stylesheet>` and `<Link>` components may need manual prefixing

## Dependencies

None.

## Status: Not Started

## Plan

(none yet)

## Findings

- `X-Forwarded-Proto` is already read in `src/main.rs:130` for HTTPS redirect
- All routes in `src/main.rs` use absolute paths (`/api/ws`, `/api/media/upload`, etc.)
- `src/app.rs` has hardcoded absolute paths: `/pkg/webrpg.css`, `/favicon.ico`
- Router routes use `StaticSegment("login")` etc. — would need a base prefix
- The WebSocket URL is likely constructed in client-side JS/WASM code
- `Cargo.toml` has `site-root` and `site-addr` but no `site-prefix` option in Leptos 0.8
