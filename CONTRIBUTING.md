# Contributing

[简体中文](CONTRIBUTING.zh-CN.md) · **English**

## Contribution policy

Development is led by the maintainers. Unsolicited external pull requests are not accepted and will be closed without code review. Maintainers and invited collaborators use the PR workflow described below. Bug reports and feature requests are welcome through Issues; report security vulnerabilities as described in [SECURITY.md](SECURITY.md).

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
cargo test --locked -p autosar-config-core
cargo clippy --locked -p autosar-config-core --all-targets --all-features -- -D warnings
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
uv run --locked python -m autosar_tooling quality --merge-base origin/master
```

Run `uv run --locked pre-commit install --allow-missing-config` once; the last `quality` command above then runs before every push and stops a push that fails it. CI still runs the same check. `--allow-missing-config` lets older branches without the configuration, including worktrees that share the same `.git/hooks`, push as before.

Default Cargo tests do not require official resources or controlled native tools. See the [testing guide](docs/development/testing/README.md) for real-process tests, official comparisons, and desktop acceptance. Check formatting on changed lines rather than reformatting historical code wholesale. Runtime C already follows `.clang-format`; run `git config blame.ignoreRevsFile .git-blame-ignore-revs` once so `git blame` skips the bulk reformat. Emitters write generated C in the same layout, and a required check verifies the generated samples.

Changes to parsing, generation, or runtime behavior should have independently defined expectations and cover key rejection paths. Generation code renders each output once and reports premises the plan cannot guarantee as diagnostics; Clippy rejects panics and text patching there, see the testing guide. Follow the [C coding standard](docs/development/c-coding-standard.md) (Chinese) and the repository's MISRA guidance when modifying C or C-generation templates. Document input, output, and error contracts for public C interfaces.

## Assets and submissions

After modifying delivery assets, run `uv run --locked python -m autosar_tooling assets check`. Runtime digests are computed at build time; `assets update` only updates the independent seal-guard inventory. Review BSW ABI and third-party identities separately. Fixed checkout line endings are checked by the asset tests; see [asset maintenance](docs/maintainers/assets.md).

Review the entire diff before submitting, including new files and generated content. Write commit messages as [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>): <description>`, where the type is `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci` or `chore`, the scope names the affected module or area (such as `dcm`), and a breaking change adds `!` and a `BREAKING CHANGE:` footer. Commit in stages that can be verified independently, and commit changes to repository rules and process documents separately. Use a title that describes the actual change. Explain the problem, changes, verification, and untested scope in the PR, and map each acceptance criterion of the Issue, and each item it hands over, to its evidence (test, sample or document), or to the reason and the Issue that takes it over; keep the full evidence in the PR and write only the conclusion and links in the Issue. Review the whole diff yourself before merging; check every AUTOSAR conclusion against the original requirement text rather than memory or existing code, and give high-risk changes one independent review that runs alongside CI. When a required check fails, look at it right away instead of waiting for the remaining jobs. A PR may merge once the required checks pass: after self-review, enable auto-merge with `gh pr merge --auto --merge`, which merges when `CI required` passes and deletes the branch; Extended checks do not block merging, when the latest completed Extended run on master has failed, first decide whether the failure relates to your PR: if it does not, make sure an Issue tracks it (pausing an unstable suite through an Issue) and merge as usual, leaving the fix or revert to the PR that introduced it; if it does, or you cannot tell, investigate before merging. Include actual screenshots for interface changes. Do not commit official downloads, personal configuration, or temporary test artifacts. Limit conclusions from host tests to host capabilities.

Add a `CHANGELOG.md` entry under `Unreleased` for every change visible to users or integrators: delivered C interfaces and ABI, configuration acceptance or rejection, generated output, tool and interface behavior. Refactoring, tests, CI and documentation get no entry. Write one line per change that states the effect and ends with the PR number, such as `(#166)`; details belong in the PR. Group entries under `Breaking`, `Added`, `Changed` and `Fixed`; a breaking or contract change states what integrators must change. At release, `Unreleased` becomes the version heading with its date. The changelog is maintained in English only.

Use a dedicated branch for each Issue, and when you start, create it with `gh issue develop <N> --base master --name <branch>` so it is linked to the Issue and others can see the work is in progress. A separate worktree is optional when the main checkout is available; use separate worktrees to isolate Issues developed in parallel.

Use an isolated environment for real GUI acceptance without occupying a developer's active desktop. Work is tracked in GitHub Issues and Milestones; before opening an Issue or starting work, search existing Issues (including closed ones) and open PRs, and extend what already covers it instead of duplicating work; when a change also completes part of another Issue, comment there with the PR, and close it with `Closes #N` once it is fully done; see [docs/roadmap/](docs/roadmap/README.md) for direction and [docs/architecture/](docs/architecture/README.md) for design constraints.

Every open Issue carries at least one type label and one area label; a cross-module `tracking` overview needs only its type label. Type labels are `bug`, `known-gap` (a capability, standard or verification gap), `tech-debt`, `enhancement`, `question`, `story` or `tracking` (a stage overview). An Issue may have several area labels:

- `area:os`: OS runtime, OS configuration generation and target ports
- `area:bsw`: BSW module runtime and configuration
- `area:rte`: RTE/SchM generation and component contracts
- `area:generator`: configuration model, ARXML reading and writing, definition catalog and the C generation mainline
- `area:ui`: workbench interface, interaction and text
- `area:desktop`: Tauri desktop backend, installers and desktop scenario tests
- `area:quality-ci`: CI, `quality`, C static checks and test infrastructure
- `area:delivery`: delivery packages, asset inventory and offline ECU tools

Add `blocked` when an Issue waits for target hardware or another Issue, and name what it waits for in the body. Progress is shown by the Milestone and the linked branch or PR, not by status labels; pull requests get area labels automatically from their changed paths (the `Pull request labels` workflow, mapped in `.github/labeler.yml`); work done in parallel should avoid sharing an `area`.

## Interface text and languages

Frontend text is organized by domain under `ui/src/i18n/`. Backend text uses `core/src/messages.json` as its single source: Cargo generates static Rust templates at build time, and the frontend imports the same resource directly. Use stable semantic keys for new product messages. Maintain complete Chinese and English sentences with matching interpolation parameters. Do not translate by matching original sentences or replacing DOM content.

Controllers and IPC retain message keys and parameters; translate at display time so old messages do not remain in the previous language. Preserve AUTOSAR identifiers, user data, and external-tool evidence in their original form. Language preferences must not affect project fingerprints, generated content, or result invalidation. After changing text, run the normal npm/Cargo checks and exercise success, failure, and narrow-window scenarios in both languages.

Windows MSI installers are built separately with `zh-CN` and `en-US` interfaces. The English WiX resource, `src-tauri/wix/locales/en-US.wxl`, uses code page 936 to accommodate the existing Chinese product identity. Do not restore the default code page 1252: characters in the brand name and installation path would prevent the English MSI from linking. Installer language is independent of the application's saved language preference.

Reader-facing root documents use English as the default, with Chinese versions named using the `.zh-CN.md` suffix. When editing bilingual documents, keep features, commands, limitations, and links aligned in both versions. `CHANGELOG.md` is English only. Internal specifications and technical documents in subdirectories retain their existing language.

## Documentation layout

Each document covers one topic. When a document grows beyond about 20 KB, or contains several topics that can be read independently, split it into a directory of the same name with a `README.md` index that gives each sub-document a one-line purpose; keep a link to the index at the original location. Version-appended changelogs, single-decision ADRs, and archives kept only for traceability are exempt; `quality` checks the other Markdown files against 20 KiB (counting LF line ends). Put new content into the existing document or directory that fits instead of starting a parallel file.

## Licensing and security reports

Original contributions are provided under [Apache-2.0](LICENSE); third-party content retains its original license. Confirm redistribution rights before adding dependencies or code. See the [licensing guide](docs/maintainers/licensing.md) for distribution requirements. Report security issues as described in [SECURITY.md](SECURITY.md). For ordinary issues, include reproduction steps and expected results.
