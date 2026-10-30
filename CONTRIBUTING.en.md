# Contributing to Tronix Launcher

[Français](CONTRIBUTING.md) · **English**

Thanks for your interest! The project is under the [MIT license](LICENSE): by submitting a change you agree that it is published under that license.

## Before you start

- **A bug?** Open a "Bug" issue (the form guides you). If it is a **security** problem (vulnerability, leaked key…), do not post details in a public issue: use the repository's *Security* tab → *Report a vulnerability* if it is enabled, otherwise contact the maintainer privately.
- **An idea?** Open an "Idea" issue before coding: the project keeps a simple line (fast, clean, useful) and a feature that does not fit will be declined. Features already scoped are in [docs/PRD.md](docs/PRD.md) (French).
- **An obvious fix** (typo, small bug): a direct pull request is fine.

## Set up

See [README.en.md](README.en.md#build) for the tools. Then:

```powershell
npm install
cargo test --manifest-path src-tauri/Cargo.toml   # core tests (Rust)
npm run test:ui                                    # interface tests (fake DOM)
cargo run --manifest-path src-tauri/Cargo.toml --profile fast
```

## Project rules

They protect speed and security. The most important:

1. **No network access at startup.** Every network call comes from a setting the user turned on, and never holds a `Mutex` while waiting.
2. **Framework-free interface**, DOM built with `el()` / `button()`. **Never `innerHTML`** (nor `eval`, nor inline scripts) with outside content: the Content Security Policy (`tauri.conf.json`) blocks them anyway.
3. **Every identifier coming from the interface is checked** before it is used in a file path (`store::is_safe_id`).
4. **API keys** are encrypted on disk (`secret.rs`) and **never** appear in an error message, a log or a displayed URL (`covers::net_error`). Calls that carry a key in the URL follow no redirects.
5. **Lock order: settings, then library** (`settings` before `store`), everywhere.
6. Launch targets that are not `.exe` go through `explorer.exe` (never `cmd /C start`), and only known link types are accepted.
7. **User files are never overwritten** without a copy: an unreadable file is set aside as `.corrupt`.
8. **State what is verified and what is not** in your pull request. This project distinguishes "tested on real games" from "tested on sample files".

## Proposing a change

1. *Fork*, then create a branch (`fix/...` or `feat/...`).
2. Keep the change **small and focused**; one pull request = one subject.
3. Add or update **tests** (`cargo test`, and `tests/ui/` if you change `src/main.js`). Tests must pass.
4. Code and comments are in **French**; you may write yours in **English**, which is welcome. **No visible text is hard-coded**: it goes through `t("key")` (interface) or `i18n::t("key")` (Rust), with the key added to **both** `fr.json` and `en.json` (see [docs/TRANSLATING.md](docs/TRANSLATING.md)).
5. Fill in the pull request template, with a screenshot if the interface changes.

## Style

- Rust: `cargo fmt` is not enforced today; follow the style of the file you edit. No `unwrap()` on user input; for a lock, use `lock_safe()`.
- JavaScript: ES modules, no new dependency without discussion (the goal is a fast startup and a small installer).
- A comment explains **why**, not what the line does.
