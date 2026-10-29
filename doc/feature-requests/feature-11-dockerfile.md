# Feature 11: Dockerfile

Create a Dockerfile for running the server (Alpine Linux based or distroless
container). Publish releases to Docker Hub.

## Status: In Progress

Containerization is done. Registry publishing (Docker Hub) is not set up yet.

### Done
- Multi-stage [Dockerfile](../../Dockerfile): a Debian builder cross-compiles the
  server to a static musl binary and builds the WASM client (Debian so
  cargo-leptos's glibc wasm-bindgen/wasm-opt run), then an Alpine runtime image
  copies the binary, `site/`, `help/`, and `modules/`.
- Migrations are embedded in the binary (`diesel_migrations`) and run at startup,
  so a fresh database volume is brought up to schema without a manual
  `diesel migration run`. This also applies to the non-Docker deploy path.
- Data (`database.db` and uploaded media) lives under `/app/data`, exposed as a
  volume, so it survives image upgrades.
- [docker-compose.yml](../../docker-compose.yml) for self-hosting with a named
  volume; `SECRET_KEY` is required (no default) since it signs auth cookies.
- [.dockerignore](../../.dockerignore) keeps `target/`, secrets, local state, and
  non-redistributable adventure art out of the build context.
- DEV.md "Running with Docker" section.

### Remaining
- GitHub Actions workflow to build and push a tagged image to Docker Hub on
  release (needs `DOCKERHUB_USERNAME` / `DOCKERHUB_TOKEN` secrets and a repo
  name). Deferred by request.

## Findings

- SQLite is compiled in via `libsqlite3-sys` `bundled`, and TLS uses rustls, so
  the builder needs only `musl-tools` (plus a C toolchain for the bundled
  sqlite and ring), no system sqlite or OpenSSL.
- cargo-leptos downloads wasm-bindgen and wasm-opt as glibc binaries, which do
  not run on a musl host, so the builder stage must be glibc (Debian) even
  though the server binary targets musl.
- `SECRET_KEY` must be stable across restarts; a regenerated key invalidates
  existing auth cookies.
