# Viontin Framework

> **Experimental Project** — This is an experimental project under active development.

Core framework crates for the Viontin application platform.

## Crates

| Crate | Description |
|-------|-------------|
| `viontin-framework` | Core library — types, traits, runtime, infrastructure kernel |

> `viontin-tui` and `viontin-macros` have moved to their own repositories:
> - [viontin/tui](https://github.com/viontin/tui) — TUI toolkit (prompts, styling, validator)
> - [viontin/viontin](https://github.com/viontin/viontin) (crates/macros) — Proc-macros `#[domain]`, `#[domain_event]`

## Features

| Feature | Flag | Description |
|---------|------|-------------|
| Async server | `async` | Tokio-based async HTTP server |
| Domain-Driven Design | `domain` | DDD building blocks |
| SMTP mail | `smtp` | SMTP email transport via `lettre` |
| HTTP client | `http-client` | `ureq`-based HTTP client |
| AES encryption | `aes` | AES-256-GCM encryption |
| Graceful shutdown | `shutdown` | SIGTERM/SIGINT handling (default) |

## Recent Additions

- **JSON helpers**: `Response::json<T: Serialize>(&T) -> Self` and `Request::json<T: DeserializeOwned>(&self) -> Result<T>`
- **Bcrypt password hashing**: `BcryptHasher` — production-ready hasher (default dep)
- **CORS middleware**: `CorsMiddleware` — permissive, origin-restricted, preflight handling
- **SMTP transport**: `SmtpTransport` — TLS, auth, HTML+text mail (behind `smtp` feature)
- **Built-in middleware**: `PanicRecovery`, `RequestId`, `RateLimitMiddleware`

## Documentation

Framework documentation: https://github.com/viontin/docs

## License

MIT
