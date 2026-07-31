# wild:secrets — Design + status

Status: **Shipped.** The critical path (P6.1, P6.3–P6.5, P6.8) is
implemented and tested — `wild:secrets@0.1.0` WIT, the
`SecretsBackend` chain, the full `wild secret` CLI, ACL
enforcement, audit events, and the encrypted-file backend all
exist. The lean-CLI split (ADR-0036) moved the engine into a
standalone `crates/runtime/wild-secrets` crate; the host plugin
(`wild_secrets_plugin.rs`) is now the thin auth adapter that links
it. ADR-0059 added a required-secrets activation gate on top (see
§ "Required-secrets gate" below). Still pending: the
`wild-secrets-component` helper crate (P6.7, DX-only). This doc
keeps the original design rationale and marks where the shipped
code diverges from it.

Phase 7 (Tier-2 component adapters) is the immediate consumer and
is **no longer blocked** — its secrets precondition is met. See
`wild/docs/llm-adapters.md` "Adapter delivery — two tiers" for the
cross-cutting context.

## Why

Components today have three ways to get an API key, all bad:

1. **Hardcoded in the WASM** — leaks via `wild component add` OCI
   image inspection. Non-starter for any real key.
2. **Env var passthrough via `wasi:cli/environment`** — works for
   Tier-1 native adapters that we audit (`OPENROUTER_API_KEY`),
   but means the wild process has every key in its env. A 3rd-
   party adapter (Phase 7) would inherit them all, which is
   exactly what we want to avoid.
3. **`wasi:keyvalue` against a file** — too generic. The component
   could enumerate keys, observe missing ones, etc. Wrong shape.

What we need: a **scoped secrets surface** where a component
declares which named secrets it wants, the user explicitly grants
each one, and the host enforces both at every read. The user's
secrets stay in the OS keychain, never in process memory longer
than one read, and every access lands an audit event.

This is the precondition for Phase 7 (Tier-2 OCI adapters).
Without it, the only way to ship a 3rd-party adapter is to
hardcode the key — meaning we can't ship 3rd-party adapters at
all.

## Scope

In scope:
- `wild:secrets@0.1.0` WIT contract — `get(name) -> result<string,
  secret-error>`. Read-only from the component side.
- `SecretsBackend` Rust trait + backend implementations
  (one cross-platform `KeychainBackend` via the `keyring` crate,
  `EnvBackend`, `EncryptedFileBackend`).
- `wild secret {add,list,show,remove,rotate}` CLI for managing
  values.
- `wild secret {grant,revoke,grants}` CLI for managing the
  component → secret ACL.
- `~/.wild/secret-grants.json` ACL persistence.
- Host-side enforcement: every `get(name)` checks the ACL before
  hitting the backend.
- Audit events on `wild.{tribe}.system.secrets.access` —
  component_id + alias + result, **never the value**.

Out of scope (separate phases / docs):
- **Per-tribe scoped secrets** — Phase 6.x extension. Today
  every secret is wild-installation-wide. A multi-tenant deploy
  may want org-scoped namespaces; the WIT surface stays
  compatible because the host resolves `name` against the calling
  component's org context.
- **Secret-rotation events on a NATS subject** — components today
  re-read on every call. A push-based "your secret was rotated"
  invalidation channel is a Phase 6.x nice-to-have for components
  that legitimately cache (none planned).
- **Hardware-backed keys (TPM, Secure Enclave passkeys)** — out
  of scope; the OS keychain backend already uses these
  transitively on Mac/Win.
- **Secret sharing across wild installations** (e.g. a team-
  shared keystore) — Phase 8+, probably an external secrets-
  manager bridge (`vault://`, `aws-secrets-manager://`).

## Non-goals

- **Component-side write/delete/list** — read-only is the whole
  trust model. Writes go through the CLI exclusively.
- **Implicit grants** — a component does NOT auto-receive grants
  for "secrets it asks for". The user MUST `wild secret grant`
  explicitly. `wild adapter add` proposes grants interactively
  but doesn't apply them silently.
- **Anonymous secrets / capability tokens** — every secret has a
  user-chosen name. No `wild secret get-temp-token` or similar.
- **Per-call secret injection from the trigger payload** — would
  let any caller override what the component reads. Secrets come
  from the keychain, period.

## Credentials that end up in a chat

A secret can reach the host without ever going near this service:
you paste an API key into a chat to ask what it is, a tool error
echoes an `Authorization` header, or the model quotes either back
inside its own reasoning. Every chat turn is persisted twice — the
Denkverlauf trace under `tribes/<slug>/chat-traces/` and the
session transcript in the `domain-elder-chat` KV — so anything in
that text used to come to rest on disk in the clear.

Both writers now run a redaction pass
(`wild_core::redact`) at the LAST point before persistence, so no
caller can skip it, and both the operator and the customer plane
inherit it. What it removes:

| Shape | Example |
|---|---|
| Vendor-prefixed keys | `sk-…` · `ghp_…` · `AKIA…` · `xoxb-…` · `AIza…` · `glpat-…` |
| JWTs | `eyJ….….…` |
| PEM private-key blocks | `-----BEGIN … PRIVATE KEY-----` |
| Bearer tokens | `Bearer <token>` |
| Labelled assignments | `api_key=…` · `Kennwort=…` · `token:…` |

Each becomes `[redacted:<kind>]`, so the record stays readable and
the operator can see that something was removed.

> **What this does NOT do.** A password made of ordinary words is
> not detectable — it looks exactly like the prose around it, and a
> rule loose enough to catch it would redact the sentence it sits
> in. Redaction is defence-in-depth over the shapes that are
> directly exploitable by whoever reads the file, never a guarantee
> that a persisted record is secret-free. The right move for a
> secret is still to put it here, with `wild secret add`, and never
> into a chat.

## The WIT contract

```wit
// wit/secrets/secrets.wit

package wild:secrets@0.1.0;

interface store {
    /// Read a named secret. Component receives the value as a
    /// transient string — no implementation hint about persistent
    /// storage on the component side. Components MUST NOT cache
    /// the value across calls; rotation works because every read
    /// hits the host, the host hits the keychain backend.
    ///
    /// `name` is the component-side alias (e.g. "api-key", "org-id"),
    /// NOT the keychain entry name. The host resolves the alias
    /// through the per-component map declared in the adapter's
    /// `~/.wild/llm-adapters.yaml` entry (or for non-adapter
    /// components, in
    /// `~/.wild/profiles/<active>/system/plugin-cache/<slug>.json`).
    get: func(name: string) -> result<string, secret-error>;

    variant secret-error {
        /// The aliased keychain entry doesn't exist. Either the
        /// user removed it via `wild secret remove`, or never ran
        /// `wild secret add` for that name in the first place.
        not-found,
        /// The component asked for an alias that isn't granted to
        /// it in `~/.wild/secret-grants.json`. Logged as a
        /// security event before the error returns.
        access-denied,
        /// The keychain backend itself failed (locked keychain,
        /// daemon down, decrypt-error on EncryptedFileBackend).
        /// Carries the raw cause string.
        backend-unavailable(string),
    }
}

world secrets-host {
    export store;
}
```

**Why `get` only, not `try-get` + `must-get`:** every error case
is structurally distinct (`secret-error`), so a typed `result` +
match is enough. Adding sugar variants gives the component three
ways to do the same thing.

**Why no `list`:** even listing names is information leakage. A
malicious component asking "do you have a `slack-token`?" learns
something about the user's setup. Tighten down.

## Storage backends — `SecretsBackend` trait + chained resolver

> **Shipped layout (ADR-0036 lean split).** The trait, backends,
> resolver, grant store, audit sink and side-index all live in the
> standalone `crates/runtime/wild-secrets` crate (modules
> `backends.rs`, `grants.rs`, `grants_check.rs`, `audit.rs`,
> `index.rs`) so the lean CLI can run every `wild secret` command
> without linking the Wasm host. `wild_secrets_plugin.rs` in
> `wild-host` re-exports from that crate and adds only the
> WIT-boundary `Host::get` wiring. The trait is **synchronous** in the
> shipped code (keychain reads are fast, ~ms) and exposes
> `id` / `writable` / `get` / `put` / `remove`, mirroring the
> `ChatBackend` / `TasksBackend` shape. `get` returning `Ok(None)`
> means "this backend doesn't have it" (the resolver continues to the
> next backend); `Err` is a backend-side failure.

### Available backends

**Shipped: three backend types, not five.** The original design
listed a separate Rust type per OS keystore. The implementation
collapsed those into **one** cross-platform `KeychainBackend`
backed by the `keyring` v3 crate, which dispatches per-OS via
cargo features — `apple-native` (macOS), `sync-secret-service` +
`crypto-rust` (Linux), `windows-native` (Windows). So the keystore
is one Rust type, three platform backends underneath. Backend ids
recognised by the resolver are exactly `keychain` / `env` / `file`
(`backends.rs`). `vault` / `1password` appear as *reserved* ids in
`docs/config-vars.md` but are **not implemented** — an unknown id
in the order is dropped with a WARN.

| Backend (`id`) | Storage | Writable | Notes |
|---|---|---|---|
| `KeychainBackend` (`keychain`) | OS keychain via `keyring` v3, scoped service `"wild"` | yes | macOS Keychain / Linux Secret Service (D-Bus) / Windows Credential Manager, picked by cargo feature. On macOS the first read shows a TouchID prompt; subsequent reads ~ms. In Keychain Access each entry displays as **`Wild - <name>`** (label stamped on write; the `(service="wild", account=name)` lookup key is unchanged, and pre-existing entries pick the label up on their next write). The OS keychain has no enumerate API → `wild secret list` reads a side-index (see below). |
| `EnvBackend` (`env`) | none — env-passthrough only | **no** | Always available. Maps `secret-name` → env var (uppercased, dashes→underscores; `my-key` → `MY_KEY`, pinned by `EnvBackend::env_var_name`). Read-only by design. |
| `EncryptedFileBackend` (`file`) | `$WILD_HOME/system/secrets.enc` (override `WILD_SECRETS_FILE_PATH`), AES-256-GCM, master key from `WILD_SECRETS_KEY` (64-hex), atomic writes, mode 0600 | yes | Only joins the chain when `file` is in the order **and** `WILD_SECRETS_KEY` is set — otherwise dropped from the chain with a WARN at boot. "Not for production keys, only for headless / CI smoke" doc-pin. |

> **Not the same store as OAuth refresh tokens.** ADR-0143's OAuth
> broker keeps its rotating **refresh tokens** in a *separate*
> host-managed store — `$WILD_HOME/system/oauth-tokens.enc`
> (`wild_oauth::TokenStore`) — which **reuses the `EncryptedFileBackend`
> mechanism and the same `WILD_SECRETS_KEY` master key** but is
> deliberately NOT part of the resolver chain and has no guest read
> path. Machine-managed/rotating credentials are kept apart from the
> operator-provisioned, immutable-from-guest secrets above so the
> `store::get(alias)` contract stays clean (a guest can never `get` a
> refresh token, and a rotation never looks like an operator edit). The
> only `wild:secrets` value OAuth stores here is the static
> `client_secret`.

### The chained resolver

A `ChainedResolver` wraps `Vec<Arc<dyn SecretsBackend>>` and is what
the wild:secrets plugin actually dispatches against. `get(name)`
walks the chain in declared order and returns the **first hit**:

```rust
fn get(&self, name: &str) -> Result<Option<String>, BackendError> {
    for backend in &self.chain {
        match backend.get(name) {
            Ok(Some(v)) => return Ok(Some(v)),
            Ok(None) => continue,                  // try next
            Err(e) => {
                tracing::warn!(backend = %backend.id(), error = %e,
                    "secrets: backend errored mid-chain; trying next");
                continue;
            }
        }
    }
    Ok(None)
}
```

The walk is **synchronous and allowed to block** — `KeychainBackend`
calls the OS keychain through `keyring`, which on macOS is a blocking
FFI call into the Security framework that waits indefinitely when the OS
wants a user confirmation a daemon cannot give.

That is why the host's `Host::get` (`wild-secrets-plugin`) binds its WIT
import **async** and runs this walk on `tokio::task::spawn_blocking`.
The WIT is unchanged — the component still calls a plain `get: func` —
but the guest's fiber now suspends across the walk instead of pinning a
runtime worker inside a single `poll`.

**It is not an optimisation, it is the only thing that keeps a guest call
bounded.** A blocking call in a *sync* host function blocks the guest
inside one `poll`, and every bound the host has lives downstream of a
poll that returns: `tokio::time::timeout` polls the future it wraps
before its own timer, so the timer is never polled; and the wasm epoch
fence cannot fire either, because a guest parked in a host call is not
executing wasm. Issue #3284 was exactly this — an LLM turn hung for 65
minutes with no `audit.egress` row, because the adapter reads its API key
*here*, before it opens the socket. **Any new host function that can
block must do the same.**

### The 20-second backend fence

The walk is bounded (`SECRET_GET_TIMEOUT`, 20 s). A healthy read is
milliseconds; the one way it takes real time is a keychain waiting on a
confirmation dialog no daemon can answer. On expiry the component gets
`backend-unavailable` carrying the cause **and the way out**:

> `no secrets backend answered within 20s — if the OS keychain is in the
> chain it may be waiting on a confirmation dialog that a daemon cannot
> answer. Unlock the login keychain, or move this secret to a backend that
> never prompts (`wild secret backend`, e.g. file or env)`

That message is the point. Without it a wedged keychain surfaced 240 s
later as the LLM shim's generic "the call ran out of budget" — true, and
useless to the operator (ADR-0116: faults surface as operator-legible
outcomes, not as a timeout somewhere up the stack).

The fence is only *reachable* because `get` is async and the walk is on
the blocking pool — around a sync host call, a timeout can never fire.
It releases the **guest**, not the thread: a blocking FFI call has no
cancellation point, so the pool thread stays parked until the OS returns.
The daemon stays alive and legible; the thread is not reclaimed.

`put(name, value)` writes to the **first writable backend** in the
chain:

```rust
async fn put(&self, name: &str, value: &str) -> Result<(), BackendError> {
    for backend in &self.chain {
        if backend.writable() {
            return backend.put(name, value).await;
        }
    }
    Err(BackendError::NoWritableBackend)
}
```

`remove(name)` removes from **every backend that has it** —
otherwise a "shadowed" entry in a later backend would be silently
exposed after the user thought they deleted the secret.

### Default chain order

```
Keychain (OS) → Env
```

**Shipped default is two slots, not three** (`build_chain_from_order`,
`WILD_SECRETS_BACKEND_ORDER` default `keychain,env`). The encrypted-file
backend is **opt-in** — it only joins when `file` is named in the order
*and* `WILD_SECRETS_KEY` is set. The original three-slot
`keychain → file → env` order still holds whenever the file backend is
configured (the WIT header comment documents that full conceptual chain).

Reasoning:
- Keychain first — strongest storage, OS-managed, what users
  expect for "I added a secret". One `KeychainBackend` covers all
  three platforms via `keyring`.
- EncryptedFile (when enabled) sits between keychain and env —
  covers headless setups; lets a power user put deployment-specific
  keys here while OS-keychain holds personal ones.
- Env last — for the "this is already in CI's env, just pass it
  through" case. Last so a same-named keychain entry wins (useful
  when the user wants to override the CI-injected key locally).

### Override

`WILD_SECRETS_BACKEND_ORDER=<csv>` env var swaps the order, drops
backends, or pins a single one for ops control:

```bash
# CI-only: env-passthrough, nothing else
WILD_SECRETS_BACKEND_ORDER=env

# Headless server: file first, env as last-resort
WILD_SECRETS_BACKEND_ORDER=file,env

# Local dev: keep default but skip the file backend
WILD_SECRETS_BACKEND_ORDER=keychain,env
```

Unknown ids in the list are ignored with a WARN-log.

### Mixing — the win

The chained model lets a user do this without touching code or
config:

```bash
# Personal API key — strongest storage, OS-keychain
wild secret add my-openrouter-key
> Enter value: sk-or-v1-...

# Team-shared bot token — comes from the deploy environment via env
export SLACK_BOT_TOKEN=xoxb-...

# Both names appear in `wild secret list` — `wild secret list`
# walks the chain too; the source-backend column shows where each
# value lives.
wild secret list
NAME                  BACKEND
my-openrouter-key     keychain
slack-bot-token       env
```

Components don't see the difference — `secrets::get(alias)` works
the same way regardless of which backend held the value.

## Naming + aliasing

Two-name model. Component-side names are stable across deploys
(the component code references `"api-key"` forever); user-side
names are whatever the user picked when `wild secret add`-ing
(`"my-openrouter-personal-key"` vs `"team-openrouter-shared"`).
The mapping is per-component config:

```yaml
# ~/.wild/llm-adapters.yaml — adapter case
- id: awesome
  kind: component
  oci_image: ghcr.io/3rd/awesome:0.1.0
  secrets:
    api-key: my-openrouter-personal-key   # alias : keychain-name
    org-id:  my-openrouter-org

# ~/.wild/profiles/<active>/system/plugin-cache/<slug>.json — non-adapter custom component
{
  "id": "my-slack-poster",
  "oci_image": "...",
  "secrets": {
    "slack-token": "team-slack-bot-token"
  }
}
```

At component instantiation, the host builds a per-store
`SecretsCtx { component_id, alias_to_keychain_name: HashMap<String, String> }`.
Every `secrets::get(alias)` call:
1. Looks up `alias` in the alias map → keychain name.
2. Checks `~/.wild/secret-grants.json` — does
   `component_id` have a grant on this keychain name?
3. Reads the backend.
4. Publishes the audit event.
5. Returns the value (or the typed error variant).

## ACL — the grants file

**Shipped path is profile-aware** (`GrantStore::default_path`):
`~/.wild/profiles/<active>/system/secret-grants.json`, with a
legacy fallback to `~/.wild/secret-grants.json` when no profile is
active (tests/tooling). The `wild-secrets` `GrantStore` does atomic
writes via `tempfile::persist`. Source of truth for "who can read
what". Hand-editable but the CLI is the safe path:

```json
{
  "grants": [
    {
      "component_id": "awesome",
      "secret_name": "my-openrouter-personal-key",
      "granted_at_ms": 1714234567890,
      "granted_via": "wild secret grant awesome my-openrouter-personal-key"
    },
    {
      "component_id": "awesome",
      "secret_name": "my-openrouter-org",
      "granted_at_ms": 1714234567891,
      "granted_via": "wild adapter add ghcr.io/... --secret org-id=my-openrouter-org"
    }
  ]
}
```

Why a separate file from `llm-adapters.yaml` / `secrets`
backend storage:
- yaml is shareable (no values, just names + aliases) — safe to
  commit to a repo. Grants ARE deployment-specific.
- Backend storage has the values; this file has the ACL. Two
  concerns, two files.

`wild secret grants` renders this as a table. `wild secret revoke`
filters it. Bulk-revoke per component (`wild secret revoke awesome`)
removes every row matching that component_id.

## CLI surface

### Value management

```bash
wild secret add <name>
  # interactive value-prompt (stdin, no echo). NEVER as argv —
  # process listings would leak it. `--from-stdin` flag for piping
  # in CI: `cat secret.txt | wild secret add foo --from-stdin`.

wild secret list
  # names only, NEVER values. Sorted, with the source-backend
  # column showing where each value lives (keychain/file/env)
  # so a user can spot mis-configured chains. With audit events
  # enabled, also shows last-used hint.

wild secret show <name> --confirm
  # shows the value. Requires --confirm to avoid accidental shell-
  # history leak. Emits a special audit event "human-show".
  # Optionally prints to a tmp file the user opens (so it's not
  # in scrollback): wild secret show foo --to-clipboard.

wild secret remove <name>
  # warns if any grant exists for this name + lists them. --force
  # removes anyway, cascading the grants.

wild secret rotate <name>
  # interactive new-value prompt. Atomic add-new + drop-old. All
  # existing grants stay attached to the same name.
```

### ACL management

> **P7 update**: `wild secret grant` is now a deprecated alias for
> `wild plugin grant <slug> --secret <name>`. The legacy verb still
> works for one release with a stderr deprecation hint; new scripts
> should use the unified plugin verb. See
> plugin-cli.md for the full surface.
> Post-P7 the grants file lives at
> `~/.wild/profiles/<active>/system/secret-grants.json` (per-profile),
> with a legacy fallback to `~/.wild/secret-grants.json` when no
> profile is active.

```bash
# Post-P7 canonical form:
wild plugin grant <slug> --secret <name> [--force]
  # routes to ~/.wild/profiles/<active>/system/secret-grants.json.

# Legacy (deprecated, removed in next release):
wild secret grant <component-id> <secret-name>
  # adds an entry to ~/.wild/secret-grants.json.

wild secret revoke <component-id> [<secret-name>]
  # removes one grant, or all grants for that component when name
  # is omitted.

wild secret grants
  # renders the table. Filters: --component <id>, --secret <name>.
```

### Discovery

```bash
wild secret backend
  # shows the active resolver chain in order, each backend's
  # writability + health probe. Renders WILD_SECRETS_BACKEND_ORDER
  # if set so the user can audit overrides.

wild doctor
  # already exists; gains a "secrets" row showing
  # backend-name + count of stored secrets + count of grants.
```

## Component-side helper

> **Status: not yet built (P6.7).** Components today call the raw
> `wild:secrets/store::get` bindgen output directly. The wrapper
> crate below is the planned DX sugar — it does not exist yet.

Components shouldn't have to handle the WIT bindgen output by
hand for every `get`. Phase 6.7 ships `wild-secrets-component`
as a small wrapper crate (cargo-component compatible):

```rust
// In a component:
use wild_secrets_component::{secrets, SecretError};

let api_key = match secrets::get("api-key") {
    Ok(v) => v,
    Err(SecretError::NotFound) => {
        // Tell the user how to fix it
        return Err("api-key secret missing. Run: wild secret add ...".into());
    }
    Err(SecretError::AccessDenied) => {
        return Err("api-key not granted to this component. Run: wild secret grant ...".into());
    }
    Err(SecretError::BackendUnavailable(reason)) => {
        return Err(format!("secrets backend down: {reason}"));
    }
};
```

Same pattern for Go / JS / Python WASI-P2 toolchains down the
line; helper crate is Rust-only at first.

## Audit events

Every `secrets::get` call publishes a JSON event on
`wild.{tribe}.system.secrets.access`:

```json
{
  "component_id": "awesome",
  "alias": "api-key",
  "keychain_name": "my-openrouter-personal-key",
  "result": "ok",
  "timestamp_ms": 1714234567890,
  "tribe_id": "acme-sales"
}
```

`result` is `"ok"`, `"not-found"`, `"denied"`, or
`"backend-error: <kind>"`. **The value is NEVER in the event.**
TUI / observability dashboards subscribe to this subject for
real-time visibility; long-term retention via JetStream is opt-
in (the org's policy decision).

`wild secret show --confirm` emits a distinct event with
`result: "human-show"` so an audit reviewer can spot human
inspection separately from component reads.

## Bootstrap question — pre-load or lazy?

**Lazy** (every `get` call hits the backend). Empirically:
- macOS Keychain reads: ~1–3ms per call after the first
  TouchID prompt.
- Linux Secret Service: ~1ms per call (D-Bus round-trip).
- Compared to a 1–60s LLM call, the keychain hit is noise.

**Pre-load** would mean the host reads every secret at component
instantiation and injects values into the component-store; values
sit in process memory for component-life. That's a longer
attack surface for a memory-dump exploit.

**Decision: lazy.** Doc-pin the rule, reject component caching
("re-read every call"). If a measured component cycle shows
keychain reads dominate latency, revisit.

## Rotation semantics

`wild secret rotate <name>` atomically replaces the value in the
backend. Long-running cycles see the new value on their next
`get` — no signal, no event, just "next read returns new value".

This is enough for the Tier-2 LLM-adapter use case: every chat
call reads its api-key, so within one request-response round
the new key takes effect. Components that build long-lived
clients (e.g. a database driver pinned at instantiate) would
need a different model — flagged as a Phase 6.x extension if a
real use case appears.

## Shipped extensions beyond the original design

Three things landed in code that the original design above didn't
name:

- **Secret side-index** (`wild-secrets/src/index.rs`). The OS
  keychain can't enumerate entries, so `wild secret add` also
  records `{name, added_at_ms, added_via}` into
  `~/.wild/profiles/<active>/system/secret-index.json`. That's
  what backs `wild secret list` — the index is a name catalogue
  only, never values. Reads observed at the host can also feed it
  (`record_observed_read`).

- **Required-secrets activation gate (ADR-0059).** A worker
  manifest can declare a `## Secrets` block; the tribe render
  projects those into the worker's `wasi:config`, and the activate
  gate (`grants_check.rs`) **refuses to start a component whose
  declared-required secrets aren't granted** — turning a runtime
  `access-denied` into an upfront, legible boot failure. This is
  the "Component declares required secrets at install time?" open
  question, answered: yes, via the manifest.

- **Pooled-component grants by slug.** A pooled adapter instance
  presents a `component_id` like `plugin/<slug>/instance`; the ACL
  check (`grant_allows`) resolves the grant against the bare
  `<slug>` so one grant covers every pooled instance.

The alias→keychain-name map also reaches the host through
`wasi:config` under the `secret-alias.` key prefix
(`SECRET_ALIAS_CONFIG_PREFIX`, projected by `extract_alias_map`),
in addition to the per-component config files described above.

## Phase 6 roadmap — status

| Step | Scope | Status |
|---|---|---|
| **P6.1** | `wild:secrets@0.1.0` WIT + `SecretsBackend` trait + `ChainedResolver` + KeychainBackend + EnvBackend (always-on read-only). | ✅ Shipped |
| **P6.2** | Linux Secret Service backend. **Folded into `KeychainBackend`** via the `keyring` crate's `sync-secret-service` feature — no separate Rust type. | ✅ Shipped |
| **P6.3** | `wild secret {add,list,show,remove,rotate,backend}` CLI. Interactive prompts, atomic writes, no-argv-values. `list` reads the side-index; `backend` renders the active chain. | ✅ Shipped |
| **P6.4** | ACL grants file + `wild secret {grant,revoke,grants}` CLI + host-side enforcement on every `get`. | ✅ Shipped |
| **P6.5** | Audit-event publishing on `wild.{tribe}.system.secrets.access` (shared `AuditSink`). | ✅ Shipped |
| **P6.6** | Windows Credential Manager backend. **Folded into `KeychainBackend`** via the `keyring` crate's `windows-native` feature. | ✅ Shipped |
| **P6.7** | `wild-secrets-component` helper crate + cargo-component example. | ⏳ Pending (DX-only) |
| **P6.8** | EncryptedFileBackend with master key from `WILD_SECRETS_KEY`. | ✅ Shipped |

Phase 7's secrets precondition (P6.1 + P6.3 + P6.4 + P6.5) is met.
Only the P6.7 helper crate is outstanding, and it's a developer-
experience convenience, not a blocker.

## References

- Shipped engine: `crates/runtime/wild-secrets/` (`backends.rs`,
  `grants.rs`, `grants_check.rs`, `audit.rs`, `index.rs`,
  `lib.rs`). Host wiring: `crates/runtime/wild-host/src/wild_secrets_plugin.rs`.
  CLI handlers: `crates/runtime/shared/src/secret.rs`. WIT:
  `wit/secrets/secrets.wit`. Config: `SecretsConfig` in
  `crates/runtime/wild-runtime/src/config.rs`.
- Sibling plugin pattern: `crates/runtime/wild-host/src/wild_ai_plugin.rs`,
  `wild_blobs_plugin.rs`, `wild_tasks_plugin.rs` — all share the
  trait + bindgen + host-adapter shape `wild:secrets` follows.
- Cross-cutting consumer: `wild/docs/llm-adapters.md` "Adapter
  delivery — two tiers" — Tier-2 components are the proximate
  driver of Phase 6.
- D4 install pattern:
  `~/.wild/profiles/<active>/system/plugin-cache/<slug>.json` —
  the alias map for non-adapter components rides on the same
  daemon-managed sidecar (`family_dir(Family::Plugin)`).
- Operation event pattern: `wild.{tribe}.system.operation.started/
  finished` — secrets-access events follow the same subject
  shape.
- Keychain Rust crate: `keyring` v3 (one cross-platform crate),
  selected per-OS by cargo feature — `apple-native` (macOS),
  `sync-secret-service` + `crypto-rust` (Linux), `windows-native`
  (Windows). Replaces the three separate per-OS crates the original
  design assumed.
