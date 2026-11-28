# Contributing to ServerBox

Contributions to code, translations, bug reports, and documentation are
welcome.

## Before opening a pull request

- **Small fixes:** just open the pull request.
- **New features or anything that changes how the app behaves:** open an issue
  first and describe the idea. It avoids the case where the work is finished and
  the direction turns out to be wrong.
- **Subjective changes** (for example "this other UI looks better") may not be
  accepted. Anything with a concrete argument behind it is welcome to be
  discussed.

## Contributor License Agreement

Every contributor signs the [CLA](CLA.md) once
([中文译本](CLA_zh.md)). An automated check posts the instructions on your first
pull request, and you sign by leaving one comment:

```
I have read the CLA Document and I hereby sign the CLA
```

ServerBox is AGPLv3 and is also distributed through the
App Store, whose terms cannot all be satisfied alongside every AGPLv3
condition. The agreement grants the maintainer the right to ship your work in
those builds. You keep the copyright to what you wrote, and you can reuse it
anywhere else however you like. Read [CLA.md](CLA.md) for the terms that
actually bind — the paragraph above is a summary and nothing more.

Two things that trip the check up:

- **Commits authored by an email address not attached to your GitHub account.**
  The check then cannot tell who wrote them. Add the address at
  <https://github.com/settings/emails> (it can stay private) and comment
  `recheck`.
- **Co-authored commits.** Everyone who authored a commit in the pull request
  signs, not only whoever opened it.

## Development

### Setup

1. Install [Rust](https://rustup.rs); `rust-toolchain.toml` pins the version.
2. For Android: the SDK, build tools and NDK pinned in `android/*-version.txt`,
   `cargo install cargo-ndk`, Gradle 9.8 and a JDK (17+).
3. `make help` lists the targets. `make run-desktop` runs the interface in a
   desktop window while you work on it; `make build-android-debug` builds an
   APK.

Run `make monitor-dev` to start the Monitor HTTP API on port 3770.
See [monitor/README.md](monitor/README.md) for the service's installation and
configuration instructions.

### Formatting

**Do not run formatters.** The codebase has formatting that is deliberate, and a
reformat buries the actual change in noise. Match the style of the file you are
editing.

### Checks before you push

```sh
make test           # cargo test --workspace
make lint           # clippy, warnings denied
make android-check  # if you touched app/
```

All three run on every pull request (`.github/workflows/checks.yml`).

### Commit messages

One prefix, lowercase, then a short description in the imperative:

```
feat: add temperature unit setting
fix: reconnect after the tunnel drops
docs: describe the CLA check
opt.: cache the parsed disk list
rm: drop the unused sensor fallback
migrate: move network parsing into sbm_parser
```

Common prefixes: `feat:`, `fix:`, `docs:`, `opt.:`, `rm:`, `migrate:`,
`refactor:`, `test:`, `chore:`. Write commit messages and code comments in
English.

## Translations

Interface strings live in `app/l10n/<locale>.json`. A new string is read with
`i18n::t("key", "English text")` and added to `en.json`; translate by adding
the key to the other files.

- Partial translations are fine — send what you have.
- Machine-translated locales exist (marked in the README) and improvements to
  them from native speakers are especially welcome.

## Attribution

Contributors are credited in the README. If a name is missing, comment on the
issue or pull request and it will be added.
