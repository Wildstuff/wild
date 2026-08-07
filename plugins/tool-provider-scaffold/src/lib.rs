//! Declarative scaffold for Tier-2 `wild:tool-provider@0.4.0` plugins.
//!
//! Every connector (`folder-connector`, `web-connector`) and format/util
//! adapter (`csv-parser`, `json-parser`, `pdf-parser`, `brave-search`,
//! `math-tools`, `http-fetcher`) re-implemented the SAME boilerplate: a
//! `Component` struct, the `ToolsGuest` (`list_tools` / `list_skill_mds` /
//! `invoke`-by-name) and `MetaGuest` (`manifest` + stateless `init`/`shutdown`)
//! impls, the `export!`, and an identical `to_manifest_sig` projection. The
//! plugins are uniform — all `kind: Provider`, stateless, no config keys or
//! secret aliases — so only the tool table, the WIT `requires`, and the typed
//! `signatures` actually vary.
//!
//! These `macro_rules!` generate that boilerplate. They are deliberately
//! `macro_rules!` (not a proc-macro): the bindgen-emitted types
//! (`exports::wild::…`) live in the CALLING crate, and `macro_rules!` path
//! resolution is call-site, so the expansion binds to each plugin's own
//! `wit_bindgen::generate!` output. Invoke AFTER `wit_bindgen::generate!`, at
//! the plugin crate root.
//!
//! A second macro family, [`egress_fault_mapping!`], generates the
//! `wild:http` egress-error → fault-class mapping (ADR-0116 D1) that every
//! egressing plugin re-implemented by hand — usable by ANY plugin with a
//! `wild:http/outbound` import (channels, embed/rerank adapters included),
//! not just tool-providers.
//!
//! ## Forge twin — keep in sync
//!
//! A FORGED tool-provider plugin does NOT use this crate: the forge
//! sandbox is a hermetic, offline build from a self-contained snapshot and
//! cannot path-dep a monorepo crate. The Forge wright instead emits the
//! same frame deterministically as a string template —
//! `build_seed_tool_provider_lib_rs` in
//! `plugins/chiefs/default/src/wright.rs`. These two are INTENTIONALLY
//! independent representations of the one WIT surface (the house rule's
//! "documented INDEPENDENT statement" carve-out, CLAUDE.md "No new
//! hand-mirrored surface"). If the `wild:tool-provider` / `wild:plugin-meta`
//! `Guest` surface changes (a new required method, a manifest field),
//! update BOTH this macro AND that seed template — the in-tree side breaks
//! loudly (8 plugin builds), the forge side is caught by the wright unit
//! tests + forge e2e.
//!
//! [`enumerate_result_schema!`] is the same twin one level up: it is the
//! source-connector `enumerate` CONTRACT, and a forged connector gets it from
//! its spec's `#### Signature:` block instead — taught by
//! `docs/intake-pipeline-conventions.md`. Both sides are held by the
//! enumerate-shape gate (`scripts/ci/check-enumerate-shape.py`), so a key
//! added here without the doc, or dropped from the doc, fails pre-push.

/// Generate the byte-identical `to_manifest_sig` helper — a `sig::ToolSig`
/// (the plugin's static typed-signature, ADR-0056) → bindgen `Sig` projection.
/// Invoke in plugins that declare typed `signatures`; the plugin must have a
/// `sig` module exposing `ToolSig { name, description, input_schema,
/// output_schema, examples: &[(in, out)] }`.
#[macro_export]
macro_rules! impl_to_manifest_sig {
    () => {
        fn to_manifest_sig(s: &sig::ToolSig) -> exports::wild::plugin_meta::meta::Sig {
            exports::wild::plugin_meta::meta::Sig {
                name: s.name.into(),
                description: s.description.into(),
                input_schema: s.input_schema.into(),
                output_schema: s.output_schema.into(),
                examples: s
                    .examples
                    .iter()
                    .map(
                        |(input, output)| exports::wild::plugin_meta::meta::SigExample {
                            input: (*input).into(),
                            output: (*output).into(),
                        },
                    )
                    .collect(),
            }
        }
    };
}

/// Generate the `wild:http` egress-error → fault mapping at the CALL SITE.
/// Every plugin that imports `wild:http/outbound` re-implemented the same
/// mapping: render the typed `ob::Error` as a stable `<kind>: <detail>`
/// string, and read the error variant onto the WIT `fault-class` (ADR-0116
/// D1 — the class routes host-side, the message stays the human detail).
/// Like the other macros here this is `macro_rules!` because each plugin's
/// own bindgen emits its OWN `ob::Error` / `FaultClass` — a shared fn
/// cannot name the type; the expansion binds to the caller's copy.
///
/// Four forms, by how much the plugin shares:
///
/// - `describe_error` — just the stable string renderer (channels,
///   connectors that flatten to prose).
/// - `fault_class` — `describe_error` + `egress_fault_class` (the typed
///   variant→class read onto `tools::FaultClass`).
/// - `classified_error` — the full Graph-style surface: a named
///   `{ class, message }` struct with `unknown` / `context` /
///   `into_tool_error`, plus `classify_ob_error` and a status classifier
///   (401/403 → Auth · 429 → RateLimited · 408/5xx → Transient · else
///   Unknown) whose message the caller shapes via `status_message`.
/// - `custom_map` — the plugin keeps its own error enum and per-variant
///   messages; the macro supplies only the exhaustive five-variant match
///   (so a new `wild:http` variant breaks every consumer loudly).
///
/// ```ignore
/// tool_provider_scaffold::egress_fault_mapping! {
///     outbound: crate::wild::http::outbound,
///     describe_error,
/// }
/// ```
#[macro_export]
macro_rules! egress_fault_mapping {
    // Just the stable string renderer.
    (
        outbound: $($ob:ident)::+,
        describe_error $(,)?
    ) => {
        /// Render a `wild:http` error as a stable, human-readable string.
        pub fn describe_error(e: &$($ob)::+::Error) -> ::std::string::String {
            match e {
                $($ob)::+::Error::DeniedHost(m) => ::std::format!("denied-host: {m}"),
                $($ob)::+::Error::DeniedAuth(m) => ::std::format!("denied-auth: {m}"),
                $($ob)::+::Error::RateLimited(m) => ::std::format!("rate-limited: {m}"),
                $($ob)::+::Error::Timeout(m) => ::std::format!("timeout: {m}"),
                $($ob)::+::Error::Transport(m) => ::std::format!("transport: {m}"),
            }
        }
    };

    // The string renderer + the typed variant→`fault-class` read.
    (
        outbound: $($ob:ident)::+,
        tools: $($t:ident)::+,
        fault_class $(,)?
    ) => {
        $crate::egress_fault_mapping! {
            outbound: $($ob)::+,
            describe_error,
        }

        /// Classify a `wild:http` outbound error onto the WIT `fault-class`
        /// (ADR-0116 D1): the transport/governance layer already knows
        /// exactly what went wrong, so the class is a 1:1 read.
        /// `denied-auth` is `Auth` (the operator fixes the key/binding —
        /// the keyless-source case), `denied-host` is `Config` (the
        /// tribe's own egress allowlist, not the remote, needs fixing).
        pub fn egress_fault_class(e: &$($ob)::+::Error) -> $($t)::+::FaultClass {
            match e {
                $($ob)::+::Error::DeniedAuth(_) => $($t)::+::FaultClass::Auth,
                $($ob)::+::Error::DeniedHost(_) => $($t)::+::FaultClass::Config,
                $($ob)::+::Error::RateLimited(_) => $($t)::+::FaultClass::RateLimited,
                $($ob)::+::Error::Timeout(_) => $($t)::+::FaultClass::Transient,
                $($ob)::+::Error::Transport(_) => $($t)::+::FaultClass::Unreachable,
            }
        }
    };

    // The full classified-error surface: a named `{ class, message }`
    // struct, the ob-error classifier, and a non-2xx status classifier
    // (the caller shapes the status message via `status_message`).
    (
        outbound: $($ob:ident)::+,
        tools: $($t:ident)::+,
        classified_error: $err:ident,
        status_message: $status_msg:expr $(,)?
    ) => {
        $crate::egress_fault_mapping! {
            outbound: $($ob)::+,
            tools: $($t)::+,
            fault_class,
        }

        /// A classified egress-call failure (ADR-0116 D1): the CLASS is
        /// what routes host-side — the fault net, the preflight report,
        /// the operator inbox — while `message` stays the human detail.
        pub struct $err {
            pub class: $($t)::+::FaultClass,
            pub message: ::std::string::String,
        }

        impl $err {
            /// An unclassified failure (pure parse errors and the like) —
            /// never a guessed class from prose (ADR-0116 D1).
            pub fn unknown(message: ::std::string::String) -> Self {
                Self {
                    class: $($t)::+::FaultClass::Unknown,
                    message,
                }
            }

            /// Prefix the human detail with call context, keeping the class.
            pub fn context(self, ctx: &str) -> Self {
                Self {
                    class: self.class,
                    message: ::std::format!("{ctx}: {}", self.message),
                }
            }

            /// The typed WIT tool error (`fault(fault-detail)`).
            pub fn into_tool_error(self) -> $($t)::+::ToolError {
                $($t)::+::ToolError::Fault($($t)::+::FaultDetail {
                    class: self.class,
                    message: self.message,
                })
            }
        }

        /// Classify a `wild:http` outbound error (the transport/governance
        /// layer already knows exactly what went wrong — carry that
        /// knowledge instead of flattening it to prose).
        pub fn classify_ob_error(e: &$($ob)::+::Error) -> $err {
            $err {
                class: egress_fault_class(e),
                message: describe_error(e),
            }
        }

        /// Classify a non-2xx status: 401/403 = the sign-in lacks
        /// permission (consent revoked, missing role); 429 = throttled;
        /// 5xx/408 = a remote-side hiccup worth a retry; everything else
        /// stays unknown.
        fn classify_status(status: u16, snippet: ::std::string::String) -> $err {
            let class = match status {
                401 | 403 => $($t)::+::FaultClass::Auth,
                429 => $($t)::+::FaultClass::RateLimited,
                408 | 500..=599 => $($t)::+::FaultClass::Transient,
                _ => $($t)::+::FaultClass::Unknown,
            };
            $err {
                class,
                message: ($status_msg)(status, snippet),
            }
        }
    };

    // A custom per-variant map onto the plugin's OWN error enum: the
    // plugin keeps its messages, the macro supplies the exhaustive
    // five-variant `wild:http` match.
    (
        outbound: $($ob:ident)::+,
        custom_map: fn $name:ident($e:ident) -> $out:ty {
            denied_host($dh:pat) => $dhv:expr,
            denied_auth($da:pat) => $dav:expr,
            rate_limited($rl:pat) => $rlv:expr,
            timeout($to:pat) => $tov:expr,
            transport($tr:pat) => $trv:expr $(,)?
        } $(,)?
    ) => {
        fn $name($e: $($ob)::+::Error) -> $out {
            match $e {
                $($ob)::+::Error::DeniedHost($dh) => $dhv,
                $($ob)::+::Error::DeniedAuth($da) => $dav,
                $($ob)::+::Error::RateLimited($rl) => $rlv,
                $($ob)::+::Error::Timeout($to) => $tov,
                $($ob)::+::Error::Transport($tr) => $trv,
            }
        }
    };
}

/// Generate the full Tier-2 tool-provider plugin surface: the `Component`
/// struct, the `ToolsGuest` + `MetaGuest` impls, and the `export!`. Every
/// tool-provider plugin is `kind: Provider`, stateless (empty `init`/
/// `shutdown`), with no config keys or secret aliases — the macro FIXES those;
/// the plugin supplies only what varies. Each tool entry names its registered
/// tool name (the `invoke` dispatch key), the `ToolSpec` expression, the
/// invoke fn (called as `f(&args_json)`), and an OPTIONAL bundled skill
/// `(slug => body)`.
///
/// ```ignore
/// wit_bindgen::generate!({ path: "wit", world: "folder-connector", generate_all });
/// tool_provider_scaffold::impl_to_manifest_sig!();
/// tool_provider_scaffold::tool_provider_plugin! {
///     slug: "folder-connector",
///     provides: ["wild:tool-provider@0.4.0"],
///     requires: ["wild:host-source/read@0.1.0"],
///     signatures: [to_manifest_sig(&sig::ENUMERATE)],
///     tools: [
///         { name: "enumerate", spec: tools::enumerate::spec(),
///           invoke: tools::enumerate::invoke, skill: ("enumerate" => ENUMERATE_SKILL) },
///     ],
/// }
/// ```
#[macro_export]
macro_rules! tool_provider_plugin {
    (
        slug: $slug:expr,
        provides: [$($prov:expr),* $(,)?],
        requires: [$($req:expr),* $(,)?],
        signatures: [$($sig:expr),* $(,)?],
        tools: [
            $(
                { name: $tname:expr, spec: $spec:expr, invoke: $inv:path
                  $(, skill: ($sslug:expr => $sbody:expr) )? }
            ),* $(,)?
        ]
        // ADR-0100 — OPTIONAL streaming tools: their per-tool `invoke` fn is
        // `async fn` and is `.await`ed, so it can drain a component-model-async
        // `stream<u8>` import (`wild:host-source/reader`) while decoding a
        // large source in place. A tool that never awaits stays in the plain
        // `tools:` list above (sync fn) — no reason to move it here.
        $(, streaming_tools: [
            $(
                { name: $stname:expr, spec: $stspec:expr, invoke: $stinv:path
                  $(, skill: ($stsslug:expr => $stsbody:expr) )? }
            ),* $(,)?
        ] )?
        // OPTIONAL tool-less prose skills (`source: prose` front-matter):
        // plugin-shipped JUDGEMENT guidance — e.g. a connector's operator
        // setup guide — served via `list-skill-mds()` like every other MD.
        // No invoke arm: dispatch on a Prose skill is a defined no-op
        // (ADR-0074 D5), the body is read-only knowledge.
        $(, prose_skills: [
            $( ($pslug:expr => $pbody:expr) ),* $(,)?
        ] )?
        $(,)?
    ) => {
        struct Component;

        impl exports::wild::tool_provider::tools::Guest for Component {
            fn list_tools() -> ::std::vec::Vec<exports::wild::tool_provider::tools::ToolSpec> {
                let mut __tools: ::std::vec::Vec<exports::wild::tool_provider::tools::ToolSpec> =
                    ::std::vec![ $( $spec ),* ];
                $($( __tools.push($stspec); )*)?
                __tools
            }

            fn list_skill_mds() -> ::std::vec::Vec<exports::wild::tool_provider::tools::SkillMd> {
                let mut __mds: ::std::vec::Vec<exports::wild::tool_provider::tools::SkillMd> =
                    ::std::vec::Vec::new();
                $($(
                    __mds.push(exports::wild::tool_provider::tools::SkillMd {
                        slug: $sslug.into(),
                        body: $sbody.into(),
                    });
                )?)*
                $($($(
                    __mds.push(exports::wild::tool_provider::tools::SkillMd {
                        slug: $stsslug.into(),
                        body: $stsbody.into(),
                    });
                )?)*)?
                $($(
                    __mds.push(exports::wild::tool_provider::tools::SkillMd {
                        slug: $pslug.into(),
                        body: $pbody.into(),
                    });
                )*)?
                __mds
            }

            // ADR-0100 — `invoke` is now an `async func` on the WIT (the host
            // drives it under `run_concurrent`). Sync tools return directly;
            // streaming tools are `.await`ed. `unused_async` is expected for a
            // provider that ships only sync tools — the async binding still
            // resolves in one poll.
            #[allow(clippy::unused_async)]
            async fn invoke(
                name: ::std::string::String,
                args_json: ::std::string::String,
            ) -> ::std::result::Result<
                exports::wild::tool_provider::tools::ToolResult,
                exports::wild::tool_provider::tools::ToolError,
            > {
                match name.as_str() {
                    $( $tname => $inv(&args_json), )*
                    $($( $stname => $stinv(&args_json).await, )*)?
                    other => ::std::result::Result::Err(
                        exports::wild::tool_provider::tools::ToolError::UnknownTool(other.into()),
                    ),
                }
            }
        }

        impl exports::wild::plugin_meta::meta::Guest for Component {
            fn manifest() -> exports::wild::plugin_meta::meta::PluginManifest {
                exports::wild::plugin_meta::meta::PluginManifest {
                    slug: $slug.into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    kind: Some(exports::wild::plugin_meta::meta::PluginKind::Provider),
                    provides: ::std::vec![ $( $prov.into() ),* ],
                    requires: ::std::vec![ $( $req.into() ),* ],
                    config_keys: ::std::vec::Vec::new(),
                    secret_aliases: ::std::vec::Vec::new(),
                    signatures: ::std::vec![ $( $sig ),* ],
                }
            }

            fn init(
                _config: ::std::vec::Vec<u8>,
            ) -> ::std::result::Result<(), exports::wild::plugin_meta::meta::InitError> {
                ::std::result::Result::Ok(())
            }

            fn shutdown() {}
        }

        export!(Component);
    };
}

/// The canonical `enumerate` RESULT schema every source connector declares —
/// ONE declaration, not five hand-written copies.
///
/// The intake-runner reads a fixed set of keys off each enumerated item, but
/// until this macro the shape lived only as prose in
/// `docs/intake-pipeline-conventions.md` plus a hand-written JSON schema in
/// every connector's `sig.rs`. Five connectors had five different item shapes,
/// and the two that dropped `size_bytes`/`modified_at` did so while already
/// holding the values — the folder connector in its `stat`, SharePoint in the
/// `driveItem` expand it was already fetching. Nothing failed; the operator
/// just got a file listing with no sizes, months later.
///
/// The four canonical keys:
///
/// - `item_ref` — the item's durable locator (a path, a URL, a Graph item id).
///   REQUIRED: the runner addresses everything by it.
/// - `name` — the human-facing filename. REQUIRED.
/// - `size_bytes` / `modified_at` — the INVENTORY pair. Optional, because a
///   queue row genuinely has neither; but emit BOTH whenever the origin knows
///   them. For a file with no text rail (an archive, an image, an installer)
///   they plus the name are all the operator has to recognise it by, and the
///   document door records them on every item.
///
/// Per-connector extras (`mime`, `fields`, `params`) are passed as `extra:`
/// and merged into the item's properties.
///
/// ```ignore
/// output_schema: tool_provider_scaffold::enumerate_result_schema!(),
/// output_schema: tool_provider_scaffold::enumerate_result_schema!(
///     extra: r#""fields": { "type": "object" }"#
/// ),
/// ```
///
/// **Forge twin — keep in sync.** A forged connector cannot path-dep this
/// crate (see the crate header), so its schema comes from the spec's
/// `#### Signature:` block. The same four keys are taught for that path in
/// `docs/intake-pipeline-conventions.md` § enumerate; `enumerate_shape_gate`
/// (`scripts/ci/check-enumerate-shape.py`) checks BOTH sides, so a connector
/// that hand-rolls the schema and omits a canonical key fails there.
#[macro_export]
macro_rules! enumerate_result_schema {
    () => {
        $crate::enumerate_result_schema!(extra: "")
    };
    (extra: $extra:literal) => {
        $crate::enumerate_result_schema!(extra: $extra, top_extra: "", top_required: "")
    };
    // An origin with no per-item file metadata at all (a queue row is a data
    // row, not a file) still shares `item_ref`/`name` and the envelope — it
    // just must not ADVERTISE an inventory it can never fill. A schema that
    // promises `size_bytes` and always omits it teaches the reader wrong.
    (no_inventory, extra: $extra:literal) => {
        $crate::enumerate_result_schema!(
            no_inventory, extra: $extra, top_extra: "", top_required: ""
        )
    };
    // No per-item inventory AND envelope extras — the web connector, whose
    // `enumerate` lists URLs the operator named BEFORE any HTTP call, so there
    // is no size and no last-change to report yet, and whose result carries
    // pagination on the envelope.
    (no_inventory, extra: $extra:literal, top_extra: $top:literal, top_required: $top_req:literal) => {
        concat!(
            r#"{
  "type": "object",
  "properties": {
    "source_kind": { "type": "string", "description": "REQUIRED (ADR-0065 D2) — the origin kind this connector docks." },
    "items": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "item_ref": { "type": "string", "description": "The item's durable locator — path, URL or item id." },
          "name": { "type": "string", "description": "The human-facing filename." }"#,
            $extra,
            r#"
        },
        "required": ["item_ref", "name"]
      }
    }"#,
            $top,
            r#"
  },
  "required": ["source_kind", "items""#,
            $top_req,
            r#"]
}"#
        )
    };
    // A connector with extra ENVELOPE keys (web-connector's pagination) still
    // shares the item shape — which is the part the runner reads and the part
    // that drifted. `top_required` appends to the envelope's required list.
    (extra: $extra:literal, top_extra: $top:literal, top_required: $top_req:literal) => {
        concat!(
            r#"{
  "type": "object",
  "properties": {
    "source_kind": { "type": "string", "description": "REQUIRED (ADR-0065 D2) — the origin kind this connector docks." },
    "items": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "item_ref": { "type": "string", "description": "The item's durable locator — path, URL or item id." },
          "name": { "type": "string", "description": "The human-facing filename." },
          "size_bytes": { "type": "integer", "description": "Size in bytes; omit only when the origin has none." },
          "modified_at": { "type": "string", "description": "Last change, ISO-8601; omit only when the origin has none." }"#,
            $extra,
            r#"
        },
        "required": ["item_ref", "name"]
      }
    }"#,
            $top,
            r#"
  },
  "required": ["source_kind", "items""#,
            $top_req,
            r#"]
}"#
        )
    };
}

/// The canonical enumerate-item keys, for the gate and for tests.
///
/// Deliberately a separate list from the macro body: the macro is the schema
/// a connector SHIPS, this is what a checker asserts about it. A key added to
/// one and not the other fails `the_macro_declares_every_canonical_key`.
pub const ENUMERATE_ITEM_KEYS: &[&str] = &["item_ref", "name", "size_bytes", "modified_at"];

// ── macro-expansion smoke tests ─────────────────────────────────────────
//
// The macros expand against CALL-SITE types, so the test defines minimal
// stand-ins for the bindgen-emitted shapes each plugin's own
// `wit_bindgen::generate!` produces (`wild:http/outbound::Error` and the
// `wild:tool-provider` fault types) and exercises every form.
#[cfg(test)]
mod tests {
    pub mod mock_ob {
        #[derive(Debug)]
        pub enum Error {
            DeniedHost(String),
            DeniedAuth(String),
            RateLimited(String),
            Timeout(String),
            Transport(String),
        }
    }

    pub mod mock_tools {
        #[derive(Debug, PartialEq, Eq)]
        pub enum FaultClass {
            Auth,
            Config,
            RateLimited,
            Transient,
            Unreachable,
            Unknown,
        }

        #[derive(Debug, PartialEq)]
        pub struct FaultDetail {
            pub class: FaultClass,
            pub message: String,
        }

        #[derive(Debug, PartialEq)]
        pub enum ToolError {
            Fault(FaultDetail),
        }
    }

    #[derive(Debug, PartialEq)]
    pub enum MockFail {
        Permanent(String),
        Retryable(String),
    }

    crate::egress_fault_mapping! {
        outbound: crate::tests::mock_ob,
        tools: crate::tests::mock_tools,
        classified_error: MockError,
        status_message: |status, snippet| format!("GET returned status {status} (expected 2xx): {snippet}"),
    }

    crate::egress_fault_mapping! {
        outbound: crate::tests::mock_ob,
        custom_map: fn map_fail(e) -> MockFail {
            denied_host(m) => MockFail::Permanent(format!("denied-host: {m}")),
            denied_auth(m) => MockFail::Permanent(format!("denied-auth: {m}")),
            rate_limited(m) => MockFail::Retryable(format!("rate-limited: {m}")),
            timeout(m) => MockFail::Retryable(format!("timeout: {m}")),
            transport(m) => MockFail::Retryable(format!("transport: {m}")),
        }
    }

    use mock_ob::Error as ObError;
    use mock_tools::{FaultClass, ToolError};

    #[test]
    fn describe_error_renders_stable_strings() {
        assert_eq!(
            describe_error(&ObError::DeniedHost("h".into())),
            "denied-host: h"
        );
        assert_eq!(
            describe_error(&ObError::DeniedAuth("a".into())),
            "denied-auth: a"
        );
        assert_eq!(
            describe_error(&ObError::RateLimited("r".into())),
            "rate-limited: r"
        );
        assert_eq!(describe_error(&ObError::Timeout("t".into())), "timeout: t");
        assert_eq!(
            describe_error(&ObError::Transport("x".into())),
            "transport: x"
        );
    }

    #[test]
    fn egress_fault_class_maps_every_variant() {
        assert_eq!(
            egress_fault_class(&ObError::DeniedAuth("a".into())),
            FaultClass::Auth
        );
        assert_eq!(
            egress_fault_class(&ObError::DeniedHost("h".into())),
            FaultClass::Config
        );
        assert_eq!(
            egress_fault_class(&ObError::RateLimited("r".into())),
            FaultClass::RateLimited
        );
        assert_eq!(
            egress_fault_class(&ObError::Timeout("t".into())),
            FaultClass::Transient
        );
        assert_eq!(
            egress_fault_class(&ObError::Transport("x".into())),
            FaultClass::Unreachable
        );
    }

    #[test]
    fn classify_ob_error_pairs_class_and_message() {
        let e = classify_ob_error(&ObError::DeniedAuth("no binding".into()));
        assert_eq!(e.class, FaultClass::Auth);
        assert_eq!(e.message, "denied-auth: no binding");
    }

    #[test]
    fn classify_status_maps_the_retry_boundaries() {
        assert_eq!(classify_status(401, "s".into()).class, FaultClass::Auth);
        assert_eq!(classify_status(403, "s".into()).class, FaultClass::Auth);
        assert_eq!(
            classify_status(429, "s".into()).class,
            FaultClass::RateLimited
        );
        assert_eq!(
            classify_status(408, "s".into()).class,
            FaultClass::Transient
        );
        assert_eq!(
            classify_status(503, "s".into()).class,
            FaultClass::Transient
        );
        assert_eq!(classify_status(404, "s".into()).class, FaultClass::Unknown);
        assert_eq!(
            classify_status(500, "boom".into()).message,
            "GET returned status 500 (expected 2xx): boom"
        );
    }

    #[test]
    fn classified_error_helpers_keep_the_class() {
        let unknown = MockError::unknown("parse failed".to_string());
        assert_eq!(unknown.class, FaultClass::Unknown);

        let ctx = MockError {
            class: FaultClass::Config,
            message: "denied-host: h".to_string(),
        }
        .context("download `x`");
        assert_eq!(ctx.class, FaultClass::Config);
        assert_eq!(ctx.message, "download `x`: denied-host: h");

        let ToolError::Fault(detail) = ctx.into_tool_error();
        assert_eq!(detail.class, FaultClass::Config);
        assert_eq!(detail.message, "download `x`: denied-host: h");
    }

    #[test]
    fn custom_map_applies_the_plugin_table() {
        assert_eq!(
            map_fail(ObError::DeniedHost("h".into())),
            MockFail::Permanent("denied-host: h".to_string())
        );
        assert_eq!(
            map_fail(ObError::Timeout("t".into())),
            MockFail::Retryable("timeout: t".to_string())
        );
        assert_eq!(
            map_fail(ObError::RateLimited("r".into())),
            MockFail::Retryable("rate-limited: r".to_string())
        );
    }
}

// ── enumerate-shape tests ───────────────────────────────────────────────
//
// The macro is the ONE declaration of the source-connector `enumerate` item
// shape; these hold it honest. They parse the expansion as JSON because a
// missing comma between the canonical keys and a connector's `extra:` breaks
// every consuming plugin at once — which is exactly how it broke once.
#[cfg(test)]
mod enumerate_schema_tests {
    #[test]
    fn the_macro_declares_every_canonical_key() {
        let schema = crate::enumerate_result_schema!();
        for key in crate::ENUMERATE_ITEM_KEYS {
            assert!(
                schema.contains(&format!("\"{key}\"")),
                "`{key}` is a canonical enumerate-item key but the macro does not declare it"
            );
        }
    }

    #[test]
    fn the_schema_is_valid_json_with_and_without_extras() {
        let plain: serde_json::Value =
            serde_json::from_str(crate::enumerate_result_schema!()).expect("plain schema is JSON");
        let with_extra: serde_json::Value = serde_json::from_str(
            crate::enumerate_result_schema!(extra: r#","fields": { "type": "object" }"#),
        )
        .expect("schema with extras is JSON — a missing comma would break every caller");
        let item = |v: &serde_json::Value| v["properties"]["items"]["items"]["properties"].clone();
        assert!(item(&plain).get("fields").is_none());
        assert!(item(&with_extra).get("fields").is_some());
        for key in crate::ENUMERATE_ITEM_KEYS {
            assert!(
                item(&with_extra).get(*key).is_some(),
                "`{key}` lost to extras"
            );
        }
    }

    #[test]
    fn only_the_locator_and_name_are_required() {
        let v: serde_json::Value = serde_json::from_str(crate::enumerate_result_schema!()).unwrap();
        assert_eq!(
            v["properties"]["items"]["items"]["required"],
            serde_json::json!(["item_ref", "name"])
        );
        assert_eq!(v["required"], serde_json::json!(["source_kind", "items"]));
    }

    #[test]
    fn the_envelope_arm_keeps_the_item_shape_and_extends_required() {
        let v: serde_json::Value = serde_json::from_str(crate::enumerate_result_schema!(
            extra: "",
            top_extra: r#","has_more": { "type": "boolean" }"#,
            top_required: r#", "has_more""#
        ))
        .expect("envelope arm is valid JSON");
        assert!(v["properties"]["has_more"].is_object());
        assert_eq!(
            v["required"],
            serde_json::json!(["source_kind", "items", "has_more"])
        );
        for key in crate::ENUMERATE_ITEM_KEYS {
            assert!(v["properties"]["items"]["items"]["properties"][*key].is_object());
        }
    }

    #[test]
    fn the_no_inventory_envelope_arm_drops_inventory_and_keeps_pagination() {
        // web-connector: URLs the operator named, listed BEFORE any HTTP call,
        // so there is no size or last-change to report yet — but the envelope
        // still paginates. Declaring an inventory it cannot know would teach
        // the reader wrong, which is the same fault the queue arm exists for.
        let v: serde_json::Value = serde_json::from_str(crate::enumerate_result_schema!(
            no_inventory,
            extra: "",
            top_extra: r#","has_more": { "type": "boolean" }"#,
            top_required: r#", "has_more""#
        ))
        .expect("no_inventory envelope arm is valid JSON");
        let item = &v["properties"]["items"]["items"]["properties"];
        assert!(item.get("item_ref").is_some());
        assert!(item.get("size_bytes").is_none());
        assert!(item.get("modified_at").is_none());
        assert!(v["properties"]["has_more"].is_object());
        assert_eq!(
            v["required"],
            serde_json::json!(["source_kind", "items", "has_more"])
        );
    }

    #[test]
    fn the_no_inventory_arm_shares_the_locator_but_promises_no_size() {
        let v: serde_json::Value =
            serde_json::from_str(crate::enumerate_result_schema!(no_inventory, extra: ""))
                .expect("no_inventory arm is valid JSON");
        let item = &v["properties"]["items"]["items"]["properties"];
        assert!(item.get("item_ref").is_some());
        assert!(item.get("name").is_some());
        assert!(item.get("size_bytes").is_none(), "must not promise a size");
        assert!(item.get("modified_at").is_none(), "must not promise a date");
    }
}
