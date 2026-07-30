# Liquidity Management — DDD Reference Tribe

The **domain-model-first** reference example (ADR-0108). The liquidity
domain (§17 InsO, 13-week forecast, gated payment + dunning verbs),
authored the DDD way: one `ontology/model.yaml` is the tribe's **constitution**,
and `wild tribe apply` *compiles* it — deterministically, no translation — into
the full ontology.

> **This is the canonical authoring example.** If you are building a new tribe
> today, start here.

## Apply it

```bash
wild tribe apply examples/tribes/liquidity-management/ --as acme-liquidity
```

One command: parse + gate the model, compile it, pin the ontology, ingest the
seed data, register the recurring feeds, stamp the tribe `ddd`, and schedule a
live chief. Then talk to it:

```bash
wild --profile <name> elder      # > "Which debtors are overdue?"
```

## What makes it the DDD lane

The switch is one line in `manifest.yaml`:

```yaml
authoring_method: ddd
```

At that key, `wild tribe apply` routes through the domain-model compiler instead
of the hand-authored loader:

1. **Parse + gate up front** — `compile_ddd_source(ontology/model.yaml)` parses
   the `DomainModelDoc` and runs the cross-layer gate (every source
   `target_type` must name a declared aggregate) **before any side effect**. A
   broken model fails loudly with zero partial state.
2. **Compile, don't translate** — the model lowers mechanically to the full
   `DataCommand` stream (types + value objects + functions + verbs + sources).
   An LLM (or you) authors the model; the transform is deterministic.
3. **Persist + stamp** — the model is written as the tribe's source of record
   (`tribes/<slug>/ontology/model.yaml`) and the tribe is stamped `ddd` in its
   `meta.yaml`. From then on the tribe is a *living* DDD tribe: model changes go
   through the `ddd_declare_*` chat tools (fold → gate → compile → persist), not
   hand-edits.
4. **Strip the hand-authored ontology** — the model owns types · value objects ·
   functions · verbs · sources · intake, so any stray hand-authored ontology
   file is ignored (this bundle ships none).

## The model — `ontology/model.yaml`

The single constitution, in two layers (ADR-0108 §3):

| Layer | Section | Lowers to |
|---|---|---|
| **A — Domain** | `value_objects`, `value_sets` | `money_amount` (EUR), `iban` (mod-97), `partner_role` (closed set) |
| | `functions` | the reusable pure expressions (`@vat_amount`, `@cash_discount_amount`, …) |
| | `aggregates` | the type vocabulary — mirror/state/derived/enriched fields, relations |
| | `aggregates[].lifecycle` | the closed `status` enum **and** each command's `requires_state` guard, from ONE source (the transitions) |
| | `aggregates[].commands` | the gated domain verbs (risk tiers) + per-verb `requires_state` preconditions (object-state guards, enforced by the gate, advertised on `describe`) |
| | `projections` | the CQRS `debtor` read-model (grouped over `invoice`) |
| **B — ACL** | `sources` | the recurring `intake_source` bindings + column→field `intake_config` |

Presentation and template marks (icons, colours, categories, `title_key`,
scaffold hints, field descriptions, `sensitive`/`customer_visible`) are authored
intent the compiler cannot infer — they ride explicitly on the model and land on
the compiled types.

## Conditional effects — `requires_state` on the invoice verbs

The invoice verbs demonstrate **conditional effects**: a verb only fires when
the invoice's CURRENT record satisfies a guard. The guard is declared on the
verb (`requires_state: {field: predicate}`), enforced **centrally** by the
effect gate before the handler runs, and advertised on `describe(invoice)` —
so the condition lives in the API layer, not inside the effect (which stays
reusable across record shapes):

| Verb | Fires only when | Refused when |
|---|---|---|
| `take_cash_discount` | `direction == inbound` **and** `status == open` | a receivable (outbound) or an already-settled invoice |
| `reconcile_transaction` | `status != paid` | the invoice is already settled (no double-settle) |
| `write_off_invoice` | `status != paid` | the invoice is already settled |
| `verify_invoice` | *(any state)* | — guards are opt-in, not blanket |

The bounded predicate DSL carries one predicate per field (`==`/`!=`/`nonempty`
/ numeric); "still owed" reads honestly as `!= paid`. Create verbs
(`dun_invoice`, `schedule_payment`) mint a new subject, so `requires_state`
cannot read the *invoice* there — their conditions live with the caller (the
`receivables-overdue` flow reacting to the daily aging write, or the chief's
judged backstop) and on the risk gate, not on the verb.

## ADR-0118 flows — the `processes:` section

> [ADR-0118](../../../docs/adr/0118-composable-pipelines.md) compiled flow records
> (the engine substrate — the subject graph with stages, ports, terminals) are
> surfaced as **flows** at the authoring and operator layer — the word the
> `flow_declare` / `flow_feed` verbs and the dashboard use. You author a
> `process`; it compiles to a flow. This section speaks *flow*.

The model's `sources:` compile to *length-1* flows. The model's
**`processes:`** section (ADR-0118 **D14**) compiles each constitutional
"when X → do Y" rule to a flow — a reactive (`new`/`change`) process to a
length-1 reactive flow the store fires on the write, a `source`/`handoff`
process to a multi-stage flow. The seven processes here exercise all four
trigger kinds, the D10 branching flow, the ADR-0126 Phase 3 document
connector, AND two ADR-0128 Join/Barrier gathers (one per lane: the connector
batch-collapse and a reactive fan-out fold), authored the DDD way
(`ddd_declare_process`, or hand-written in `model.yaml`) and lowered
deterministically by the compiler, not scripted at runtime:

- **intake, gathered (connector lane)** — `invoices-inbound` extends its own
  folder feed (`on: {source: …}`, the D14 takeover: process slug == source
  slug, so the multi-stage record replaces the length-1 ingest fold). After
  the auto `ingest` step, every partial run of one pull arrives at an
  ADR-0128 **`gather`** barrier (`join:gather` edges) and ONE
  `notify-operator` notice goes out per pull — N per-file runs fold to one
  with fan-out on (`WILD_CONNECTOR_FANOUT_MAX > 0`), the single batch run
  folds alone on the sequential path; the notice body renders the barrier's
  own `arrived/expected` stamp either way.
- **intake, extracted + enriched (document connector)** — `invoices-inbound-pdf`
  (ADR-0126 **Phase 3**) is the same D14 takeover, but over a DOCUMENT source
  that accepts both PDF and DOCX scans from `documents/inbound-invoices`:
  3 PDF samples and 5 DOCX samples ship with the tribe.
  An `extractor:` block makes the source fold `[ingest, extract,
  ingest-candidates]`, and the same-slug `source` process inserts an ENRICH
  step, so each scanned file walks its own run `ingest → extract → enrich →
  ingest-candidates`. The `extract` step is the built-in agentic `extract-worker`
  (one candidate invoice per file); the `enrich` step is a deterministic
  `lookup` that merges the creditor's payment terms from the partner master
  (zero LLM turns); the candidate is then admitted through the confirm gate.
  The operator authors the per-connector intent; the extraction complexity
  stays under the waterline.

  ADR-0089 full-text indexing is turned on for this source:
  `index_corpora: [documents]` feeds every PDF/DOCX into the search corpus, and
  `index_options.per_page: true` makes the `search-indexer` emit one corpus entry
  per page rather than one per file. That lets an operator search inside
  multi-page scans and land on the exact page — e.g. "show me invoices
  mentioning 'Net 14' on page 2".
- **external data, two acquisition shapes (fetch vs. intake)** — a debtor's
  external creditworthiness (`credit_assessment`) is acquired two ways, side by
  side, to make the point honest: `partner-credit-lookup` (credit inquiry) is
  the CLEAN structured read (`does: fetch` a cross-tribe `pipeline:credit-bureau/
  partner-credit-check` offer), but that only works when the bureau is itself a
  Wild tribe — the EXCEPTION. `partner-credit-report-pdf` (credit report) is
  the REALISTIC path: a Schufa/Creditreform result arrives as a PDF or DOCX and
  the rating is pulled out of it — a DOCUMENT source whose `extractor:` concretely
  lands the `credit_assessment` datum through the confirm gate (the fetch is the
  clean-case alternative that would supply the same datum). Document intake, not
  a fetch, is where operators actually spend their effort (ADR-0176 Context). No
  sample credit-report documents ship (a real one is a third-party document); the
  source is declared and armed, empty until one lands. It uses the same ADR-0089
  full-text indexing as the invoice scan, so any incoming report becomes
  searchable per page.
- **screening, branching (reactive lane)** — `invoice-screening`
  (`on: {new: invoice}`) is the record-granular branching flow: each FILED
  invoice walks its own run (ADR-0118 PR-12), so a deterministic `check`
  step (`screen`, no worker) routes on the invoice's OWN fields over an
  **exclusive port** (D10, first match wins): a cancelled re-ingest halts at
  a `stop:` terminal; a cash-discount-window-open payable is flagged
  (`done: flagged-cash-discount`); everything else is filed straight away — all
  deterministic, no worker spent on routing. (No overdue edge: an arriving
  invoice's source status is never `overdue` — aging is time, and time
  enters as the daily aging WRITE; ADR-0156 OQ5 / ADR-0157.) The two lanes
  are the deliberate granularity split: per-PULL/per-FILE work on the
  source process, per-RECORD routing on the reactive process.
- **worker (payables→receivables loop)** — the chase is TRIGGERED since
  ADR-0157: the invoice's declared `aging:` rule makes the daily host tick
  write `effective_status: overdue` on each past-due open receivable (a
  governed, actor-stamped write), and the `receivables-overdue` process
  (`on: {change: {aggregate: invoice, field: effective_status, becomes:
  overdue}}`) dispatches `receivables-chaser` (one agentic `review` step)
  on that write. The blueprint's `receivables-chased` judge (ADR-0049)
  stays as the agentic BACKSTOP over this deterministic lane. The new
  dunning then fires `dunning-escalation`
  (`on: {new: dunning}`) — since the ADR-0124 proof point a **deterministic
  multi-stage flow**: `lookup` merges the debtor's `telegram_chat_id` onto the
  dunning, `template` renders the notice from the level (no LLM prose),
  `telegram-send` delivers it, and the ADR-0127 `on_done` write records
  `status: sent` — zero LLM turns for the routine notice. A missing debtor or
  chat id stops audited (`no-debtor` / `no-recipient`); only an out-of-scale
  `level > 3` reaches the agentic `dunning-notifier` (which is also the v1
  fallback on a runtime without the multi-stage walk). The store fires both
  processes on the write (replacing the old `on-upsert: dunning` spec).
- **logic-first decision (ADR-0144 dogfood)** — `invoice-dual-check`
  (`on: {new: invoice}`) decides an invoice's validity from its OWN fields with
  **zero workers and zero LLM turns**: a deterministic `map` step (`validate`)
  adds `reference_ok` / `amount_ok`, then a `does: decide` step (`triage`) runs
  an ordered rule table over those fields and stamps a top-level `resolution`
  the edges route on — a bad reference or amount halts at `stop:
  invalid-invoice`, a clean invoice folds to `done: checked`. The routing/triage
  decision that used to be wired to an agentic worker is a rule over fields, so
  it is logic; the flow arms with no deployed component (the `decision` host
  built-in resolves directly at D16). (The ADR-0128 join/barrier fold is shown
  by `invoices-inbound`'s `does: gather` batch-collapse.)
- **effect** — `dunning-notice`: `on: {handoff: …}` → a length-1 `Sink-Effect`
  over the Telegram sender, offered to a partner tribe with a cross-tribe Grant.

These are Layer B of the one constitution — read them in
[`ontology/model.yaml`](./ontology/model.yaml) under `processes:`. `wild tribe
apply` compiles them alongside the ontology; the compiled records carry
`origin: model` (the reconciler owns them, the same as the source folds).

## ADR-0185 flows — the `flows:` section

> [ADR-0185](../../../docs/adr/0185-generic-flow-builder.md) adds a second Layer-B
> authoring lane for flows. A `processes:` entry (above) uses the `does/using/
> steps` sugar — a business rule that *reacts* to a domain event. A **`flows:`**
> entry is a `FlowDecl` written **directly** — the exact shape an operator grows
> in Elder chat with `flow_declare` / `flow_edit`, or edits in the dashboard Flow
> editor. Both compile to `origin: model` pipeline records and share the
> pipeline-record slug namespace (a collision fails the gate). `wild flows ls`
> lists every flow; the dashboard draws each as a Flow node with a "▸ n steps"
> badge.

The three flows here exercise the ADR-0185 **D3 presets** end to end:

- **`payable-cash-discount-review`** — *Decide → Act*. `map` (compute the discount) →
  `decision` (an exclusive-port rule table: is taking the cash discount worthwhile?) →
  `notify-operator` only when it is; otherwise an audited `terminal:abort`. Zero
  workers, zero LLM.
- **`sepa-payment-export`** — the ADR-0184 **`invoke-tool`** envelope. A
  deterministic Transform addresses an external `sepa_xml_export` tool by name,
  `args` validated against the tool's *own* schema (opaque to the core; an
  unknown tool aborts audited at run time, never at compile), then notifies the
  operator that the file is ready to sign.
- **`multi-invoice-scan-split`** — the full *Capture → Extract → Confirm → File*
  chain: `render-document` → `detect-document-boundaries` → a `join`/`collect`
  gather → an agentic `extract-worker` → a `confirm` pause (human in the loop) →
  a `store` sink-data. The manual sibling of the `invoices-inbound-pdf`
  connector takeover, but with an explicit operator confirm before filing.

## Demo status — what's live vs. stubbed

This example teaches the *shapes*; a few stages reach for a real integration the
example does not ship, so they are **demo stubs**. Each stub carries a `note:`
that the dashboard renders as an amber "⚠ Stub: …" line **under the flow node**,
so an operator reads what *would* happen instead of guessing at a dead end. What
each stub needs to become live:

| Flow / door | Stub stage | Needs to run for real |
|---|---|---|
| `partner-credit-lookup` (credit inquiry) | `assess` — cross-tribe `fetch` | a deployed `credit-bureau` tribe exposing the `partner-credit-check` offer. The realistic alternative — `partner-credit-report-pdf` — is live once a PDF lands. |
| `partner-credit-report-pdf` (credit report) | the document extractor | a credit-report PDF in `documents/credit-reports` (none ship — a real one is third-party) + an LLM for extraction. |
| `dunning-escalation` / `dunning-notice` | `send` — `telegram-send` | a Telegram bot token + egress binding (see `manifest.yaml`). Without it the send is a no-op in the run rows. |
| `dunning-escalation` / `receivables-overdue` | `judge` / `chase` — agentic workers | an LLM model pinned (`workers[i].model` or `manifest.default_model`; `wild tribe apply` warns when none is set). |
| `sepa-payment-export` | `build-file` — `invoke-tool sepa_xml_export` | a tool provider that ships the `sepa_xml_export` tool. Until then the stage aborts audited at run time. |

Everything else — the CSV intakes, the deterministic `map`/`template`/`lookup`/
`decision`/`store`/`gather` stages, the aging tick, the confirm gate — runs with
no external dependency.

## The operator app — `apps/liquidity-cockpit.yaml`

The published end-user app (ADR-0154), a composition of **views** over this
ontology. It exercises all **eight** built-in view kinds — `kpi-tile`, `chart`
(incl. a multi-measure debtor-exposure chart and a 13-week time-bucket window),
`table`, `detail-card` (with the D11 change-history trail + D16 cross-view record
wiring), `effect-form`, `chat-panel`, plus the ADR-0175 G13 `search` and
`timeline`/kanban — and the ADR-0175 feature surface: a responsive **grid layout**
(G9), **audience** gates (G6), **onboarding** / **empty-state** cards (G14),
design-token **branding** (G11), and the time-driven **gantt** / **calendar**
table representations (D11). Validate it with `wild app validate
apps/liquidity-cockpit.yaml`.

## CRUD effects — the `crud/` script (ADR-0083)

Generic **create / read / update / delete** can be exposed on a type as
governed, entity-scoped effects that act exactly like the bespoke verbs. They
are **materialised, never authored** (ADR-0083) — the compiler emits none from
`model.yaml` — so, like the operator flows, the reproducible artifact is a
post-apply bind: [`crud/apply-crud.sh`](./crud/apply-crud.sh) binds
create/update/delete on **`dunning`** (`crud_bind`), giving the dashboard detail
card a **"+ Add new"** action and the domain surface `dunning_create` /
`dunning_update` / `dunning_delete`. `read` is auto-bound (reads stay free);
`crud_unbind` retires an effect (→ internal-only).

`dunning` is an **authored** aggregate, so a hand create/delete is meaningful.
`invoice` deliberately gets **no** create effect — it is a `source_mirror` whose
records the ingest feed owns, so "create an invoice" is correctly refused (the
feed, not an operator verb, mints invoices). Who may invoke a CRUD effect derives
from the bound type's live `customer_visible`. Since ADR-0178 the operator can
also drive this in Elder chat ("enable creating dunning notices").

## Correctness — the compiler gate

The claim "the model compiles to a correct ontology" is not a promise, it is a
**test**: the compiler tests in `wild-ddd` (`type_lane_*`, `verb_lane_*`, and
their value-object / function / source / mapping / flow / process siblings)
compile this model and assert the emitted records against committed goldens — so
a compiler regression reddens a test rather than shipping a broken tribe.

## Lane-independent assets

The domain knowledge is carried by the model; the rest of the bundle is
lane-independent (copy the whole folder as a template):

- `blueprint.md` — the deployed chief's system prompt (mission).
- `specs/` — `domain.md` (framework knowledge, §17 InsO, KPIs), `onboarding.md`
  (priority stack), `sketch.md` (mission narrative). The dunning reaction is no
  longer a spec trigger — it is the `dunning-escalation` **process** in the
  model. There is **no** `specs/domain-model.yaml` — the real
  `ontology/model.yaml` *is* the model.
- `data/` — the CSV starter seed + intake mappings (one-shot, lane-independent).
- `documents/` — sample inbound-invoice PDFs for the document door.
- `workers/dunning-notifier.md` — the Telegram dunning worker (ADR-0098), now
  the judgement (`level > 3`) + v1-fallback step of `dunning-escalation` (the
  routine notice is deterministic since ADR-0124); `workers/receivables-chaser.md` — raises
  the first-level dunning when the `receivables-overdue` flow (or the chief's
  judged backstop) dispatches it (atlas-only, no egress). The `invoice-dual-check` triage is a `does: decide` rule step
  (logic, no worker), so it ships no `workers/*.md` — see ADR-0144.
- `ontology/enrichment_rules/` — the ingest-boundary enrich rule (a pass-through
  lane the compiler does not emit).

## Try it in a probe session

1. **P1 guidance** — the chief opens with a bank-data ask, not a blank prompt.
2. **Off-mission rejection** — hand it fitness data → it explains why it doesn't
   fit and names what it needs.
3. **Grow the model in chat** — describe a rule the model doesn't carry (a new
   invoice state, a new source) → the chief offers a `ddd_declare_*` and grows
   the constitution through the gate, never a raw schema write.
