# The CLI

> The **complete** command reference — every verb and flag is here.
> Companion to ADR-0010 (the
> rationale) and [`getting-started.md`](getting-started.md) (the
> friendly first run). House style: `STYLE.md`.

One binary, **three surfaces**. `wild` doesn't run a single all-in-one
TUI; it splits by *what you're doing*:

```mermaid
flowchart TD
    bin["the <b>wild</b> binary"]
    bin --> CLI["<b>CLI verbs</b><br/>one-shot · scriptable<br/>--json everywhere"]
    bin --> Chat["<b>wild chat</b> (or bare wild)<br/>conversational REPL<br/>native scrollback"]
    bin --> Watch["<b>wild watch</b><br/>monitoring console<br/>Activity · Tribes · Inbox · Bus · Forge"]
```

- **`wild` / `wild chat`** — inline chat REPL with native scrollback / selection.
- **`wild watch`** — full-screen monitoring console (Activity · Tribes · Inbox · Bus · Forge).
- **CLI verbs** — one-shot, scriptable commands (`wild up`, `wild config llm …`).

> 🔍 **Dig deeper — the surfaces.** Chat REPL design (slash matrix,
> streaming): `inline-chat-design.md`. Watch
> console layout: `watch-dashboard-design.md`.

## The split rule

> **"Wäre das in einem Bash-Script sinnvoll? → CLI. Multi-turn Konversation? → chat. Live-Monitoring? → watch."**

A **CLI command** is one-shot, deterministic, scriptable, and exits
cleanly. The **chat REPL** is the conversational surface — talk to
the Elder or a tribe, paste a multi-line message, get the assistant's
streamed reply inline; native terminal scrollback / selection / Cmd-C
all work. The **watch console** is the monitoring dashboard — fixed
panes for Activity, Tribes, Inbox, Bus, Forge, no input field.

| Action | Surface | Reason |
|---|---|---|
| `wild up`, `wild down` | **CLI** | Lifecycle; CI scripts call them |
| `wild status`, `wild doctor` | **CLI** | One-shot health check |
| `wild config llm add ...` | **CLI** | Provisioning script |
| `wild tribe apply ./acme.tribe` | **CLI** | Idempotent ops |
| `wild data query scan inflow --tribe acme` | **CLI** | Backup / migration |
| `wild plugin add oci://...` | **CLI** | Operator install |
| `wild metrics --json` | **CLI** | One-shot host metrics |
| `wild traces list --json` | **CLI** | One-shot history read |
| `wild content ls acme` | **CLI** | One-shot capture queue read |
| `wild content rerun acme <blob_ref>` | **CLI** | Re-queue document enrichment |
| `wild flows ls acme` | **CLI** | One-shot Flow read (ADR-0185) |
| `wild flows show acme <flow>` | **CLI** | Flow spec + recent runs |
| `wild flows feed acme <flow>` | **CLI** | Hand an item into a Flow (#4597) |
| `wild flows run retry acme <run-id>` | **CLI** | Re-trigger a failed Flow run (#4112) |
| Multi-turn chat with the Elder / a tribe | **chat REPL** | Interactive, scrollback-friendly |
| `/blueprint edit` (suspend → `$EDITOR`) | **chat REPL** | Multi-step form |
| `/diff <key>`, `/upload <path>`, `/copy`, `/save` | **chat REPL** | Context-aware slashes inline |
| Live multi-tribe Activity + Bus + lifecycle indicators | **watch console** | Continuous render across panes |
| Health summary at-a-glance | **watch console** | Always-visible alongside event stream |

## Command families at a glance

Every family supports `--json` for scripting and `--help` for the
canonical flag list. Jump to one below, or read the full cheatsheet.

```mermaid
flowchart LR
    L["<b>Lifecycle</b><br/>up · down<br/>status · doctor"]
    P["<b>Provisioning</b><br/>config · plugin<br/>bootstrap · component-type"]
    D["<b>Data &amp; specs</b><br/>data · spec · change<br/>forge · search"]
    O["<b>Operate</b><br/>metrics · reconcile<br/>traces · loops<br/>llm usage"]
    S["<b>Secrets &amp; service</b><br/>secret · service<br/>mcp · profile"]
```

## CLI cheatsheet

### Lifecycle

```bash
wild up                    # spawn `wild-hostd` as a child + attach (default
                           # per ADR-0035 §3.3.2.C); cli supervises the
                           # daemon and drains it on Ctrl-C
wild up --watch            # opens the watch console alongside the runtime
                           # in this same process (headless without it)
wild up --daemon           # spawn detached and return; daemon stays running
                           # for subsequent `wild` invocations to attach
wild up --mcp-http 127.0.0.1:7531
                           # MCP-over-HTTP operator plane at an explicit
                           # bind. Explicit = required: a failed bind
                           # (port already held) aborts boot non-zero
                           # (issue #3196); unset ⇒ profile `mcp_port`
                           # fallback, best-effort
wild up --rest-http 127.0.0.1:8088
                           # ADR-0050 V1: also host the REST control/query
                           # plane (OpenAPI at /api/v1/openapi.json). Opt-in
                           # (MCP-over-HTTP already serves every tool);
                           # loopback-only in V1. `WILD_REST_BIND` = same.
wild up --customer-http 127.0.0.1:8090
                           # ADR-0154 D3 / ADR-0083: bind hostd's private
                           # CUSTOMER listener (the plane `wild-appd` proxies
                           # to). Opt-in; keep it loopback in the single-box V1.
wild up --with-appd        # also bring up the `wild-appd` end-user portal.
                           # Foreground: a reaped-on-drop child (stops on
                           # Ctrl-C). With `--daemon`: a DETACHED PID-file
                           # sidecar (like nats/llama) that survives the
                           # launcher and is reaped by `wild down`. Implies a
                           # loopback `--customer-http`.
wild down                  # shut down a running runtime
wild status                # one-shot status snapshot
wild status --json         # same, NDJSON
wild doctor                # walks every layer, prints pass/fail; targets the
                           # resolved profile (--profile > WILD_PROFILE > active
                           # pointer); a down bus degrades the NATS-backed rows
                           # to dim skips instead of ending the walk
wild doctor --fix          # ADR-0149 D7: regenerate a drifted write-once
                           # config (nats.conf) from the current template —
                           # previous file kept as a timestamped .bak, known
                           # values (port / store_dir / operator token)
                           # carried forward, unknown operator additions
                           # reported against the backup (never silently
                           # merged or dropped). No-op when the conf already
                           # matches; restart `wild up` for the new bus
                           # config to take effect
```

The default `wild up` mode (spawn-and-attach) keeps the operator UX
of "block until Ctrl-C" while moving the host into a separate
`wild-hostd` process. See [`docs/operator-daemon.md`](operator-daemon.md)
for the service-managed install (LaunchAgent on macOS, systemd
user-unit on Linux) that replaces the foreground supervision when
the daemon should survive shell exit + reboots.

### Chat / Watch

```bash
wild                       # bare → inline chat REPL (alias of `wild chat`)
wild chat                  # explicit form; --tribe X to scope to a tribe
wild watch                 # full-screen monitoring console (separate terminal)
wild --host=remote:4222    # attach to a remote daemon (same flag works for CLI)
wild config show           # print resolved connection config
wild attach                # print the current default chat addressee
wild attach <tribe>        # set the per-session default addressee (ADR-0001 §6);
                           # persists to profiles/<active>/state/active-tribe,
                           # consumed by chat/watch/TUI/MCP routing (default: root)
```

### Tribe ops

> **`tribe apply` vs `package install` — different jobs.** `apply`
> materializes a WHOLE authored Tribe from a bundle (ontology + workers +
> Chief + live source locators + seed data) — the authoring/dev
> primitive. `package install` grafts a sanitized, versioned DECLARATIONS
> subset from a publisher (no locators, no workers). Author with
> Elder/`apply`; distribute with `package`. See
> [`tribe-creation-paths.md`](tribe-creation-paths.md).

```bash
wild tribe list                          # human table; a Domain column appears when any tribe declares domain membership (ADR-0118)
wild tribe list --json                   # NDJSON, one tribe per line; every row carries `domain` (null = not in any domain)
wild tribe list --with-intent            # ADR-0008 — show originating session
wild tribe status <tribe>                # per-tribe status snapshot
wild tribe apply <bundle-dir>            # apply AND START (ADR-0031 amendment 2026-07-31): manifest + blueprint + ontology/seeds.yaml pinned + sample-data ingested + documents/** delivered + apps/ installed, then activated through the SAME readiness gate `tribe activate` uses
wild tribe apply <bundle-dir> --dormant  # register dormant instead — pinned, chief + connectors idle until `tribe activate`
wild tribe apply <bundle-dir> --retire   # ADR-0221 — ALSO carry out the retirements the bundle implies (a source or command its model no longer declares: the feed stops arming, the action leaves every surface). WITHOUT the flag apply reports them and leaves the running system alone — read that report first: an OLDER bundle retires everything the live model gained since it was cut
wild tribe activate <tribe>              # start a dormant (or paused) tribe — the explicit start
wild tribe pause <tribe>                 # pause a running tribe (unloads the workload AND drains its schedules → ~zero cost)
wild tribe resume <tribe>                # resume a paused tribe (re-deploys + re-arms schedules)
wild tribe unarchive <tribe>             # ADR-0030 — the door OUT of archived: restore an archived tribe to dormant (inert but present); run `wild tribe activate` after. Refused with invalid-transition when the tribe isn't archived
wild tribe reflect <tribe>               # run the tribe's SELF-REVIEW now instead of waiting for its cron (`--window-days 7`)
wild tribe stop <tribe>                  # stop a running tribe
wild tribe backup <tribe>                # ADR-0117 — archive tribes/<tribe>/ PLUS the tribe's profile-level apps (ADR-0154 D9: the apps whose `default_tribe` is this tribe — spec, i18n sidecar, .published marker, history) → ./<tribe>-backup.tar.gz (excludes the rebuildable projection); local FS op, no daemon needed. The bundle starts with a manifest.yaml (ADR-0149 D8: format/wild/layout versions + per-file sha256). An app naming no tribe is in no backup and is reported by name. Back up while the tribe is idle.
wild tribe backup <tribe> --out <path>   # write the archive to a chosen path
wild tribe restore <archive.tar.gz>      # ADR-0117 — unpack a tribe backup under the active profile; the read-model rebuilds from the event log on next open. Validates the manifest first (ADR-0149 D8): newer bundle format → refused naming both versions; layout mismatch → refused until restore-time migrations arrive (ADR-0149 D6); checksum mismatch → refused naming the file; a manifest-less generation-1 bundle stays accepted. Refuses to overwrite unless --force — including an app-id collision in the shared apps/ dir, named before anything is written
wild tribe restore <archive.tar.gz> --force  # replace an existing tribe with the archive's copy
```

**Starting is the default (ADR-0031 amendment).** Pointing `apply` at a
prepared bundle is a decision already taken, and a tribe with its data loaded
but its time-driven writes idle (the ADR-0157 aging tick) reads as broken
rather than pending. The start does NOT ride `HDR_DEPLOY_START` — that bridge
is record-only and would start *around* the readiness gate. Apply registers
dormant, then walks `tribe activate`'s own door, so the active-tribe cap,
`NeedsSecrets`, `UnmatchedCapability` and `NeedsGenesis` all still apply. A
refusal is an outcome, not an apply failure: the ontology is pinned, the data
is in, and the tribe stays dormant with the reason and the retry command. This
changed `apply` only — `tribe_create` in chat still pins a dormant sketch.

**The apps lane.** A bundle's `apps/<id>.yaml` specs are installed into the
PROFILE-level app plane (`<profile>/apps/`, ADR-0154 D9) — until this existed,
only the package lane wrote there and a tribe bundle's apps were dead files.
The installed copy is stamped with the tribe it was applied as (a bundle omits
`default_tribe` to stay portable under `--as`; a profile-level app has no
carrier tribe, so the binding must be written down). Re-apply follows the
seeded-file baseline: unchanged since we wrote it → refreshed; edited by the
operator → kept, and the report says their copy is behind; no baseline (a
profile that predates the lane) → adopted, nothing overwritten. An id another
tribe's bundle already owns is refused by name — `apps/` is one shared
namespace.

**The documents lane.** A bundle's `data/*.csv` feeds the store; its
`documents/**` feeds the tribe's FILE store
(`tribes/<tribe>/files/documents/…`), where a declared document connector
actually reads — otherwise a bundle could ship a document door and eight
sample scans and still hand the operator an empty inbox. Delivery is
**one-shot per path**, recorded in a per-tribe ledger: a scan you processed
and deleted does not come back on the next apply, and a delivered file is
never overwritten, but a sample ADDED by a later bundle version still arrives.
The report line is `documents:  N sample document(s) delivered …`.

`wild tribe apply` on a `ddd`-stamped bundle additionally surfaces two
ADR-0157 D4 behaviours:

- **Compile notices** — after the (errors-only) cross-layer gate passes,
  the non-blocking NOTICES lane prints each teaching finding as a
  `notice: …` line beside the usual warnings (dead mirror vocabulary,
  a state member nothing writes, a clock-shaped process guard nothing
  ever produces, `aging:` declared without its `requires:` marker or on
  a build lacking the reactive lane). Notices never block the apply.
- **`requires:` feature tokens** — a bundle whose `manifest.yaml` names
  a runtime capability token this daemon does not know (known set today:
  `aging`) is REFUSED before any side effect, naming the missing
  token(s) — never silently applied minus the behaviour it depends on
  (ADR-0157 D4.4). Absent/empty `requires:` applies as before.

### Configuration (ADR-0009 — RPC-routed)

```bash
# LLM adapters
wild config llm list                     # tabular
wild config llm list --json              # NDJSON
wild config llm add --kind component --slug openai --id kimi \
  --model kimi-k2-0711-preview \
  --endpoint https://api.moonshot.ai/v1/chat/completions
wild config llm remove <id>
wild config llm test <id>                # ping the adapter, no provisioning

# Profile / connection
wild config show                         # resolved cli.toml chain
wild config show --json
```

Config-touching CLI verbs route through the daemon over NATS —
there's one source of truth. The TUI mirrors with `/config llm
<op>` slash commands.

### Plugins

```bash
wild plugin add oci://ghcr.io/wildstuff/plugins/tools/math-tools:0.1.0 \
  --provides wild:tool-provider/tools@0.4.0
wild plugin add oci://example/chief-research:0.1.0    # ADR-0014; kind is derived
wild plugin list                         # added plugins
wild plugin list --json
wild plugin show <slug>                  # sidecar + derived primitive roles
wild plugin remove <slug>
wild plugin upgrade <slug>               # ADR-0223: apply the feed-offered update
                                         # (daemon door: compare → pull → pin → live reload)
wild plugin replace <target> --name <slug>  # manual re-install from an explicit
                                         # target (alias: `update`, deprecated)
```

### Component-types (ADR-0009 Phase 5 / ADR-0012)

```bash
wild component-type list                 # approved-only by default
wild component-type list --all           # incl. pending / revoked
wild component-type show <type-name>
wild component-type approve <type-name>  # operator-only
wild component-type revoke <type-name>
```

### Bootstrap inventory

```bash
wild bootstrap list                      # list seed plugins
wild bootstrap add <name> <oci-ref>      # add a Tier-2 entry
wild bootstrap remove <name>
wild bootstrap enable / disable <name>
wild bootstrap sync                      # refresh lock from manifest
wild bootstrap reset                     # restore embedded default
```

### Data (`wild:data` membrane)

The event-sourced data membrane (default-on). All ops are tribe-scoped
via `--tribe <slug>`; `--json` everywhere. The CLI is the file/operator
second authority over the SAME store the WIT interfaces serve (one
backend, no second writer).

```bash
# Ontology types (FS-canonical `types/<slug>.yaml` author form)
wild data type apply ./inflow.yaml --tribe acme   # pin a type (new slug → v1; changed → next version)
wild data type ls --tribe acme                      # list pinned types
wild data type show inflow --tribe acme            # show one type (latest, or --version N)
wild data type render --tribe acme                 # regenerate read-only tribes/<t>/types/<slug>/schema.generated.yaml (backstop; the on-write render keeps it fresh)

# Typed SchemaDelta migration of an EXISTING pinned type — the operator's
# HEADLESS migration surface. The pin seam refuses a BREAKING re-pin ("route
# it through propose_migration"); this verb IS that route for the CLI: the
# delta is compat-classified first (the ONE rule the pin seam uses), and a
# Breaking delta (required-flip, field/relation removal, enum shrink, retype)
# is refused unless --allow-breaking confirms it — the explicit flag is the
# operator's ratification (High-risk, schema-migration-classed, audited).
# Delta JSON, e.g.: {"set_required":[{"field":"effective_status","required":true}]}
wild data type migrate invoice --delta-file ./delta.json --tribe acme
wild data type migrate invoice --delta-file ./delta.json --allow-breaking --tribe acme

# Read records
wild data query scan inflow --tribe acme --limit 100   # current records (live head)
wild data query get inflow D001 --tribe acme           # one record by key
wild data query aggregate inflow --tribe acme \
    --op sum --field amount_eur                          # count | sum (decimal → exact string)

# Derived views (Phase-2 virtual entity — deterministic, decimal-exact)
wild data query view cash-coverage --tribe acme --as-of 2026-01-01
# → computes the declared measures over the view's sources, windowed on
#   the shared date dimension from `as-of`; returns measures + lineage/drift.

# Privileged ingest of `source-mirror` records from a JSONL file
wild data ingest inflow ./inflow.jsonl --tribe acme   # one {"key":…, "value":{…}} per line
# ADR-0065 D2 — stamp the external origin so a real `source` + `mirrored_from`
# edge is recorded ("where did this come from?"); absent ⇒ a `cli-ingest` dock.
wild data ingest price ./prices.jsonl --tribe fred \
  --source-kind web-page --source-uri https://competitor.example/prices

# Privileged upsert of AUTHORED records — the authored-write twin of ingest
# (authored fields take upsert, source-mirror feeds take ingest; on a
# mixed-origin type the fold merges the authored patch onto the mirror).
wild data upsert invoice ./verdicts.jsonl --tribe acme

# LLM-prompt Function evals — golden-case corpus for `llm-prompt`-backed
# Functions (ADR-0204 PR1). Runs each case through the SAME `llm_prompt`
# tool path production uses, scores k-of-n trials, and writes
# `trials.jsonl` + `summary.{md,json}`. Advisory only — it reports, it
# never gates deploy.
wild data function llm-prompt-eval acme partner_risk_band   # one function
wild data function llm-prompt-eval acme                     # every *.llm-prompt-eval.yaml in the tribe
wild data function llm-prompt-eval acme partner_risk_band --case short-terms-are-low
wild data function llm-prompt-eval acme partner_risk_band --trials-k 2 --trials-n 3 --out ./eval-run
```

### Snapshots (`wild snapshot` — ADR-0201 PR3)

Named bitemporal bookmarks. A snapshot pins a `(known_at, valid_at)` pair
(or an event-log `position`) by name so reports, apps, and read tools can
reference a stable as-of view without repeating dates. The store is
profile-global, not tribe-scoped. `--json` everywhere.

```bash
wild snapshot create year-end-2025 --valid-at 2025-12-31 --known-at 2026-01-15
wild snapshot create migration-cutoff --position 42          # logical event-log position
wild snapshot ls                                              # list all snapshots
wild snapshot show year-end-2025                              # show the pinned coordinates
wild snapshot rm year-end-2025                                # delete (alias: remove)
```

A snapshot used as `snapshot: year-end-2025` in a read context resolves to
the same coordinates as passing the raw `known_at`/`valid_at` values. If both
a raw axis and `snapshot` are supplied, the raw axis wins (explicit beats
named). `--known-at` and `--position` are mutually exclusive when creating a
snapshot; at least one of `--valid-at`, `--known-at`, or `--position` is
required.

### Connector status (`wild connector status`)

PR-F — dedicated operator status for configured intake sources. Reports
slug/name/locator/target-type, armed/disarmed state, triggers, last run,
current run, last fault, and credential readiness for each source.

A run that finished but **parked** what it read is reported as
`N file(s) waiting for their columns to be mapped`, and counted in the
roll-up as `awaiting_mapping` — parking is the designed outcome for an
unknown shape, so the job status alone stays `done` and the source used
to read `ready and idle` while ingesting nothing.

```bash
wild connector status                 # active tribe (or root)
wild connector status acme            # specific tribe
wild connector status acme --json     # compact JSON for jq
```

### WCAG contrast calculator (`wild contrast`)

Compute the WCAG 2.1 relative-luminance contrast ratio between two sRGB
colors and report AA/AAA pass/fail for normal and large text. Colors can be
`#RGB`, `#RRGGBB`, or a CSS color name (`black`, `white`, `red`, `orange`, …).

```bash
wild contrast black white                # 21.00:1 — all levels PASS
wild contrast '#777' '#fff'              # 4.48:1 — AA normal FAIL, AA large PASS
wild contrast orange white --json        # machine-readable JSON output
```

### Egress allowlist (`wild egress` — ADR-0159)

Governed outbound egress is **default-deny**: a forged source connector (e.g.
a mobile.de importer) can't reach a host until it's on `system/egress.yaml`.
`wild egress allow` is the guided writer — it appends a narrow, tribe-scoped
destination idempotently, so an operator satisfies a connector's
`egress-needed` grant without hand-editing YAML. (The dashboard/Elder path is
the `wild.system.egress.add` governed action.)

```bash
wild egress ls                                  # print the current allowlist
wild egress allow mobile.de --tribe acme        # allow one host for one tribe
wild egress allow api.mobile.de mobile.de       # several hosts (all tribes if --tribe omitted)
wild egress allow mobile.de --methods get,post  # restrict to specific methods (default: all)
# ADR-0167 D3 — bind a provider token as `Authorization: Bearer` on the destination
# (a channel's outbound token; the alias resolves from keychain/env, never enters the guest):
wild egress allow gate.whapi.cloud --bearer-secret whapi-api --bearer-binding whapi_api
```

### Channel bindings (`wild channel` — ADR-0167)

A customer channel routes to a tribe through `system/channels.yaml`; an unbound
`(channel, tribe)` makes the inbound webhook answer 503. `wild channel bind` is
the guided writer — it upserts one binding idempotently (preserving any other
row's outbound routing fields), so an operator wires a channel without
hand-editing YAML. (The dashboard/Elder path is the `wild.system.channel.bind`
governed action; the in-chat `bind_channel` tool follows with the guided
onboarding escalation.)

```bash
wild channel ls                                             # print the current bindings
wild channel bind whapi --tribe acme \
  --inbound-secret whapi-webhook --on-unknown-sender mint_ephemeral
wild channel bind whapi --tribe acme --route chief          # non-customer inbound route
```

### Generative UI (ADR-0063 §6 — `wild ui`)

The M3 proof-of-concept for the Tier-1 UI-descriptor vocabulary
(`common::ui_descriptor`): it auto-builds a declarative descriptor for a
type, fetches the data live over the same `wild data` path, and renders it
in the terminal — exercising the **descriptor → fetch → render** loop the
chief will eventually drive (`chief_reply_to_user`). Operator-only (no
ADR-0050 identity yet).

```bash
wild ui invoice --tribe acme                 # render a type as a table (title · row-count · table)
wild ui invoice --tribe acme --columns id,amount,status   # declare the bound columns
wild ui invoice --tribe acme --filter status=open --sort amount:desc   # bound filter + sort (source.filter/sort)
wild ui invoice --tribe acme --chart status,amount         # add a chart block (group by status, sum amount)
wild ui invoice --tribe acme --descriptor    # dump the JSON wire-format the chief would emit
wild ui --from reply.json --tribe acme       # render a descriptor emitted elsewhere (e.g. a chief reply); `-` = stdin
```

### Tools (tool-provider dev / debug)

Operator/dev surface over the host's aggregated tool-provider catalog —
the **same** dispatch path a worker's `wild:tools/invoke` takes, minus
the LLM loop. Use it to smoke-test a freshly-added provider plugin (Rust
or a jco/ComponentizeJS JS component) without standing up a tribe +
worker. NB: this is not how an *agent* calls a tool — see
`docs/plugin-developer-guide.md` § "Who invokes a provider tool".

```bash
wild tool ls                                    # flattened catalog: name · provider · description
wild tool ls --json
wild tool invoke md-render '{"markdown":"# Hi"}'   # dispatch by name, print the result
wild tool invoke slugify '{"text":"Héllo Wörld"}' --json
```

### Diagnostics / control plane (ADR-0035 §3.3.3)

```bash
wild metrics                             # host metrics in one round-trip
wild metrics --json                      # same shape as Response::Metrics, for jq
wild debug inventory                     # dump daemon-internal state (host id, workloads, WIT)
wild debug vitals                        # every tribe's ADR-0136 health rows WITH the detail
                                         #   line + remediation `/readyz` folds away; no token,
                                         #   no REST listener needed (see docs/observability.md)
wild reconcile                           # force the reconcile loop to tick now (all tribes)
wild reconcile <tribe>                   # scope the tick to one tribe
wild log-level <directive>               # live-set the daemon log filter
                                         #   (e.g. `info`, `wild_host=debug`)
```

There is no `wild events` / `wild logs` verb — the event firehose is
the NATS bus + the daemon log + the `wild watch` Bus pane (see
`docs/observability.md`); replayable records read
via `wild traces list` / `wild loops list` below.

### Elder prompt introspection (`wild elder` — #3503)

```bash
wild elder dump-prompt --surface runner --mode intake  # LIVE agentic-loop L0 (base +
                                                       #   intake.md + stance-ddd.md +
                                                       #   profile ELDER.md + overrides)
wild elder dump-prompt --surface domain-elder          # dashboard/REST per-tribe chat,
                                                       #   static L0 (--dashboard adds
                                                       #   the dashboard fragment)
wild elder dump-prompt --surface domain-elder-root     # same, root chat persona
wild elder dump-prompt --surface mcp --mode operate    # pure host-Elder/MCP stack
wild elder dump-prompt --surface chief                 # pointer: CLI `wild chat` REPL
                                                       #   is chief-served (in-guest)
wild elder mode-report [--json]                        # ADR-0190 misclassification
                                                       #   ledger: initial mode picks
                                                       #   vs consent switches
```

Prints the exact system prompt a surface hands the model — the
authoritative replacement for the canary-emoji "which prompt file is
live" hack. `runner` and `domain-elder`[`-root`] call the SAME composer
functions the live paths call (`wild_shared::{elder_prompt,
domain_elder_prompt}`), so the dump is byte-identical to the live static
(L0) composition; per-turn dynamic grounding sections are named in the
stderr banner, never faked. The `mcp` surface is the pure
`common::elder_mode` stack; `--surface chief` (the CLI `wild chat` REPL,
composed in the chief wasm guest) prints an honest pointer to
`assets/prompts/chief-default/cycle.md` rather than a wrong answer. The
banner goes to stderr, so `… --surface runner > out.md` captures only
the prompt body. See `docs/prompt-layers.md` for the
surface→files map.

### Spec authoring + change triage (ADR-0034)

```bash
# Validate hand-written specs/briefs — read-only, no daemon/NATS needed
wild spec validate <tribe>               # checks tribes/<tribe>/{specs,briefs}/;
                                         # exit 0 clean, 1 on any issue

# Triage pending change folders
wild change list <tribe>                         # pending changes for a tribe
wild change show <tribe> <change-id>             # proposal + parsed deltas
wild change accept <tribe> <change-id>           # merge deltas into live specs/briefs,
                                                 # archive; hot-reloads an Active tribe
wild change reject <tribe> <change-id> --reason "…"   # archive with -rejected suffix
wild change reconcile [--tribe <slug>] [--dry-run]    # auto-accept low-risk brief-only
                                                 # changes past their 24h grace window
```

### Published apps (ADR-0154)

```bash
# Validate an operator-published end-user app spec — read-only, no daemon/NATS.
# Parses apps/<id>.yaml through the closed 6-view schema, prints a plain-language
# summary (pages · views · bindings) and any structural problems.
wild app validate <path>                 # e.g. tribes/<tribe>/apps/liquidity-cockpit.yaml
                                         # exit 0 clean, 1 on any issue

# Serve the end-user portal (ADR-0154 D3). Brings up `wild-appd` in the
# foreground in front of a running tribe: it serves the app shell + forwards
# every read to hostd's private CUSTOMER listener with the end-user's own bearer
# (appd holds no token, reads no backend). Announced on the LAN over mDNS as
# http://<name>.local:<port> (Stage-1 URL story). Ctrl-C stops it.
wild app serve [--bind 0.0.0.0:8091] [--upstream http://127.0.0.1:8090] [--hostname wild]
wild app status                          # the portal's honest plain-language status
                                         # (published URL + whether it reaches the tribe)
```

```bash
# Install a DOMAIN PACKAGE (ADR-0156 — a declarations-only bundle: ontology
# with reading marks + app templates + unbound sample sources; never code).
# Without --yes: prints the D4 preview (types, actions in the author's words,
# the trust sentence, waiting-for-your-connection sources) and writes NOTHING.
# A name collision with a local / other-package type is REFUSED by name (the
# rename is yours); a prior install of the SAME package is the upgrade path —
# the pin-seam arbiter preserves your `local`-marked field edits.
# --with-example-data (OQ6.3, opt-in): if the package carries example rows, also
# ingest them as real records under a distinct `package-sample` source you can
# delete later. Off by default — your tribe stays clean until you opt in.
wild package install <dir> --tribe <slug> [--yes] [--with-example-data]

# ADR-0156 PR6 — the SUPPLY side: export a tribe's live ontology as a sanitized
# domain package (the inverse of install). Reads the ontology daemon-side
# (decompiles it back to a `model.yaml`), DISARMS every source to an `unbound`
# template (blanks the locator + secret handle, keeps the shape, moves the
# cadence to a hint), and writes a package directory an importer installs. A
# READ of the tribe — it mutates nothing. Refuses a re-export of declarations
# installed from ANOTHER package (naming it). Refuses a non-empty out-dir
# without --force. --name defaults to the tribe slug, --version to 0.1.0.
# --with-example-data (OQ6.3, opt-in): also ship <=50 example rows per type under
# data/<slug>.jsonl so an importer sees what a filled record looks like. Every
# `sensitive`/`internal` field is stripped first; a type whose REQUIRED field is
# sensitive ships no rows (a redacted sample could not re-ingest). Off by default.
wild package export <out-dir> --tribe <slug> [--name <n>] [--version <v>] [--force] [--with-example-data]

# ADR-0191 D3 — pack a package DIRECTORY into the canonical `.tar.gz` you publish
# to the marketplace (`xtask marketplace-domain-publish`). Byte-identical to the
# dashboard download and accepted by the daemon's reader; a raw `tar` is fragile
# (macOS AppleDouble `._` entries alone break it). See `docs/domain-packages.md`.
wild package pack <dir> --out <file.tar.gz>
```

`wild app serve` needs hostd's customer listener bound — bring both up together
with `wild up --customer-http 127.0.0.1:8090 --with-appd` (foreground; the portal
is co-supervised and stops with `wild up`). The detached form
`wild up --daemon --with-appd` brings the portal up as a PID-file sidecar that
survives the launcher and is reaped by `wild down` — this is the packaged macOS
app's autostart path (ADR-0120 D3), so a published app's URL resolves with no
`wild app serve` step.

The end-user UI is the `wild-portal` WASM bundle (ADR-0154 PR3c). Build it with
`cargo run -p xtask -- portal build` (a `dx build --platform web`); point
`wild-appd` at the output via `WILD_APPD_ASSETS=<dir>` (else `wild-appd` serves an
honest placeholder shell). End users open `http://<name>.local:<port>/apps/<tribe>/<app>#token=<their-token>`;
`?locale=de-DE` drives number grouping. `cargo run -p xtask -- portal serve` runs a
browser dev loop.

### Idea search (ADR-0026 — in-process, no daemon)

```bash
wild search query "<text>" [--limit 10]  # ranked hits across the three search tiers
wild search reindex                      # rebuild the Tier-3 inverted index from
                                         # ideas/*/spine.md (idempotent)
wild search documents "<text>" [--tribe root] [--limit 10] [--corpus documents]
                                         # full-text search the ADR-0089 DOCUMENT
                                         # corpus (intake-indexed PDFs/docs) for ONE
                                         # tribe, read from
                                         # <profile>/tribes/<tribe>/search/documents.json
```

### Forge (briefs + crate allowlist)

```bash
# Assemble the Forge brief for a tribe's capability
wild forge brief <tribe> <capability>            # render to stdout
wild forge brief <tribe> <capability> --save ./brief.md

# Wild-wide extra-crate allowlist (operator-managed, install-wide).
# Config file: <profile_root>/forge/allowlist.toml (NOT a WILD_* env var).
wild forge allowlist show                # effective allowlist (baseline + extras)
                                         # + derived sandbox-image status
wild forge allowlist sync                # bake the operator extras into a derived
                                         # sandbox image so they resolve in the
                                         # network=none build; no-op when no extras
```

### Skills (user-added)

```bash
wild skill add <path.md> [--force]       # validate + import a Skill MD into ~/.wild/skills/
wild skill update <path.md>              # replace an existing imported skill
wild skill list                          # (alias: ls) print every imported slug
wild skill remove <slug>                 # (alias: rm) delete an imported skill
```

### History / telemetry

```bash
# Episodic traces (one row per chief Reflect; ADR-0005)
wild traces list [--tribe <slug>] [--limit 20] [--json]    # newest first
wild traces export [--tribe <slug>] [--limit 10000] [--out ./corpus.jsonl]
                                         # JSONL training records (SFT/DPO corpus)

# Loop telemetry (per-turn / per-tool / per-budget signals; ADR-0027)
wild loops list [--tribe <slug>] [--limit 50] \
    [--since 24h] [--type loop.tool_call] [--json]   # --since: 30m|24h|7d|30d

# F3 judge rollup (per-tribe outcome distribution, mean score,
# iterate rate — the condensed read for ADR-0057 graduation evidence)
wild loops judge [--tribe <slug>] [--since 7d] [--json]

# Content Flow — captured documents + enrichment results (ADR-0171 Phase 1.9)
wild content ls [--days 7] [--json]                  # active tribe, last 7 days
wild content ls <tribe> [--days 7] [--json]          # specific tribe
wild content show <tribe> <blob_ref> [--json]        # provenance + enrichment detail
wild content rerun <tribe> <blob_ref>                # re-queue enrichment

# Flows — compiled DDD spec + runtime run state (ADR-0185 PR #2)
wild flows ls [<tribe>] [--json]                     # name, stages, status, last run + outcome (✓/✗), next run
wild flows show [<tribe>] <flow-slug> [--json]       # spec stages/edges + recent runs (each with its outcome)
wild flows feed [<tribe>] <flow-slug> [--item <json>] [--type-slug <slug>]  # hand an item into a flow
wild flows run retry [<tribe>] <run-id>              # re-trigger a failed run (content or connector flow)

# Metered LLM-call usage (ADR-0135 Layer B)
wild llm usage                           # by model, last 30 days, all tribes
wild llm usage --tribe acme              # scope to one tribe
wild llm usage --since 7d                # last 7 days
wild llm usage --dimension caller        # rollup by caller/strategy/adapter/model/tribe
wild llm usage --json                    # NDJSON buckets + total line
```

### Secrets (`wild:secrets`)

```bash
wild secret add <name> [--from-stdin]    # store a value in the OS keychain
wild secret list                         # list stored secret names (no values)
wild secret show <name> --confirm        # reveal a value (explicit opt-in)
wild secret remove <name>                # delete a stored secret
wild secret rotate <name> [--from-stdin] # replace a value in place
wild secret backend                      # show the active backend chain
```

Components read secrets via the `wild:secrets/store` WIT import; this
CLI is the operator-side surface for managing the values. Design:
[`docs/secrets.md`](secrets.md).

### OAuth (ADR-0143)

```bash
wild oauth login <binding> [--tribe <tribe>]   # one-time sign-in for an authorization_code binding
wild oauth grant <component-id> <binding>      # let a component call get-token for a binding
wild oauth revoke <component-id> [<binding>]   # remove one or all grants for a component
wild oauth grants [--component <id>] [--binding <name>]
```

`login` runs the one-time operator consent for an `authorization_code`
binding declared in `system/oauth.yaml`: opens the provider's
`authorize_url` (with PKCE + a random `state`), captures the redirect on a
local loopback listener
(`http://127.0.0.1:<redirect_port>/callback`), validates the returned
`state`, exchanges the code for the initial access + refresh token, and
stores the refresh token in the host token store. A missing or expired
consent surfaces as a plain-language mailbox row naming this exact command.
Client-credentials bindings need no login.

`grant` / `revoke` / `grants` manage the per-component OAuth binding ACL
(`system/oauth-grants.json`). A component must (a) declare the binding in
its sidecar `oauth_bindings`, (b) be granted it by the operator, and (c)
have a configured provider row in `system/oauth.yaml` before the host
serves it a token via `wild:oauth/token::get-token`. Design:
`docs/adr/0143-outbound-oauth-broker.md`.

### Connector conformance probe

```bash
wild connector test <tribe> <source-slug>        # dry-run enumerate only
wild connector test <tribe> <source-slug> --fetch-first   # also fetch the first item
wild connector test <tribe> <source-slug> --sample        # also map a sample row (setup proof)
wild connector test <tribe> <source-slug> --json   # compact JSON for jq
```

Read-only diagnostic for ONE declared intake source. Resolves the source
from the tribe's `flow` records, checks credential readiness (secrets +
OAuth), invokes the connector's `enumerate` verb, and optionally fetches
the first item. `--sample` additionally decodes a sample row through the
same probe the intake pipeline uses **and checks the columns it read
against the ones the declaration maps from** — the verify-before-done
completion proof (ADR-0152 D9): only a green probe WITH a successful
sample means the source is truly set up. Nothing is ingested or
persisted. Use it to prove a connector is reachable and correctly
configured before scheduling intake or after rotating credentials.
Returns success/failure, item count, first-item metadata, fault
classification, and elapsed ms.

`sample.status` is one of:

| Status | Meaning |
|---|---|
| `ok` | decoded, and every declared column is present in the file |
| `mismatch` | decoded, but declared columns are **missing** from the file — every file of this shape will be PARKED, not ingested. `missing_columns` names them. |
| `skipped` | no in-process blob reader — a host limitation, not a source fault |
| `failed` | the item could not be decoded at all; flips the whole report red |

`mismatch` covers the gap the ADR-0108 cross-layer gate structurally
cannot: that gate refuses a mapping reading an undeclared column, but it
never sees the FILE, so a declaration that is internally consistent and
drifted from the real header passes it, applies cleanly, and parks every
file it reads. It is not `failed`, because a source may declare several
shapes and one file match only one of them.

### Provisioning (ADR-0146)

```bash
wild provisioning ls [--tribe <tribe>]              # list installable/remediable items
wild provisioning invoke --tribe <tribe> --subject <subject>  # consent to install one item
```

Lists the unified provisioning surface: missing models, secrets, plugins,
connectors, OAuth bindings, or binaries that the daemon needs before a
capability works. `invoke` is the operator consent that starts the governed
install/download; progress is reported back through the same vitals channel.
Design: `docs/adr/0146-operator-provisioning-surface.md`.

### MCP server

```bash
wild mcp                                 # stdio MCP server (Claude Desktop subprocess)
wild mcp --transport http --bind <addr>  # HTTP transport (lean build defers to `wild up --mcp-http`)
wild config mcp setup                    # print a Claude Desktop mcpServers snippet
```

### Bearer token (REST + MCP)

```bash
wild config token rotate                 # rotate the shared bearer token (system/token, mode 0600)
wild config token show                   # print the token file path + mode (bytes redacted)
```

The bearer token in `<profile>/system/token` is **transport-neutral** —
the REST (`wild up --rest-http`) and MCP HTTP transports resolve the
same secret. It is the single all-powerful operator bearer; scoped,
named access is `wild user` (below).

### Named users (ADR-0115)

```bash
wild user add <id> --operator             # named operator: full surface, all tribes
wild user add <id> --tribe acme --tribe eu  # scoped user: role editor, only these tribes
wild user add <id> --tribe acme --role viewer  # read-only in a tribe
wild user add <id> --customer --tribe acme  # ADR-0154 "invite user": an app end-user (customer-mode, viewer by default; use --role editor for write-effect apps)
wild user add <id> --operator --attr language=de-DE  # descriptive preference(s); repeatable
wild user ls                              # id · role · tribes · token fingerprint (never the secret)
wild user rotate <id>                     # new token (prints once); old one dies immediately
wild user rm <id>                         # revoke (effective without a daemon restart)
wild user attr <id> --attr language=de-DE --unset timezone  # change an existing user's preferences (no args = show)
```

Manages the per-user identity map `<profile>/system/tokens.toml`
(`[[user]]` token → role + tribes + groups + attributes). `--attr
key=value` (repeatable, ≤16 pairs, values ≤256 chars, no control
characters) attaches DESCRIPTIVE preferences — `language`, `timezone`,
`tone`, … — an open set the chat reads to adapt (e.g. the
Elder answers in the user's declared language); attributes are never
authorization (roles/tribes/groups stay the gated fields) and never
SECRETS — they ride message headers and render into the chat prompt, so
treat them as visible to every host-side consumer. Tokens are always
**generated** (never caller-supplied) and stored **hashed**
(`sha256:<hex>`) — `add`/`rotate` print the secret **once**; it cannot
be recovered. A tribe-scoped user is confined to the listed tribes on
the operator REST surface and denied the raw tool-dispatch plane; an
operator is a named, auditable stand-in for the shared token. Edits
take effect without a daemon restart.

### Profiles / connection

```bash
wild profile list                        # list connection profiles
wild profile use <name>                  # switch the active profile
wild profile add <name> …                # add a profile
wild profile remove <name>               # remove a profile
```

### Setup / dev

```bash
wild install-deps                        # pre-fetch the pinned nats-server into ~/.wild/bin
```

`--help` on every subcommand prints the canonical flag list.

## Chat REPL cheatsheet

`wild chat` (or bare `wild`) opens an inline conversational REPL —
plain stdout, native scrollback, Cmd-C / Cmd-F all work. Single
addressee per session; switch with `/elder` / `/tribe <id>`. Type
your message, hit Enter, the assistant streams its reply inline.

### Slash commands (chat REPL)

| Slash | What |
|---|---|
| `/help`, `/?` | Show every command + one-line description |
| `/exit`, `/quit`, `/q` | Leave the chat (Ctrl-D / EOF also works) |
| `/welcome` | Re-print the welcome block |
| `/elder` | Switch addressee to Elder (root system tribe) |
| `/tribe <id>` | Switch addressee to a tribe |
| `/tribes` | Snapshot list of registered tribes (KV bucket) |
| `/health` | Latest boot-health report |
| `/activity [N]` / `/activity follow` | N most-recent operator events / live-tail |
| `/bus [N]` / `/bus follow [pattern]` | N most-recent NATS msgs / live-tail (Ctrl-C exits follow) |
| `/blueprint show` | Read-only inline blueprint print |
| `/blueprint edit` | Suspend → `$EDITOR` → write back → reboot trigger |
| `/blueprint reboot` | Publish a Reboot trigger without changing content |
| `/diff <key>` | Unified `git diff`-style diff between the two latest revisions |
| `/upload <path>` | Upload a local file to the active tribe |
| `/copy` | Copy the last reply to the system clipboard |
| `/save <path>` | Write the session transcript as Markdown |
| `/!shell-cmd` | Run a shell command without leaving the chat |

## Watch console cheatsheet

`wild watch` is the full-screen monitoring dashboard — five panes
(Activity, Tribes, Inbox, Bus, Forge), no input field, no chat surface. Operators
typically run it in a second terminal tab next to `wild chat`.

| Key | What |
|---|---|
| `Tab` | Cycle focus through Activity → Tribes → Inbox → Bus → Forge |
| `F2`–`F6` | Jump focus directly (Activity · Tribes · Bus · Inbox · Forge) |
| `PgUp` / `PgDn` | Scroll the focused pane |
| `Ctrl-Home` / `Ctrl-End` | Jump to top / tail of focused pane |
| `↑` / `↓` | Move Tribes cursor (Activity / Bus: scroll one row) |
| `q` / `Esc` / `Ctrl-C` | Quit, restoring the terminal |

## When to use which

A few worked examples:

- **"Add an LLM adapter for the team."** → CLI.
  `wild config llm add --kind component --slug openrouter --id team-or`
  runs in your provisioning script (`--kind component` requires the
  backing plugin `--slug`; a freshly added component adapter wires at
  the next daemon boot). Both `wild watch`'s Activity pane and
  `wild chat`'s `/activity follow` show the config-change event.

- **"Talk to a tribe about its current findings."** → chat REPL.
  `wild chat --tribe q3-sales`, type. Multi-turn, references the
  conversation history, supports `/save` for archiving.

- **"CI watches the bootstrap lock."** → CLI.
  `wild bootstrap list --json | jq .`

- **"Inspect why a tribe is stuck."** → watch console for live look,
  CLI for scripting. `wild watch` shows lifecycle events + heartbeat
  freshness across all tribes; `wild metrics --json` is the
  scriptable equivalent for an alert.

- **"Approve a forge-built component-type."** → CLI verb.
  `wild component-type approve <name>` works from a script and from
  the chat shell via `/!wild component-type approve <name>`.

## What about `wild chat <subcommand>`?

`wild chat {resume, new, ls, close, ask}` are **session-management
verbs** for scripting. `resume` / `new` / `ls` / `close` operate on
the persisted SQLite session store; `ask` does a one-shot publish +
await against the Elder / Domain-Elder operator-chat lane, same as the
REST `/api/v1/chat/ask` twin (`wild chat ask "deploy …"
--json | jq` for bash scripts).

The interactive REPL is bare `wild chat` (no subcommand) — the
default action. See `docs/inline-chat-design.md`
for the full design.

## Read more

- ADR-0010 — the original split
  rule with rationale + migration plan.
- `docs/inline-chat-design.md` — chat REPL
  design (slash matrix, streaming, session state).
- `docs/watch-dashboard-design.md` —
  watch console design (3-pane layout, Forge-pane extension shape).
- ADR-0009 — why config CLI
  verbs are RPC clients.
- `docs/elder.md` — how chat REPL talks to
  the Elder.
- `docs/development.md` — adding a CLI verb or
  chat slash command.
