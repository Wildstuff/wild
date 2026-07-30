# Examples

Worked examples, grouped by what they are. Two axes live here: things
you **deploy** (tribes) and things you **build** (components + the host
itself).

```
examples/
├── tribes/          # deployable tribe bundles + templates
├── packages/        # ADR-0156 domain packages (declarations-only bundles)
├── tool-providers/  # components that expose tools to workloads
├── consumers/       # source-free builds against the published wild:* WIT
└── embedding/       # downstream binaries that embed wild-hostd as a library
```

## tribes/

Pre-fabricated tribes you can apply to a fresh profile to see
end-to-end behaviour without writing a manifest from scratch.

| Bundle | What it does |
|---|---|
| `math-tribe/` | Smallest useful tribe — chief + one worker, multi-path tool use. End-to-end smoke fixture. |
| `news-pipeline/` | Hourly Hacker-News pull → two workers produce a B2B brief and a microfiction vignette from the same feed. |
| `liquidity-management/` | **DDD reference** (ADR-0108) — the model-first liquidity tribe: one `ontology/model.yaml` is the constitution, `authoring_method: ddd` compiles it into the full ontology (types **and** the 8 gated verbs) plus the ADR-0118 process flows. Start here for new tribes. |
| `fitness-tracker/` | **Template** — authored (source-less) ontology: Exercise/Workout/Goal vocabulary the operator fills over time. |
| `web-crawler/` | ADR-0114 queue-fed intake: a governed crawl with zero crawler-specific core code. |

Two bundle shapes coexist:

- **manifest bundles** (`math-tribe`, `news-pipeline`, `web-crawler`)
  carry `manifest.yaml` + `workers/` + `blueprint.md` — the on-disk
  shape `wild tribe apply <dir>` consumes. Every such bundle parses
  against the current schema in CI (`crates/runtime/wild-tribe-ops/src/bundle.rs::
  tests::every_examples_bundle_parses_against_current_schema`).
- **DDD templates** (`liquidity-management`, `fitness-tracker`) carry
  `specs/*.md` + a single `ontology/model.yaml` (`authoring_method: ddd`) —
  `wild tribe apply` compiles the model and pins the ontology. See each
  template's README.

The directories `invoice-formats-branch/`, `invoice-scans-extract/`,
`notify-and-wait/`, `receipts-gather/`, and `receipts-review-gather/` are
intake-flow **fixtures** used by the E2E suite (`tests/e2e/tribes/`). They
are valid bundles, but they are not aimed at operators learning the system.

### blueprint.md in the prompt-layer model

A manifest bundle's `blueprint.md` is the **mission layer** for the
chief that runs the tribe. Three layers shape every chief, generic to
specific:

1. **Engine core** — `prompts/chief-default/*.md` (binary-shipped).
2. **Operator overlay** — optional `<bundle>/CHIEF.md` (planned).
3. **Mission** — `<bundle>/blueprint.md`. What *this tribe* is for.

Full reference: [`docs/prompt-layers.md`](../docs/prompt-layers.md).

### Deploying a tribe

```bash
wild profile new demo
wild --profile demo up &
wild --profile demo llm add claude          # the adapter the README names
wild --profile demo tribe apply examples/tribes/<name>/ --start  # apply AND run
# (omit --start to register it DORMANT, then `wild tribe activate <name>`)
wild --profile demo tui                         # watch it work
```

Tear down with `wild --profile demo down && wild profile delete demo --force`.

## tool-providers/

Components that implement the `wild:tool-provider` contract — they hand
tools to workloads.

| Component | What it shows |
|---|---|
| `sharepoint-connector/` | ADR-0141 PR1 existence proof: one component exercising all four installable compute primitives. Built with `cargo build --target wasm32-wasip2`. |
| `ts-tool-provider/` | The same `wild:tool-provider` contract authored in TypeScript (built with `bun` + `jco`), proving the contract is language-agnostic. |

## consumers/

Source-free builds against the published `wild:*` WIT — reference
callers that show how to consume a contract without the monorepo.

| Component | What it shows |
|---|---|
| `embed-consumer/` | Reference `wild:ai/embed` caller (text→vector). |

## embedding/

Downstream binaries that embed the host as a library.

| Binary | What it shows |
|---|---|
| `custom-daemon/` | A custom `wild-hostd` that injects its own native provider via `manager::extension`, WITHOUT forking bootstrap. A workspace member so CI compiles it on every change (the real guard against a silent break of the extension contract). |

## Adding a new example

1. Put it under the right category (`tribes/`, `tool-providers/`,
   `consumers/`, `embedding/`).
2. Give it a self-contained README: what it does, which adapter /
   secrets it expects.
3. Tribes: keep schedules conservative (hourly+) so an unattended demo
   doesn't run up an LLM bill; no hard-coded secrets (`wild secret set`).
4. Components: standalone Wasm crates stay out of the root workspace
   (built via `cargo build --target wasm32-wasip2`); the only workspace
   member is `embedding/custom-daemon` (it builds a native binary).
