# php-runtimes Submodule

CleanServe downloads PHP from [LyeZinho/php-runtimes](https://github.com/LyeZinho/php-runtimes). The same repository is vendored in this repo as a **git submodule** at `php-runtimes/`, so you can change runtime builds and the CLI in one workspace.

## Clone with submodules

```bash
git clone --recurse-submodules git@github.com:LyeZinho/cleanserve.git
cd cleanserve
```

If you already cloned without submodules:

```bash
git submodule update --init --recursive
```

## Day-to-day workflow

| Task | Command |
|------|---------|
| Update submodule pointer | `git submodule update --remote php-runtimes` |
| Work on runtime builds | `cd php-runtimes` → edit → commit → push **php-runtimes** repo |
| Point cleanserve at new runtime commit | `cd ..` → `git add php-runtimes` → commit **cleanserve** |

## Building PHP locally

From `php-runtimes/` (see that repo’s README):

```bash
cd php-runtimes
cargo run -- build <version> linux-x64
```

After changing `get_configure_flags()` in `src/curator/build.rs`, rebuild, publish a GitHub release, and refresh `manifests/versions.json` so `cleanserve update` serves the new binaries.

## Composer-required extensions

CLI builds must include at least:

- `--enable-phar`
- `--enable-tokenizer`
- `--enable-ctype`
- `--enable-filter`
- `--enable-xml` / `--with-libxml`

These are enabled in the submodule’s `get_configure_flags()` for new releases.
