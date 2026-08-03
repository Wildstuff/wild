# Plugin trust — schema + CLI

D3 Stage-2 ships in three pieces under `wild_host`:

| Module | What it owns |
|---|---|
| `plugin_trust` | `~/.wild/plugin-trust.json` — operator-state (publishers + capability overrides). |
| `plugin_trust_writer` | The single write path for trust-tier mutations on a plugin's sidecar (+ SQLite cache mirror). |
| `plugin_trust_gate` | Load-time check of a plugin's `requires:` against its declared tier. |

`plugin-concept.md` carries the *why*. This doc is the operator
reference: file shape, command flow, what the loader does on
plugin instantiation.

## On-disk shape — `~/.wild/plugin-trust.json`

```json
{
  "publishers": [
    {
      "name": "wildstuff-org-local",
      "key_url": "https://keys.example.org/cosign.pub",
      "tier": "verified",
      "registered_at": "2026-04-27T12:00:00Z"
    }
  ],
  "capability_overrides": [
    {
      "plugin_slug": "math-tools",
      "added_caps": ["wasi:http/outgoing-handler"],
      "granted_at": "2026-04-27T12:00:00Z",
      "granted_by": "wild plugin grant math-tools wasi:http/outgoing-handler"
    }
  ]
}
```

- Both arrays default to empty. Missing file → empty store
  (a fresh `wild` install runs without a manual `touch`).
- Empty file → empty store too.
- Unknown top-level fields are ignored (forward-compat).
- `publishers[].tier` accepts `verified` / `community` / `unknown`
  today. `verified-local` is reserved for P8 cosign verification
  (lands W5) and rejected at parse + write time with a typed
  error pointing at the P8 lane — keeps a forward-written file
  from silently downgrading on read.
- File is written mode 0600 via `NamedTempFile` + `persist`
  (atomic same-filesystem rename).

## OQ#10 — sidecar is authoritative

Per-plugin trust tier lives in
`~/.wild/plugins/<slug>.json`. The `component_types.tier` SQLite column is a
*derived cache* the plan-renderer reads on the hot path.
`plugin-trust.json` does **not** mirror per-plugin tiers — it
holds operator-state on top of install metadata.

Every tier write goes through
`PluginTrustWriter::update_tier(slug, tier)`:

1. Atomically rewrite `<sidecar_dir>/<slug>.json` with the new
   tier (preserving every other field via `serde_json::Value`).
2. If a `component_types` row exists for the slug, mirror via
   `replace_component_type` so the plan-renderer's next call
   picks it up.
3. Return `UpdateOutcome::SidecarOnly` (provider-flavor: no
   SQLite row) or `UpdateOutcome::SidecarAndSqlite`
   (workload-flavor).

A SQLite mirror failure surfaces as
`TrustWriterError::SqliteMirrorFailed` — the sidecar (canonical)
is correct; the cache will catch up on the next `update_tier` or
on a manual `mirror_sidecar_to_sqlite` call.

`bootstrap_seed_component_types` and the existing
`register_component_type` / `replace_component_type` APIs stay
direct paths for now (built-ins have no sidecar). P7 absorbs
the migration when the plugin rename lands.

## Trust-tier allowance map

| Tier | Policy |
|---|---|
| `Verified` | Full caps the manifest declares — gate is a no-op. |
| `Community` | Blocklist: no `wild:secrets/store`/`admin`, no `wild:bundles/admin`, no `wild:plugin-storage/*`. Reads are fine. |
| `Unknown` | Allowlist: `wild:blobs/read`, `wild:messaging/consumer` only. Raw `wasi:http/outgoing-handler` is **not** blanket-granted here (it is an ungoverned outbound socket — no egress allowlist, no per-call audit); an Unknown-tier plugin that needs it gets a per-slug `capability_overrides` grant, minted automatically for an operator-confirmed connector install (see below) or by hand via `wild plugin grant`. |

**Override stacking** is additive across all tiers — every entry
in `capability_overrides` for the slug is treated as
pre-allowed regardless of tier. So an override can unblock a
Community-blocklisted cap (`wild:plugin-storage/store`) AND
extend an Unknown-tier allowlist with a single cap, with the
same shape.

**Block matching:**

- Exact qname match: `wild:secrets/store` blocks the exact qname.
- Package-prefix match: `wild:plugin-storage` blocks every
  `wild:plugin-storage/*` interface. The `/` separator is
  required — `wild:plugin-storageish/whatever` does **not**
  match.

## Operator CLI flow

```sh
# Grant one or more caps to a plugin.
wild plugin grant math-tools wasi:http/outgoing-handler
wild plugin grant math-tools wasi:http/outgoing-handler wild:secrets/get

# Render every override (or filter by slug).
wild plugin grants
wild plugin grants --slug math-tools

# Drop a single cap from every entry that lists it.
# Entries with multiple caps are rewritten without it; entries
# that held only the target cap are dropped.
wild plugin revoke math-tools wasi:http/outgoing-handler

# Drop every override for a slug.
wild plugin revoke math-tools
```

The gate reloads `~/.wild/plugin-trust.json` on every check, so
a `grant` issued during a long-running `wild up` lands for the
next plugin instantiation — no host restart needed.

### Connector-install auto-grant

An OCI-pulled connector lands at the `Unknown` tier, but a
marketplace install is an explicit, operator-confirmed act — the
operator saw the offer (secrets, sign-in, egress) and said yes.
So when the confirmed `install_offering` completes, the host
mints a per-slug `capability_overrides` grant for exactly the raw
egress cap the connector *declares* it needs
(`wasi:http/outgoing-handler`) and nothing else. The install
response echoes it under `granted_capabilities`. A connector that
declares no raw-http import gets no grant; a more powerful cap
(e.g. `wild:secrets/store`) is never auto-granted — it stays an
explicit `wild plugin grant`. This is what lets a deliberately
installed connector reach its API while the tier's *blanket* raw
egress stays closed to a plugin that merely appeared on disk.

`wild plugin add` / `list` / `remove` shipped in P7 (D4's
`components/` directory was renamed to `plugins/` then ADR-0026
§1 Amendment 2 dissolved the `added/` wrapper above it). The
legacy `wild component add` thin alias remains for one release
with a deprecation hint.

Publisher-management subcommands (`wild plugin trust-publisher
add/remove`) are deferred to P8 cosign verification — the
schema persists publishers but no operator workflow needs them
yet.

## Load-time check (W2 hook)

The provider-flavor loader hooks into:

```rust
trust_gate.check(plugin_slug, tier, requires)
    .map_err(|e| /* surface to wild_up output */)?;
```

before the host accepts the plugin registration. On reject the
error carries:

- plugin slug
- declared tier
- the FIRST denied cap (short-circuit so CLI output stays readable)
- the full allowed-set for the tier (with override caps appended)
- the exact `wild plugin grant <slug> <cap>` command the
  operator can run

Hook point lives in Agent A's `wire_p1_component_plugins` (P1.6
on `plugin/p1-shim`). The gate API is frozen — integration is
a one-liner once the W2 P1.x merge lands.

**P2 follow-up note:** P1.6's loader-walk processes
`add_to_linker` snapshots only. `init_store` snapshots need the
same treatment so real Tier-2 plugins get their WASI slots —
~+20 LOC capability-shim-specific. Without that the gate may
let a plugin past, but its WASI setup will fail downstream.
