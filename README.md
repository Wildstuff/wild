# The Wild

**The Wild** turns your data and a few sentences of intent into a
living system — one that understands your domain, works it
autonomously, and hands the people around you finished apps. No
code written by anyone.

It is not glue between your tools. Every **Tribe** The Wild
raises stands on infrastructure of its own: an embedded,
event-sourced object graph — entities fed from every source you
connect, full-text and vector search built in, every fact
carrying its provenance, every earlier state of knowledge
replayable in time. On that ground the Tribe *understands* your
data (**Atlas**), *works* the topic on its own (a **Chief** and
its **workers**), *forges* missing tools for itself (the
**Forge**), and *serves* finished apps to the people around you
(**appd**) — all of it operated in plain language, through
**Elder**.

> **Beta.** The Wild is pre-1.0 and moving fast. It already runs
> real desks and real kitchen tables; the polished, double-click
> macOS app is what the first public launch is aimed at. If you
> build software, start at
> [`docs/plugin-developer-guide.md`](docs/plugin-developer-guide.md).

<p align="center">
  <img src="docs/assets/wild-arc.svg" alt="The Wild — the arc: your folders, mail, and web sources become one entity graph on Atlas; a Tribe with its Chief, workers, and Forge works on that ground; the people around you get apps, you talk to Elder in plain language" width="920"/>
</p>

## Picture this

<p align="center">
  <a href="docs/diagrams/idea/scenes/03-folder-to-app-strip.svg"><img src="docs/diagrams/idea/scenes/03-folder-to-app-strip.png" alt="From a folder to an app, in six panels. One: you point at a folder of invoices, letters and contracts. Two: Elder asks what matters — due dates, notice periods, who may see them — then offers to spawn a Tribe. Three: Atlas pins the ontology, Partner with Contract and Invoice hanging off it. Four: a folder and a mailbox both resolve into one Partner, both receipts kept. Five: the Forge fills a gap with a parser that did not exist a minute ago. Six: your partner opens the app on their phone, and nobody wrote code" width="920"/></a>
</p>

You point The Wild at the folder where your household paperwork
lands — invoices, insurance letters, contracts.

**Elder** — the gatekeeper of the wilderness — listens. Asks what
matters: due dates? cancellation windows? who should see it? Then
offers to spawn a **Tribe**: a small autonomous community of
specialists that will own this topic from now on.

You agree. A connector mounts the folder, extraction reads the
PDFs, and **Atlas** — the Tribe's memory — pins the ontology:
Contract, Partner, Invoice, amounts, notice periods, relations.
You add your mailbox as a second source; the power bill arriving
by e-mail lands on the *same* Partner object as the contract
from the folder — one entity, two provenance lines. From now on
*"which contracts renew in the next sixty days?"* is a question,
not a query — and every answer names its source file.

The Tribe's **Chief** runs its cycles: new mail ingests itself,
the car-insurance cancellation window is flagged three weeks
ahead, a monthly digest arrives.

Then a provider switches to a scan format the extractor can't
read. The Chief notices the gap, walks over to the **Forge** —
the Tribe's smithy — and forges a parser: Rust source, built in
a sandbox, pinned by content hash, slotted in as a new tool. It
didn't exist twenty seconds ago. You did nothing.

*"Give my partner an app for our contracts."* Elder reads the
ontology and scaffolds one — a table, a detail card, a
cancel-reminder button — and **appd** serves it into your home
network. Your partner opens it on their phone. Nobody wrote code.

Now swap the household folder for your department's invoice
folder. Nothing else changes — the same system that runs your
kitchen table runs an accounts-payable desk: debtors over their
credit limit, a thirteen-week liquidity forecast, an auditor
asking what you knew on March 31st — answered from the event
stream, as of March 31st, sources attached.

That's The Wild. **A domain handed over. A system grown.**

## Why "Tribe"? Why "Wild"?

The names are load-bearing, not decoration. To *own* a topic —
not just answer prompts about it — takes everything a small
community of specialists has:

- a **shared memory** of everything it has ever seen, with every
  fact's origin — that's **Atlas**;
- a **shared understanding** of what exists and how it hangs
  together — supplier, invoice, due date, who relates to whom —
  that's the **ontology**;
- **members who act**, each with a craft, changing real things
  through governed, auditable effects — the **workers** and their
  verbs;
- a **smithy** for the day the work demands a tool nobody has —
  the **Forge**;
- and **one voice to the outside** that speaks your language —
  **Elder**.

Take any piece away and autonomy collapses into a chatbot with a
schedule. Together they are why a Tribe can be *handed* a topic
and trusted to carry it.

And "The Wild" is where such communities live. Software is
usually built like a machine — specified, shipped, frozen,
drifting away from the world from day one. Tribes are *raised*,
not built: they start small, learn the terrain, grow tools, and
adapt when the world moves. A wilderness, not a factory — that's
the bet in the name.


## Two showcases — both ends of the ladder

The entry path is deliberate: start at home, carry it into the
company. Two worked examples ship with the repo, one for each
end, each with a guided tour. Both are slated to become one-click
templates.

**Fitness (home).** Your own coach: you say *"lunch was 200 g of
chicken, I ran half an hour"* — it keeps the books, computes real
calories with receipts, looks up foods it doesn't know, and holds
one honest number against your goal. And it can tell you what
your balance was in February, from records — not from a chatbot's
vague memory.
**Guided tour:** [`docs/showcase-fitness.md`](docs/showcase-fitness.md).

**Liquidity (business).** A cash-flow desk that runs itself up to
the money line: overdue invoices get chased politely and exactly
once, early-payment discounts are found for you, a paid invoice
knows what paid it, thirteen weeks of cash are in the headlights —
and paying anything always remains your call. When the auditor
asks what you knew on April 30, the desk replays it, byte for
byte.
**Guided tour:** [`docs/showcase-liquidity.md`](docs/showcase-liquidity.md).

---


## Get The Wild

**macOS (Apple Silicon):** grab **`Wild.app`** from the
[latest release](https://github.com/wildstuff/wild/releases/latest)
— unzip, drag it into *Applications*, open. The app brings
everything with it: the runtime, the dashboard, and Elder chat.
Point it at a folder, answer a few questions, and your first
Tribe is running — the guided tours above make good first
challenges.

> **First launch:** the app is not yet signed with an Apple
> developer certificate, so macOS will refuse it once ("damaged /
> can't be opened"). Clear the quarantine flag one time in
> Terminal, then it opens normally — signing is on the launch
> list:
>
> ```sh
> xattr -dr com.apple.quarantine /Applications/Wild.app
> ```

Your data stays under `~/.wild` on your machine. The only traffic
that leaves is the LLM calls you configure — and with a local
model (the bundled `llama-server` lane) even that stays home.

**Linux, Intel Mac, or prefer the terminal?**

```sh
brew install wildstuff/tap/wild
# or
curl -fsSL https://raw.githubusercontent.com/wildstuff/wild/main/install.sh | sh
```

Both install `wild` (the CLI) and `wild-hostd` (the runtime daemon).
`wild up` starts it; `wild chat` talks to it.

## Not glue — infrastructure

Three anchors, depending on what you already know:

- *If you know agent frameworks:* a Tribe is an agent loop — but
  it stands on its own database, not a scratchpad. The event
  stream is the memory; throw away the prompts and the knowledge
  survives.
- *Palantir?* You may have heard the name — the company that
  pulls every scrap of an organization's data together so it can
  decide. That power is real, and it is what The Wild builds —
  but self-hosted, transparent, and sized for a normal person:
  it runs on your own machine, it shows you where every fact
  came from, and nobody sees your data but you. Your home folder
  today, your department tomorrow.
- *If you know app servers:* appd serves real apps to real
  people — but nobody wrote them; they are derived from what the
  Tribe knows.


## Concepts in 60 seconds

<p align="center">
  <a href="docs/diagrams/idea/scenes/04-the-cast.svg"><img src="docs/diagrams/idea/scenes/04-the-cast.png" alt="The cast, as six cards. Elder, one per install — the gatekeeper who onboards new Tribes and routes you between them. Atlas, one per Tribe — the ground everything stands on: your things, their relations, each fact with its origin. Chief, one per Tribe — reads the Tribe's charter, runs the cycles, decides what happens next. Worker, many per Tribe — the specialists doing the actual work, AI-, rule-, shell-, api- or human-backed. Forge, one per Tribe — the smithy that builds a missing tool, sandboxed and approval-gated. App, many per Tribe — what a Tribe hands the people around you, derived from the ontology" width="920"/></a>
</p>

A handful of nouns carry the whole model. Each links out to its
deep doc.

**Tribe** — the top-level unit. A clan with its own bus prefix
(`wild.{tribe}.…`), storage, secrets, and forge. Tribes can nest
(`acme-sales-eu` is a sub-tribe of `acme-sales`). See
[`docs/idea.md`](docs/idea.md) for the philosophical framing.

**Elder** — exactly one per install. The system-tribe orchestrator
(hardcoded into the `wild` binary). It onboards new tribes from a
chat, routes the user between them, and operates running tribes.
See ADR-0001 and
`docs/elder.md`.

**Chief** — exactly one per Tribe. The per-tribe orchestrator. Reads
the **Blueprint** (a Markdown charter), runs cycles, dispatches
workers, decides what to do next. The default chief ships embedded
in the binary (Tier-1.5 per ADR-0014); specialised flavors slot
into the same `wild:chief@0.1.0` contract. See
ADR-0002.

**Worker** — many per Tribe. Specialised inhabitants (Analyst,
Crawler, Writer, Triager, …) doing the actual work. Workers can be
ai-, rule-, shell-, api-, or human-backed. Today `ai-worker`
(Claude-driven) is the first production worker type.

**Atlas** — one per Tribe. The data ground: an event-sourced
object graph that turns raw intake (folders, mailboxes, web,
SharePoint) into an ontology — Types, Fields, Relations — with
full-text and vector search built in, provenance on every fact,
and the append-only event stream as replayable truth. Everything
else stands on it: workers read and write through it, apps are
derived from it. Concept tour with figures:
[`docs/ontology.md`](docs/ontology.md) · deep reference:
`docs/atlas.md`.

**Forge** — one per Tribe. The smithy where the Tribe forges new
tools for itself: Chief notices a missing capability, generates
Rust source via the LLM, the Forge builds it into a WebAssembly
component inside a sandbox, pins it by content hash, the host
installs it as a plugin. This is what makes a Tribe **generative**.

**App** — what a Tribe hands the people around it. You tell Elder
*"give my team an app for open items"*; Elder reads the ontology
and scaffolds a declarative spec — tables, detail cards, charts,
effect forms — rendered by one generic renderer and served over
the LAN by the `wild-appd` sidecar, governed per person. Nobody
writes code. Concept tour: [`docs/apps.md`](docs/apps.md) ·
engineering record:
ADR-0154.

**Plugin** — anything not hard-wired into the binary. Three
delivery tiers (native Rust / embedded Wasm / OCI Wasm) × five
roles (chief / worker / llm-adapter / storage-adapter /
tool-provider). All sandbox-gated, signed, trust-tiered. See
[`docs/plugin-concept.md`](docs/plugin-concept.md).

---


## Learn more

- `docs/README.md` — the documentation front
  door: every doc by tier (understand · start · reference).
- [`docs/idea.md`](docs/idea.md) — the deeper *why*.
- [`docs/how-tribes-live.md`](docs/how-tribes-live.md) — the
  life-story of a Tribe: working, remembering, learning, forging.
- [`docs/ontology.md`](docs/ontology.md) — the ontology in plain
  words: Ingest · Entities · Effects · Workers, and where you see
  them in the app.
- [`docs/apps.md`](docs/apps.md) — one sentence to a governed app
  for the people around you.
- `docs/atlas.md` — the data ground: ontology,
  provenance, search, time.
- The guided tours:
  [fitness (home)](docs/showcase-fitness.md) ·
  [liquidity (business)](docs/showcase-liquidity.md).

## For developers

The Wild is a Rust mono-repo: an embedded WebAssembly host on a
NATS spine, component plugins, a DDD compiler, and a Forge that
builds new components at runtime.

**Writing a plugin?** A plugin is a WebAssembly component — a
binary contract, not a language, so the host loads a Rust one and
a TypeScript one identically. Everything you need is published:
the interface contracts under `wit/`, working samples under
`examples/`, a template to copy, and the handbook at
[`docs/plugin-developer-guide.md`](docs/plugin-developer-guide.md).

The architecture trail, the CLI surface and the per-ADR status of
record live in the development repository.

## License

[Wildstuff Commercial License](LICENSE) (`LicenseRef-Wildstuff-Commercial`) —
the core is commercial and carries no third-party grant.

Three parts are deliberately permissive, because they exist to be read and
copied: the WIT interface contracts (`wit/`), the coding samples
(`examples/`), and the plugin template (`plugins/tool-provider-scaffold/`)
are **Apache-2.0**. Distributed binaries are covered by the end-user terms
accompanying that distribution.

