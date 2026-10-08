# Contributing

[简体中文](CONTRIBUTING.zh-CN.md) · **English**

The project uses Rust, React/TypeScript, Tauri, and Python. Package manifests and lockfiles define dependency versions. Development uses Node 24, npm 11, and Python 3.12 or later; rustup reads the Rust version from `rust-toolchain.toml`.

## Getting started

Run from the repository root:

```powershell
npm ci --prefix ui
npm run dev --prefix ui
```

For desktop debugging, run `npm run tauri --prefix ui -- dev`. Prepare the platform dependencies described in the [environment guide](docs/development/environment.md) first. Run `uv sync --locked` when developing the Python tools. Interface-only development does not require Python, official archives, or an ECU compiler.

## Checking changes

```powershell
npm run lint --prefix ui
npm run test --prefix ui
npm run build --prefix ui
cargo test --locked --manifest-path core/Cargo.toml
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
uv run --locked python -m autosar_tooling quality --base <baseline-commit>
```

Default Cargo tests do not require official resources or controlled native tools. See the [testing guide](docs/development/testing.md) for real-process tests, official comparisons, and desktop acceptance. Check formatting on changed lines rather than reformatting historical code wholesale.

Changes to parsing, generation, or runtime behavior should have independently defined expectations and cover key rejection paths. Follow the repository's MISRA guidance when modifying C or C-generation templates. Document input, output, and error contracts for public C interfaces.

## Assets and submissions

After modifying delivery assets, run `uv run --locked python -m autosar_tooling assets check`. Once changes to project-owned source have been reviewed, run `assets update` explicitly if needed. Do not automatically accept BSW ABI or third-party identity changes through source-digest updates; see [asset maintenance](docs/maintainers/assets.md).

Review the entire diff before submitting, including new files and generated content. Use a title that describes the actual change. Explain the problem, changes, verification, and untested scope in the PR. Include actual screenshots for interface changes. Do not commit official downloads, personal configuration, or temporary test artifacts. Limit conclusions from host tests to host capabilities.

Use an isolated environment for real GUI acceptance without occupying a developer's active desktop. BMad tracks project specifications and status; it is not a prerequisite for starting or modifying the project.

## Interface text and languages

Frontend text is organized by domain under `ui/src/i18n/`. Backend text uses `core/src/messages.json` as its single source: Cargo generates static Rust templates at build time, and the frontend imports the same resource directly. Use stable semantic keys for new product messages. Maintain complete Chinese and English sentences with matching interpolation parameters. Do not translate by matching original sentences or replacing DOM content.

Controllers and IPC retain message keys and parameters; translate at display time so old messages do not remain in the previous language. Preserve AUTOSAR identifiers, user data, and external-tool evidence in their original form. Language preferences must not affect project fingerprints, generated content, or result invalidation. After changing text, run the normal npm/Cargo checks and exercise success, failure, and narrow-window scenarios in both languages.

Windows MSI installers are built separately with `zh-CN` and `en-US` interfaces. The English WiX resource, `src-tauri/wix/locales/en-US.wxl`, uses code page 936 to accommodate the existing Chinese product identity. Do not restore the default code page 1252: characters in the brand name and installation path would prevent the English MSI from linking. Installer language is independent of the application's saved language preference.

Reader-facing root documents use English as the default, with Chinese versions named using the `.zh-CN.md` suffix. When editing bilingual documents, keep features, commands, limitations, and links aligned in both versions. Internal specifications and technical documents in subdirectories retain their existing language.

## Licensing and security reports

Original contributions are provided under [Apache-2.0](LICENSE); third-party content retains its original license. Confirm redistribution rights before adding dependencies or code. See the [licensing guide](docs/maintainers/licensing.md) for distribution requirements. Report security issues as described in [SECURITY.md](SECURITY.md). For ordinary issues, include reproduction steps and expected results.
