# Installation & first boot

> Companion to the [README](../README.md). Walks from a clean
> machine to a running Tribe.

## Prerequisites

- **Rust 1.97.0** — only needed for path (c) below (build from
  source). Pinned via `rust-toolchain.toml` at the repo root.
  `rustup` reads the toolchain file automatically; no manual
  version selection needed.
- **macOS or Linux** is the supported target surface. Windows
  isn't supported; use WSL and run anything from inside the WSL shell.
- **NATS** — the embedded host can supervise its own NATS server
  (default), or you can point it at an existing one via
  `WILD_NATS_URL`. `wild doctor` reports which path you're on.
- **Docker** — only required if you want the Forge to build new
  Wasm components. Without Docker, the embedded host runs fine
  but `forge_build` requests fail with an explicit
  `forge-not-available` message that tells you whether Docker is
  missing or just not running. `wild doctor` surfaces the same
  state before a build is attempted.
- **`cargo-component`, `wkg`** — only needed when building Wasm
  components from source (development workflow). Skipped for
  pure consumers; see `docs/development.md`.

## Install paths

There are three ways to land a `wild` binary on your machine.
Pick whichever matches your platform + workflow.

### (a) curl-pipe installer (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/wildstuff/the-wild/main/install.sh | sh
```

Pulls the prebuilt host binary from the latest GitHub Release
that matches your OS + arch, verifies its sha256, drops it in
`$HOME/.wild/bin/wild`. Knobs (env vars):

| Var | Default | Effect |
|---|---|---|
| `WILD_VERSION` | latest GitHub Release | Pin to a tag (`v0.1.2`). |
| `WILD_INSTALL_DIR` | `$HOME/.wild/bin` | Install location for the binary. |
| `WILD_NO_MODIFY_PATH` | `0` | Set to `1` to suppress the PATH-hint output. |

Targets the script can install today (matches
`.github/workflows/release.yml`'s build matrix):

- macOS arm64 → `aarch64-apple-darwin`
- macOS x86_64 → `x86_64-apple-darwin`
- Linux x86_64 → `x86_64-unknown-linux-gnu`

Linux aarch64 isn't published yet — the script exits with a
build-from-source hint there. Windows isn't supported; use WSL
and run this installer from inside the WSL shell.

### (b) Homebrew

```bash
brew install wildstuff/tap/wild
```

Custom tap (not homebrew-core yet). Currently a source build via
`cargo install` (3-5 min); will switch to a binary Formula once
release.yml's tarballs have a stable track record. Tap setup
notes: `docs/homebrew-tap-setup.md`.

### (c) Build from source

For any platform Rust supports, including Linux aarch64 until
that binary lane catches up.

```bash
# Clone the repo
git clone https://github.com/wildstuff/the-wild
cd the-wild

# First-time setup: verify the pinned external tools are present
cargo run -p xtask -- check-tools     # `install-tools` fetches any missing

# Build the host binaries (wild + wild-hostd)
cargo build -p wild-frontend -p wild-daemon --release
# → ./target/release/wild and ./target/release/wild-hostd
```

Symlink the binaries into your `$PATH` (or `cargo install --path
crates/runtime/frontend && cargo install --path crates/runtime/daemon`
if you prefer):

```bash
ln -sf "$(pwd)/target/release/wild" /usr/local/bin/wild
wild --version
```

## First boot — `wild up`

The runtime is **headless by default** (ADR-0010). `wild up`
spawns `wild-hostd` as a child process (ADR-0035 §3.3.2.C),
supervises it for the lifetime of the invocation, and drains it
gracefully on Ctrl-C / `wild down`.

```bash
wild up
# → wild up: starting wild-hostd...
# → boot sequence (daemon's stderr surfaces in your terminal):
#   ✓ NATS supervised (or external)
#   ✓ JetStream up
#   ✓ tribes.db migrated
#   ✓ embedded host ready
#   ✓ default chief (Tier-1.5) registered
#   ✓ ai-worker / http-fetcher / pdf-parser pulled (first run)
# → wild-hostd ready. Control socket answering.
# → wild up: attached. Ctrl-C drains the daemon.
```

To spawn the host as a detached `wild-hostd` daemon that keeps
running for later `wild` invocations to attach (instead of blocking
until Ctrl-C), pass `--daemon`; `wild down` shuts it down. (The
legacy `--in-process` flag was retired with ADR-0036 Phase H.)

For a daemon that survives shell exit + reboots (the recommended
shape for ongoing use), register the LaunchAgent / systemd unit
that ships in `dist/` — see
[`docs/operator-daemon.md`](operator-daemon.md).

What happens on first boot:

- **`~/.wild/`** is materialised (profile dir, OCI cache,
  bootstrap.yaml seed copy if missing).
- **`bootstrap.yaml`** drives which Tier-2 plugins get pulled —
  see `docs/bootstrap-and-default-inventory.md`
  for the full schema. The embedded default ships
  `ai-worker / http-fetcher / pdf-parser / anthropic-cli /
  openrouter / math-tools` enabled.
- **`bootstrap.lock`** pins each plugin to an OCI digest. Commit
  it for reproducible deploys, leave it gitignored on personal
  devboxes.
- **`tribes.db`** is the per-profile SQLite mirror. Schema
  migrations apply automatically on each boot.

If you're laptop-developing and want one terminal with the watch
console wired in:

```bash
wild up --watch
# → starts the runtime AND opens the watch console in-process
```

For the conversational REPL, run `wild chat` in a second terminal:

```bash
wild chat
# → "What's your challenge?"
```

## Verify with `wild doctor`

`wild doctor` walks every layer the runtime touches and prints a
structured pass/fail report. Run it after `wild up` from another
terminal:

```bash
wild doctor
# wild doctor — runtime health checks
#
#   · preflight tools          (no PINS configured)
#   · claude CLI               /opt/homebrew/bin/claude (1.2.3)
#   ✓ nats-server binary       /opt/homebrew/bin/nats-server
#   ✓ ~/.wild/nats.conf
#   ✓ llama-server binary      ~/.wild/bin/llama/llama-server
#   ✓ llama embed model (vec)  bge-m3-embed @ http://127.0.0.1:11435 — healthy, 1024-dim vector
#   ✓ llama rerank model       bge-reranker-v2-m3 @ http://127.0.0.1:11436 — healthy, top score 0.912
#   ✓ NATS @ nats://127.0.0.1:4222
#   ✓ JetStream                ($JS.API.INFO ok)
#   ✓ tribe-state KV           bucket present
#   ✓ tribe-registry KV        bucket present
#   ✓ embedded runtime         host ready, 0 workloads
```

The two `llama …` rows fire a real embed / rerank request at the
loopback sidecars, so a green row means vec + rerank actually
answer — not merely that the binary and model files are present.
Before `wild up` has started the sidecars they soften to `·`
(`… sidecar not answering … — is wild up running?`); a missing
binary or model points at `wild install-deps` / `wild models
pull`.

A blocking failure (NATS unreachable, JetStream off) returns
non-zero. Soft signals (no runtime running yet, no demo data
seeded) print a hint but exit zero so `wild doctor` doubles as
"tell me what's running."

## Connect — chat REPL or watch console

Bare `wild` (no subcommand) opens an inline chat REPL with native
scrollback / selection / Cmd-C — Claude-Code-style shell:

```bash
wild
# → "What's your challenge?"
```

Connection details (NATS URL, credentials, default profile) live
in `~/.wild/cli.toml`. `wild config show` prints the resolved
chain. To target a remote daemon:

```bash
wild --host=ops.internal:4222
```

For live multi-tribe monitoring (Activity · Tribes · Bus panes),
run the watch console in a second terminal tab:

```bash
wild watch
```

The full surface map is in [`docs/cli.md`](cli.md). Slash-command
reference + streaming details: `docs/inline-chat-design.md`.
Watch console layout + Forge-pane extension: `docs/watch-dashboard-design.md`.

## First conversation — talking to Elder

When you hit "What's your challenge?", you're talking to Elder
(the system-tribe orchestrator, ADR-0001). Elder's job is to:

- **Onboard** — figure out what you want, ask clarifying
  questions, search prior conversations for similar topics.
- **Spawn** — when you're ready, deploy a Tribe with the right
  Chief + initial worker roster.
- **Route** — route subsequent user input to the right Tribe (or
  back to Elder for cross-tribe ops).
- **Operate** — apply blueprint changes, swap a worker image,
  stop a tribe.

Try:

```
> I want to monitor competitor pricing weekly and alert me when
  something changes by more than 5%.

Elder: That sounds like a recurring crawl-and-diff job. Before I
  spawn a tribe, let me ask a few things:
  - Which competitors? (URLs or names?)
  - "Pricing" — listed prices, or do we need to handle login /
    quote forms?
  - Where do you want the alert? Email, Slack, file, MCP-callback?
```

Elder will iterate with you, then offer to deploy. When you
confirm, a fresh Tribe materialises with the Chief running its
first cycle. From then on, the Tribe is the addressee — type
`/tribe <id>` in the chat REPL to switch the conversation to its
Chief.

## Bootstrap reproducibility

For multi-machine / production deploys, commit `bootstrap.lock`
alongside `bootstrap.yaml`. The lock pins each plugin to an OCI
digest:

```bash
wild bootstrap sync       # refresh lock from current manifest
wild bootstrap list       # show resolved digests
git add bootstrap.lock
git commit -m "pin bootstrap to verified digests"
```

The boot path refuses to start when the manifest and the lock
have drifted (`WILD_OFFLINE=1` overrides this for air-gapped
re-runs against a populated cache). Full schema + drift handling
in `docs/bootstrap-and-default-inventory.md`.

## Optional: sharpen recall with local embeddings

Everything above works with **exact-shape** recall: a Tribe (and the
Elder, when designing one) surfaces past lessons whose goal *shape*
matches the one at hand. You can optionally upgrade this to
**semantic recall** — matching *similar* shapes, not just identical
ones — by running a local embedding model. It is **off by default**;
without it, recall simply falls back to exact-shape matching, and
nothing else changes.

**Why turn it on**

- A Tribe recalls lessons from *adjacent* problems, not only ones
  shaped the same way — so it reuses experience more often.
- The Elder, while designing a new Tribe, draws on what the **whole
  fleet** has already learned about similar challenges (the knowledge
  commons — ADR-0058).
- It runs **fully local and zero-cost**: an Ollama model on your own
  machine — no data leaves the box, no per-token bill.

**Enable it**

```bash
# 1. Run Ollama locally and pull an embedding model
ollama serve
ollama pull nomic-embed-text          # 137M, 768-dim — the default

# 2. Build + install the embed-adapter sidecar into your profile
cd plugins/embed/ollama && cargo component build --release
wild plugin add target/wasm32-wasip1/release/ollama_embed.wasm \
  --name ollama-embed --kind provider \
  --provides wild:embed-adapter@0.1.0 \
  --config-key model --config-key endpoint --no-pull

# 3. Restart the runtime — the shipped default wires the adapter
wild up
```

The embedded `embed-adapters.yaml` already points `ollama-embed` at
`nomic-embed-text` on `http://127.0.0.1:11434`, so no further config
is needed for the default model. Confirm it wired in the boot log:

```
category: "boot.subsystem-up", subsystem: "wild:ai/embed", "… Tier-2 adapter wired"
```

**Ollama on a non-default port?** It's pure config — no rebuild. Edit
`~/.wild/<profile>/embed-adapters.yaml` and change the `endpoint`
field (e.g. `http://127.0.0.1:11500` if your Ollama listens
elsewhere), then `wild up`. The `--config-key` flags on `wild plugin
add` only *allow* those keys; the actual values are read from this
YAML, never from the install command. Swapping the `model` works the
same way **only for another 768-dim model** — the vector width is
fixed in the `learning` schema, so a different-dimension model is a
schema change that needs the learnings re-seeded (see
`embed-adapters.md`).

Once live, the daemon backfills embeddings for existing learnings on
its own (a periodic host pass) — **there is no command to run**. New
Tribes are born ready; Tribes created *before* you enabled it keep
working on exact-shape recall until their learnings are re-seeded.

> **Note (local build).** A from-source `wild plugin add` stamps the
> sidecar version `0.0.0+local`, which trips the version-integrity
> gate at warm-up. Set `version` in
> `~/.wild/<profile>/system/plugin-cache/ollama-embed.json` to match
> the component (`0.1.0`). An OCI-published adapter carries the right
> version and skips this.

> 🔍 **Dig deeper — embeddings & semantic recall.**
>
> - **Concept:** [`self-modeling.md`](self-modeling.md) (the relational learning layer; semantic recall in full) · ADR-0058 (knowledge commons + outcome history).
> - **Reference:** `embed-adapters.md` — the full `wild:ai/embed` adapter contract, the install walkthrough, and the conformance + live-E2E notes.
> - **On disk:** the registry is `~/.wild/<profile>/embed-adapters.yaml` (shipped default entry: `ollama-nomic-embed`).

## What's next

- **CLI cheatsheet:** [`docs/cli.md`](cli.md)
- **Inline chat REPL design:** `docs/inline-chat-design.md`
- **Watch console design:** `docs/watch-dashboard-design.md`
- **Elder walkthrough:** `docs/elder.md`
- **Configure LLM adapters:** `docs/llm-adapters.md`
- **Add a plugin:** [`docs/plugin-concept.md`](plugin-concept.md)
- **Develop on the host:** `docs/development.md`
