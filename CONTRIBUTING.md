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
uv run --locked python -m autosar_tooling quality --scope all --merge-base origin/master
```

To select regular local checks from the actual changes, use:

```sh
uv run --locked python -m autosar_tooling verify --scope changed --base origin/master
uv run --locked python -m autosar_tooling verify --scope changed --base origin/master --list
```

The second command lists the checks without running tools or creating output. Selection shares
CI's rules and includes committed, staged, unstaged and untracked changes; renames and unknown
inputs select the full set. The changed scope uses one `native-tests` feature set for core tests
and fresh samples shared within that invocation. Existing explicit scopes keep their behavior.
See the [local feedback guide](docs/development/testing/feedback.md) for the selected checks and
the additional C, OS, sanitizer and desktop acceptance required by each task.

A shared compiler cache is optional. After installing `sccache`, a developer can try
`RUSTC_WRAPPER=sccache CARGO_INCREMENTAL=0` in the environment of a Cargo or verification command,
then inspect `sccache --show-stats`. The [Rust cache constraints](https://github.com/mozilla/sccache/blob/main/docs/Rust.md)
include incremental compilation, linker outputs and working-directory identity; measure actual
hits for the chosen worktrees. The default commands require no cache service. Keep each
worktree's Cargo target separate for concurrent builds, and reuse the normal uv cache.

Run `uv run --locked pre-commit install --allow-missing-config` once; the last `quality` command above then runs before every push and stops a push that fails it. CI still runs the same check. `--allow-missing-config` lets older branches without the configuration, including worktrees that share the same `.git/hooks`, push as before.

`quality` reads working-tree content at indexed and non-ignored untracked paths, including unstaged edits. Hygiene checks run before changed-line filtering; `--base` and `--merge-base` select the comparison baseline, not a source snapshot. The installed pre-push hook uses pre-commit to temporarily set aside tracked unstaged edits and restore them afterwards; untracked files still participate. A clean worktree can mask invalid staged content in a direct run, and a staged fix can mask invalid content in the commit being pushed. Commit intended fixes before validating the push.

When local changes must stay untouched, validate and push from a clean checkout of the exact commit being pushed. For example, after committing intended changes on the topic branch, run the following from the original checkout, replacing the destination with an unused directory outside it:

```sh
git worktree add --detach ../autsaro-quality-check HEAD
cd ../autsaro-quality-check
uv sync --locked
npm ci --prefix ui
uv run --locked python -m autosar_tooling quality --scope all --merge-base origin/master
git push origin HEAD:refs/heads/<topic-branch>
```

Keep the normal pre-push hook enabled; a linked worktree shares Git hooks. This checkout contains committed bytes only: it excludes local staged fixes as well as unrelated edits, so a violation in the pushed commit remains visible. After the push succeeds, return to the original checkout and remove the temporary worktree with `git worktree remove ../autsaro-quality-check`.

IPC declarations come from the Rust command signatures and their serialized DTOs. After changing either, run `npm run ipc:generate --prefix ui` and commit both generated bindings under `ui/src/ipc/`; `npm run ipc:check --prefix ui` compares freshly generated bytes without modifying tracked files. Both use the locked Cargo generator and the project's locked Prettier, so run `npm ci --prefix ui` first. The default and `native-webdriver` signatures are checked separately; the normal TypeScript build also rejects invalid command names, payloads and result assumptions. Keep Serde attributes and the native transport unchanged when exporting types.

Default Cargo tests do not require official resources or controlled native tools. See the [testing guide](docs/development/testing/README.md) for real-process tests, official comparisons, and desktop acceptance, and [verification and review](docs/development/testing/verification-and-review.md) for entry investigation, behavior-falsification, mutation checks, and independent review rules. Exercise behavior changes from normal entries across success, failure, and key state changes (never weaken assertions, relax deadlines, retry, or expand exclusions to pass), and reuse shared subprocess runners that attach stdout/stderr. Check formatting on changed lines across all configured tools (including fixtures and Rust tests) rather than reformatting historical code wholesale. Runtime C already follows `.clang-format`; run `git config blame.ignoreRevsFile .git-blame-ignore-revs` once so `git blame` skips the bulk reformat. Emitters write generated C in the same layout, and a required check verifies the generated samples.

Changes to parsing, generation, or runtime behavior should have independently defined expectations and cover key rejection paths. Generation code renders each output once and reports premises the plan cannot guarantee as diagnostics; Clippy rejects panics and text patching there, see the testing guide. Follow the [C coding standard](docs/development/c-coding-standard.md) (Chinese) and the repository's MISRA guidance when modifying C or C-generation templates. Document input, output, and error contracts for public C interfaces.

Install the message hook with `uv run --locked pre-commit install --hook-type commit-msg --allow-missing-config`.
It checks the current message with the same rules as `quality`: lowercase types from the list
below, an optional scope naming the affected module or area, and `!` with a `BREAKING CHANGE:` footer.
Titles and bodies reject all nine GitHub closing keywords followed by `#N`, `owner/repo#N` or an
Issue URL, including uppercase keywords and optional colons. Use `Refs #N` for references.
Merge commits and generated `git revert` messages are skipped. `quality --merge-base origin/master`
checks every new commit after the fork; existing history is left unchanged. CI checks the real PR
head against its base, while master pushes check the newly introduced `before..HEAD` range.
A first push checks all reachable commits; a manual CI run uses its required baseline input.

## Assets and submissions

After modifying delivery assets, run `uv run --locked python -m autosar_tooling assets check`. Runtime digests are computed at build time; `assets update` only updates the independent seal-guard inventory. Review BSW ABI and third-party identities separately. Fixed checkout line endings are checked by the asset tests; see [asset maintenance](docs/maintainers/assets.md).

Review the entire diff before submitting, including new files and generated content. Write commit messages as [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `<type>(<scope>): <description>`, where the type is `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci` or `chore`, the scope names the affected module or area (such as `dcm`), and a breaking change adds `!` and a `BREAKING CHANGE:` footer. Describe the actual capability or fix in English without internal task IDs. When a commit refers to an Issue, omit closing keywords (close, fix, resolve): GitHub closes Issues on merge to master. State closing relations only in the PR body: `Closes #N` when fully done, `Refs #N` for partial delivery (verify `closingIssuesReferences` and unlink whole-issue closes on intermediate PRs). Commit in independently verifiable stages (module remediation: definitions/validation, runtime/generator, dispositions/inventories); commit repository rules/process docs separately and list them in the PR body. Use a descriptive title. In the PR, explain the problem, changes, verification, and untested scope, mapping each acceptance criterion and handoff item to evidence (test, sample, or doc) or its follow-up Issue and reason; keep full evidence in the PR (body and final comment), and put only conclusions, links, and relevant parts in Issues without repeating evidence.

Self-review the whole diff before merging without a human approval/wait gate; verify AUTOSAR conclusions against original spec text (ID, signature, return/error semantics, state transitions, disposition tags), and run one parallel independent review for high-risk changes (module remediation, delivery layout/ABI, concurrency/exclusive areas; see [verification and review](docs/development/testing/verification-and-review.md)). The full flow covers branch, implementation, verification, commit, PR, review, required CI, merge, and target-branch check. Inspect failing required checks immediately. In a single-maintainer repository, run `gh pr merge --auto --merge` only after independent review confirms no unresolved P0–P2 findings for high-risk or core changes, or after small changes pass a complete self-review. Sync first if conflicting with master. Required checks gate the automatic merge and branch deletion. Wait for post-merge suites only when they are Issue exit evidence (record results back), and confirm PR merge before deleting the worktree or checking off the overview (without merge authorization, stop at the final PR comment for the merger). Every master push runs the full Development matrix and selects Extended checks against pushed inputs; `Post-merge checks result` summarizes failures and opens an `area:quality-ci` Issue. Check the latest completed master result before opening or merging a PR: continue if an unrelated failure is already tracked, otherwise investigate first. The introducing author fixes confirmed regressions; maintainers revert changes breaking delivery or critical master behavior. Attach screenshots for UI changes; never commit official downloads, personal config, or temporary test artifacts. Limit host-test conclusions to host capabilities.

Add a `CHANGELOG.md` entry under `Unreleased` for user- or integrator-visible changes: delivered C interfaces and ABI, configuration acceptance/rejection, generated output, tool and UI behavior (not refactoring, tests, CI, or docs). Write one line per change ending with `(#PR)`, grouped under `Breaking`, `Added`, `Changed`, and `Fixed` (state required integrator changes for breaking/contract changes). At release, `Unreleased` becomes the dated version heading. English only.

Before process, tool, or design decisions, check mature similar projects' established practices first, pick an efficient and reliable approach, and state concrete reasons if deviating. Follow current explicit delegation for in-scope design/ABI/delivery choices; major contracts, roadmap, releases, legal commitments, protection bypasses, and out-of-scope work require reserved authorization (see [verification and review](docs/development/testing/verification-and-review.md)).

Track work in GitHub Issues and Milestones (create Milestones only for committed stages; `planning-archive-2026-10` is traceability-only; status queries are read-only; no extra agent orchestration, reports, matrices, or ledgers). One Issue = one PR: split multi-PR work into GitHub child Issues before starting (`Closes <child>` per PR, `Refs` only for parent Issues), write scope as a countable checklist, update a status table at the top of the parent Issue in place, and use comments only for decisions, blockers, and scope changes (file a new child Issue instead of expanding scope). Before filing an Issue, implementing, or adding rules, search existing Issues (including closed), open PRs, and branches, extending or cross-referencing with a single source of truth. Read the Issue's scope, dependencies, entry conditions, acceptance criteria, comments, PRs, and Milestone `tracking` rules, and check current code so completed work is not redone; key module Issues wait for dependencies to merge before pre-investigation. When entry conditions are unmet, investigate and comment evidence before coding ([verification and review](docs/development/testing/verification-and-review.md)).

Use a dedicated branch per Issue via `gh issue develop <N> --base master --name <branch>` (also when a local same-name worktree branch exists; push fast-forwards), linking child Issues so merging closes them directly. Name branches by feature/fix before linking (re-link and delete old remote branch if renamed/split); isolate parallel Issues in separate worktrees. When completing part of another Issue, comment there with the PR and check off its checklist (close with `Closes #N` or verified reason when complete). File `known-gap`/`tech-debt`/`question` Issues (basis, impact, exit criteria) for out-of-scope gaps—including shortcuts valid only under narrowed config (e.g., static delivery files hardcoding generator-chosen names)—into the owning Milestone or unscheduled. Write an ADR before changing roadmap order, product contracts, or support boundaries, update [docs/roadmap/](docs/roadmap/README.md) and [docs/architecture/](docs/architecture/README.md), and update only the status of superseded ADRs.

Use an isolated environment for real GUI acceptance without occupying a developer's active desktop. Full native GUI/IPC scenarios run on master. Extended runs these scenarios only on manual dispatch. An analysis-selected PR still produces the eight Linux/Windows receipts and inputs for [rebinding source acceptance records](docs/development/testing/source-acceptance.md).

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

README introduces capabilities, entry points, limits, and real screenshots—no progress logs, agent narratives, design explanations, or fix histories (evidence in PRs, conclusions in Issues/Milestones, roadmap shifts in `docs/roadmap/`). Sync guides on usage/support changes (checking redundancy, hierarchy, and bilingual alignment); dev docs target regular contributors (mark special environment/collaboration needs conditional). Distill general requirements without personal, organizational, or project-origin attribution unless requested.

Proactively write confirmed, reusable workflow changes and high-cost lessons into repo docs (never leave only in session memory): keep [AGENTS.md](AGENTS.md) as a cross-task directory under 8 KB, and put details here or in topic docs (`<= 20 KiB`), updating its "Read Before Task" table when topic docs change. Remove or rewrite obsolete rules, consolidate instead of appending blindly, and never relax acceptance, data-protection, or authorization boundaries.

## Licensing and security reports

Original contributions are provided under [Apache-2.0](LICENSE); third-party content retains its original license. Confirm redistribution rights before adding dependencies or code. See the [licensing guide](docs/maintainers/licensing.md) for distribution requirements. Report security issues as described in [SECURITY.md](SECURITY.md). For ordinary issues, include reproduction steps and expected results.
