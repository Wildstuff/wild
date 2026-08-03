# Operator guide — running `wild-hostd` as a system daemon

ADR-0035 §3.3 split `wild` into a frontend CLI + a long-running daemon
called `wild-hostd`. This guide covers the operator-facing side:
how to install the daemon as a system-managed service so it survives
reboots / logouts and gets restarted on crash.

> **Status:** ADR-0035 §3.3.4 shipped. Init-script artefacts ship in
> the repo under `dist/`; the dual-binary release pipeline ships both
> binaries in the GitHub Release tarball, and the brew-tap formula
> installs them (see `docs/homebrew-tap-setup.md`). For a from-source
> install, `cargo install --path crates/runtime/frontend` (the `wild`
> cli) and `cargo install --path crates/runtime/daemon` (`wild-hostd`),
> or copy the binaries built into your per-worktree `target/release/`.

## What `wild-hostd` is

The same runtime `wild up` boots, minus the TUI. Both binaries link
the `cli` library and share the bring-up sequence end-to-end
(`cli::bootstrap::run_up_with`). The split:

- **`wild`** — frontend. Talks to a running daemon over the IPC
  control socket (`<profile>/system/runtime.sock`). All of `wild
  chat`, `wild tribe apply`, `wild status`, `wild log-level`, `wild
  reconcile`, `wild metrics`, `wild debug` are thin clients.
- **`wild-hostd`** — daemon. Owns the `wild_host::Host`, the
  reconcile loop, the embedded-deploy NATS subscriber, and the
  control socket. Headless by construction; logs to
  `~/.wild/profiles/<active>/system/logs/wild-hostd.log`.

Today (post-3.3.3) you can run them in two shapes:

| Mode | Command | Notes |
|---|---|---|
| In-process | `wild up` | `wild` boots the host inside its own tokio runtime. The TUI exits with the daemon. Default for laptops. |
| Out-of-process | `wild-hostd &` then `wild chat`/`wild watch`/etc. | Daemon survives the TTY closing. The next tranche flips this to the default. |

The init-script artefacts in this PR are for the **out-of-process**
mode — wired into systemd / launchd so the daemon is managed like
any other service.

## macOS — launchd LaunchAgent

```sh
bash dist/install/install-launchd.sh
```

What it does:

1. Resolves your `wild-hostd` path (`command -v wild-hostd`) — works
   for brew (`/opt/homebrew/bin/...` on Apple Silicon, `/usr/local/bin/...`
   on Intel) or cargo (`~/.cargo/bin/...`).
2. Renders `dist/launchd/com.wildstuff.wild-hostd.plist` with that
   path and writes it to `~/Library/LaunchAgents/`.
3. `launchctl bootstrap`-loads + enables + kickstarts the agent.

After install:

```sh
# status
launchctl print gui/$(id -u)/com.wildstuff.wild-hostd

# stop / start manually
launchctl kickstart -k gui/$(id -u)/com.wildstuff.wild-hostd     # restart
launchctl kill TERM gui/$(id -u)/com.wildstuff.wild-hostd        # stop

# logs
tail -f ~/.wild/profiles/$(cat ~/.wild/active-profile)/system/logs/wild-hostd.log
```

Uninstall:

```sh
bash dist/install/install-launchd.sh --uninstall
```

The script `bootout`s the agent and removes the plist. Operator
state under `~/.wild/` is untouched.

### LaunchAgent vs LaunchDaemon

The shipped plist is a per-user **LaunchAgent**, not a system-wide
LaunchDaemon. Reasons:

- Wild's profile state lives at `~/.wild/profiles/<active>/` (per
  ADR-0026). A LaunchDaemon running as root would need a separate
  root-owned profile root + ownership rules.
- LaunchAgents start at user login; LaunchDaemons start at boot. The
  brew/cargo install model matches "start when I log in".
- An operator-side install doesn't need the privilege escalation a
  LaunchDaemon costs.

If you genuinely need a system-wide daemon (multi-user host, headless
CI runner, kiosk), copy the plist into `/Library/LaunchDaemons/`,
adjust `UserName` / `ProgramArguments` / log paths to match a
service account, and `sudo launchctl bootstrap system ...`. That
flavour is not covered by the install helper.

## Linux — systemd user service

```sh
bash dist/install/install-systemd.sh
```

What it does:

1. Resolves your `wild-hostd` path the same way as the macOS
   script.
2. Renders `dist/systemd/wild-hostd.service` with that path into
   `~/.config/systemd/user/`.
3. `daemon-reload`s + `enable --now`s the service.

After install:

```sh
# status
systemctl --user status wild-hostd

# stop / start
systemctl --user stop wild-hostd
systemctl --user start wild-hostd

# follow daemon log via journald
journalctl --user -u wild-hostd -f

# the rotated tracing log lives in the profile root regardless
tail -f ~/.wild/profiles/$(cat ~/.wild/active-profile)/system/logs/wild-hostd.log
```

Uninstall:

```sh
bash dist/install/install-systemd.sh --uninstall
```

### Headless servers

`systemctl --user` needs a logind session. On a headless box without
a persistent shell, the user manager dies on logout and the service
dies with it. Run `loginctl enable-linger <user>` once to keep the
user manager alive across logout — then the service runs uninterrupted
even when nobody's logged in.

```sh
loginctl enable-linger "$USER"
systemctl --user enable --now wild-hostd
```

### User vs system unit

Same logic as macOS: per-user matches the brew/cargo install model
and avoids the root/profile-root ownership question. For a system
unit (e.g. a CI runner where the daemon should run under a service
account), copy the file into `/etc/systemd/system/`, swap `[Install]
WantedBy=multi-user.target`, set `User=<service-account>` /
`Environment=HOME=...`, and `systemctl daemon-reload && systemctl
enable --now wild-hostd`. Out of scope for the helper.

## Verifying the daemon

Both install paths run `wild-hostd` headless. The cli verbs see it
the same way they'd see an in-process `wild up`:

```sh
$ wild status
host id     : <id>
tribe       : root
state       : Running
uptime      : 47s
…

$ wild metrics
host id        : …
primary tribe  : root
uptime         : 47s
components     : 4
…

$ wild reconcile
✓ kick enqueued for every registered tribe
  reconcile kick enqueued; the loop will tick on its next select cycle
```

If `wild status` reports "no runtime is up" while the OS service
manager says it IS running, the daemon's listener didn't bind — check
the log file in the profile root for the bootstrap error.

## Troubleshooting

### "another `wild` daemon is already running"

The IPC control socket is single-binder. If `wild up` and
`wild-hostd` (via launchd/systemd) try to coexist, the second one
bails. Pick one ownership model:

- **Service-managed daemon** (recommended for long-running setups):
  install via the helper, then drive everything through `wild`
  client commands. Don't run `wild up` separately.
- **Interactive `wild up`** (laptops, demos): uninstall the service
  first (`--uninstall` on the helper), then `wild up` as before.

The new "daemon already running" probe (ADR-0035 §3.3.2.B) makes
this collision visible in <1 s instead of letting the second boot
race for resources.

### Service won't start

```sh
# macOS — recent stderr
tail -n 50 ~/.wild/wild-hostd.launchd.err.log

# Linux — recent journal
journalctl --user -u wild-hostd -n 50 --no-pager
```

Common causes: stale `runtime.pid` lockfile (lockfile probe should
clean it; if not, delete it manually), pre-flight tool drift
(`wild doctor`), NATS port already bound by another process.

### Pinning a specific profile

The daemon resolves the active profile from `~/.wild/active-profile`
unless overridden. To pin a specific profile in the service manager:

- macOS: edit the LaunchAgent plist, add `WILD_PROFILE=<name>` to
  `EnvironmentVariables`, reload via the install helper.
- Linux: drop a `~/.config/systemd/user/wild-hostd.service.d/profile.conf`
  with `[Service]\nEnvironment=WILD_PROFILE=<name>\n`, then
  `systemctl --user daemon-reload && systemctl --user restart
  wild-hostd`.

## What shipped

- **§3.3.4.B** — the release pipeline ships both binaries side-by-side
  in the GitHub Release tarball + the OCI workflow.
- **§3.3.4.C** — the Brew tap formula installs both binaries and (when
  `--with-launchd` is passed) the LaunchAgent automatically.
- **§3.3.2.C default flip** — `wild up`'s default is now
  spawn-and-attach against the daemon; `--in-process` keeps the legacy
  single-binary mode (mainly for tests).

The brew tap (`docs/homebrew-tap-setup.md`) is the operator-facing
install path; the from-source `cargo install` helpers above stay the
developer path.
