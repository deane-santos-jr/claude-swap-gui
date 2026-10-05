# Claude Swap for macOS

A native macOS window for [claude-swap](https://github.com/realiti4/claude-swap), written in Rust with [egui](https://github.com/emilk/egui). It shows every Claude Code account's 5-hour, 7-day, per-model, and extra-usage meters, switches accounts in one click, runs auto-switch as a login item, and rescues a Claude desktop app session that hit its limit.

The window is a thin shell over the `cswap` CLI. Every action runs a `cswap … --json` command and renders the result, so the CLI, its TUI, the menu bar, and this window always agree on state.

## Requirements

- macOS 12 or later
- Rust 1.80 or later
- claude-swap with `cswap` on `PATH`, or its path in `CSWAP_BIN`:

  ```bash
  uv tool install claude-swap   # or: pipx install claude-swap
  ```

## Build

```bash
./bundle.sh
cp -R "target/Claude Swap.app" ~/Applications/
```

`bundle.sh` builds a release binary, wraps it in `Claude Swap.app`, and signs it ad hoc. The bundle's `PATH` includes `~/.local/bin`, `/opt/homebrew/bin`, and `/usr/local/bin`, where `uv tool`, `pipx`, and venv symlinks usually put `cswap`.

## Controls

- **Switch** makes that account the login every Claude Code terminal and the VS Code extension use. Running sessions pick it up within about 30 seconds.
- **Terminal** opens a Terminal window running Claude Code as that account only (`cswap run N`), leaving the active login alone.
- **Hold out / Rejoin rotation** keeps an account out of auto-switch and rotation (`cswap disable` / `enable`).
- **Rotate / Most quota / Next not limited** are `cswap switch` with the matching strategy.
- **Auto-switch in the background** installs a login item (`~/Library/LaunchAgents/com.cswap.auto.plist`) that runs `cswap auto`, so it keeps working with the window closed and after a reboot. It polls every minute and moves the active login to the account with the most quota before the current one reaches the threshold. The slider sets `autoswitch.threshold`, **Check now** runs one decision immediately, and the log is `~/Library/Logs/com.cswap.auto.log`.
- **Add current login** snapshots whichever account Claude Code is logged in as (`cswap add`). Run `/login` in Claude Code first to add a different account.
- **Resume in Terminal** reopens a Claude desktop app session in Terminal under the chosen account (`cswap run N --share-history -- --resume <id>`). The desktop app keeps its own login and cannot be switched from outside.

Usage refreshes every minute and after every action, and each card shows when its numbers were measured.

## Keeping the numbers live

Anthropic's usage endpoint has a small request budget and answers HTTP 429 for up to an hour once it is spent, leaving only the last measurement. Claude Code, however, receives the account's live 5-hour and 7-day usage with every reply and passes it to the status line.

`statusline/install.sh` wraps your existing status-line command with a hook that pipes that payload into `cswap ingest-usage`, which writes it into the cache the window, the CLI, and the auto-switcher all read. The hook only calls `cswap` when the numbers change, in the background, so the status line stays instant.

```bash
statusline/install.sh
```

**Note:** `cswap ingest-usage` is not in a claude-swap release yet. It lives on the [`rust-gui` branch of my fork](https://github.com/deane-santos-jr/claude-swap/tree/rust-gui). Without it, the hook does nothing and the window falls back to the usage endpoint.

## Other hosts

Xirp, VS Code, and other hosts that spawn `claude` read the default login from the macOS Keychain, so a switch reaches them within about 30 seconds without a restart or `--resume`. The Claude desktop app signs in on its own; use **Resume in Terminal** for those sessions.

## Credits

Built on [claude-swap](https://github.com/realiti4/claude-swap) by Onur Cetinkol (MIT). This project started as the `gui/` folder of a fork and was split out with its history.

## License

MIT
