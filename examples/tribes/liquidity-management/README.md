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

### The sample data has a fixed "today"

The seed is cut around **2026-07-31**: four receivables already overdue (the
dunning chain works on those), the rest falling due across the following
thirteen weeks, and bank movements running to two days before. That shape is
what makes the demo readable — a forecast with something in it, an overdue
bucket that is not everything.

It is also perishable. Run this bundle a year later and every open item is
overdue, the forecast window is empty, and the aging tick has nothing left to
demonstrate. Re-cut it: move `TODAY` in
[`data/regenerate-seed.py`](./data/regenerate-seed.py) to the current date and
run it. That is the whole chore — the report's forecast window anchors on
`today` and follows the clock by itself.

You will not have to remember: `example_seed_freshness` (in `wild-ddd`) reads
these CSVs against the real clock and reddens once too few open invoices fall
inside the next 13 weeks, or once the overdue set stops being a few and starts
being the whole book.

Edit the CSVs by hand and you inherit the drift the generator exists to
prevent — the previous cut had payments dated before their invoice, three
debtors paying someone else's bill, and a balance that tied to nothing.

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

## Settled and late are two facts

An invoice carries two verdicts of its own, and keeping them apart is
deliberate:

| field | answers | members | who writes it |
|---|---|---|---|
| **`settlement`** | **is it settled?** | `open` · `paid` · `written_off` | derived — read THIS one |
| `status` | what the feed says | `open` · `paid` · `cancelled` | the accounting export |
| `effective_status` | did the tribe override the feed? | `from_source` · `paid` · `written_off` | the tribe's verbs |
| `aging_state` | is it late? | `current` · `overdue` | the daily aging rule |

`settlement: open` means **still owed, past-due included** — which is what an
operator means by "offene Forderungen". Every measure and every grouped read
binds `settlement`, never one of its two sources: the feed alone misses a
write-off the tribe decided, and the override alone reads `from_source` on
everything nobody has touched.

Two corrections got the model here, both found by measuring rather than
reasoning, and both worth knowing before editing this file:

**The axes were one enum.** `effective_status` used to carry an `overdue`
member, so ageing moved a receivable OUT of `open` — every reading of the field
was correct while the label lied, and a grouped read returned an `open` bucket
that silently excluded exactly the invoices that need action.

**The override claimed a state it was never given.** Its initial member was
`open`. An ingest may never write an authored field, so the fold seeds the first
member on *every* imported invoice — and that seeded `open` then beat the feed's
own `paid`. Naming it `from_source` is what makes the field honest: it says the
tribe passed, not that the invoice is unpaid.

Together these produced the wrong total about one turn in five against a live
Elder (`open-receivables-is-the-declared-measure`); grouping outbound invoices
by `settlement` now returns `{open: 13, paid: 5}` summing to the same
135.800,00 EUR the declared measure reports.

Two consequences worth knowing:

- A measure that wants the LATE unsettled ones names **both** axes — a
  `filter:` list ANDs. `aging_state` alone would keep counting an invoice that
  was paid late.
- Settling does **not** reset `aging_state`, on purpose: "which invoices were
  paid late?" is a question the merged enum destroyed and this one answers.

Upgrading an already-deployed copy: `effective_status` retired both `overdue`
and `open`, so old rows keep values the schema no longer declares. This example
ships no migration — re-apply onto a fresh profile.

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

The `payment` lifecycle guards (`approve_payment`: planned → approved;
`pay_invoice`: approved → executed) need a record in the state they demand, so
`data/payments.csv` seeds all three: two `executed` payments matching the
settled payables and their bank debits, one `approved` awaiting execution, two
`planned`. A verb whose only demonstrable behaviour is refusing on an empty set
teaches the refusal, not the verb.

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
  write `aging_state: overdue` on each past-due open receivable (a
  governed, actor-stamped write), and the `receivables-overdue` process
  (`on: {change: {aggregate: invoice, field: aging_state, becomes:
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

## The operator apps — `apps/`

Two published end-user apps (ADR-0154), because they answer different
questions. `wild tribe apply` installs both into the profile's app plane
(`<profile>/apps/`) and stamps each copy with the tribe you applied as.

Neither spec in the BUNDLE declares a `default_tribe` — absent, a view binds
the carrier, so the bundle stays portable under `--as <any-id>`. The installed
copy does carry it, because a profile-level app sits under no tribe and would
otherwise bind nothing. Re-applying refreshes an app you have not touched and
keeps one you have edited (saying so either way).

**`liquidity-report.yaml` — the accountant's page.** Cash on hand, how many
invoices are open, how much is already overdue, what is still expected in —
then the next 13 weeks in and out, and the accounts and bank movements those
figures fold from. One page, no schema tour. Start here when you want to see
what the tribe *knows*.

The "expected in" tile carries an ADR-0189 D7 **sparkline**: the same measure
folded into 13 weekly buckets along `due_date`. It is the only tile on the page
that can, and that is an ontology fact — a trend buckets its measure over a date
field OF THE SAME PROJECTION, and of the four read-models the report binds only
`cash_forecast` has one. Read the halves apart: the figure sums every row of the
projection, the sparkline shows only the window.

Its 13-week curve is the ADR-0154 PR3d **`window:`** block: the chart's read
becomes a `TimeBucket` fold over 13 consecutive 7-day windows along the
invoices' `due_date`. `window.start` is `today` — an INTENT, not a date: the
renderer stays clock-free and the HOST resolves the token where the read is
served (`wild_core::window_start`), so the window is always the next 13 weeks
and never needs re-cutting. An absolute ISO date still works, and is the right
choice when a window names a real period (a fiscal year, an audit range).

**`liquidity-cockpit.yaml` — the working surface, and the schema showcase.** It
exercises **eight of the ten** built-in view kinds — `kpi-tile`, `chart`,
`table`, `detail-card` (with the D11 change-history trail + D16 cross-view
record wiring), `effect-form`, `chat-panel`, plus the ADR-0175 G13 `search` and
`timeline`/kanban. The two it does not: `diagram` (ADR-0189 D8 — authored
structure, bound to no data) and `custom` (ADR-0161 — needs a widget plugin the
`WidgetCatalog` does not carry here). It also shows the ADR-0175 feature
surface: a responsive **grid layout** (G9), **audience** gates (G6),
**onboarding** / **empty-state** cards (G14), design-token **branding** (G11),
and the time-driven **gantt** / **calendar** table representations (D11).

Its two debtor charts are deliberately two, not one two-measure chart: the read
plan folds `y[0]` and only `y[0]`, and the renderer draws a single series — a
second measure parses and then never reaches a pixel. Open exposure and the
overdue slice therefore get a chart each, in their own ADR-0209 S1 palette
token. The cockpit also takes the `compact` density preset where the report
stays `normal` and the fitness sample's client app takes `comfortable`: one
preset per surface, each multiplying the same spacing ladder.

Validate either with `wild app validate apps/<name>.yaml`. Both are also
checked in CI: `example_bundle_apps_bind_to_their_compiled_model`
(`wild-mcp-tools`) compiles this bundle's `model.yaml` and runs every app spec
in `apps/` through the publish-strength gate, so a view naming a projection,
measure or effect the model does not declare reddens a test instead of
surfacing at an operator's `app_publish`.

### The read-models a report needs

A `kpi-tile` and a `chart` bind a **projection**, never an entity, and a tile's
read is a SUM of the measure across the projection's rows. So a headline figure
must exist as a measure in `model.yaml` before any app can show it — four do,
beside the older `debtor` and `cash_flow_13w`:

| Projection | Grouped over | Answers |
|---|---|---|
| `cash_position` | `bank_account` by currency | how much cash is actually on hand |
| `invoice_book` | `invoice` by direction | how many invoices, how much open, how much overdue — both sides |
| `cash_movement` | `transaction` by direction | money in vs. money out (the filtered sum `cash_flow_13w` cannot express) |
| `cash_forecast` | open `invoice` by due date | the forward curve the 13-week window folds |

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
  Generated, not typed: bank balances fold from an `opening_balance` plus the
  movements, every settled invoice has a matching bank booking with the right
  counterparty and IBAN, and every IBAN carries a valid mod-97 check digit —
  so the reconciliation story the verbs teach actually reconciles. `amount` on
  a transaction is UNSIGNED, as the model declares; `direction` carries the
  sign. The dates are cut around a fixed "today" (see below).
- `documents/` — sample inbound-invoice scans (3 PDF, 5 DOCX) for the document
  door. `wild tribe apply` DELIVERS them into the tribe's file store, so the
  connector has something to read the first time it ticks; a per-tribe ledger
  makes that a one-shot, so a scan you processed and deleted does not come
  back on the next apply.
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
