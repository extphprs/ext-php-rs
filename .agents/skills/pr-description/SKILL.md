---
name: pr-description
description: Use when writing the title or description of an extphprs/ext-php-rs pull request (gh pr create / gh pr edit), or when asked to reformat, tidy or shorten an existing PR description, including PRs opened by other people. Defines the conventional-commit title, the house format (one-line bug + Before/After code taken from PR tests that assert the fix, inside the repository's PR template) and how to verify it.
---

# PR titles and descriptions

## Title

PRs are squash-merged, and release-plz turns the squashed commit into the
`CHANGELOG.md` entry and the version bump. So the title is a conventional commit
that reads well as a changelog line:

- `type(scope): what changes, as the user sees it`, e.g.
  `fix(array): fix non-UTF-8 keys, reverse iteration and length of iterators`,
  `feat(class): declare typed properties with \`ClassBuilder::typed_property\``.
- Types used here: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `ci`, `chore`
  (`chore(deps)` for dependency bumps). Scopes come from the existing history
  (`git log --format=%s origin/main | head -50`): `class`, `macro`, `array`, `embed`,
  `sapi`, `php-build`, `cargo-php`, `observer`, …
- A breaking change gets `!` (`feat(class)!: …`), and the PR has a migration guide
  (skill `semver-breaking-changes`).
- Code in backticks. No issue numbers in the title (GitHub appends `(#n)` on merge).

## Description format

The repository's template (`.github/PULL_REQUEST_TEMPLATE.md`) stays: a
`## Description` section and the `## Checklist`, with each box ticked only when it is
true. Inside `## Description`, as brief as possible: the reader only cares which bug
is fixed or what can now be done, shown with code. The shape (the bug below is made
up, only the layout matters):

````markdown
## Description

A `#[php_function]` that takes `Option<&str>` rejects an explicit `null` in strict mode.

**Before**
```rust
#[php_function]
pub fn greet(name: Option<&str>) -> String {
    format!("Hello, {}", name.unwrap_or("world"))
}
```
```php
<?php declare(strict_types=1);
greet(null); // TypeError: greet(): Argument #1 ($name) must be of type ?string, null given
```

**After**
```php
greet(null); // "Hello, world"
```

Fixes #1234

## Checklist

- [x] I have read the [contribution guidelines](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md).
- [x] I have added tests that prove my code works as expected.
- [ ] I have added documentation if applicable.
- [ ] I have added a [migration guide](CONTRIBUTING.md#breaking-changes) if applicable.
````

- **One sentence** naming the bug or the feature, as the extension author or the PHP
  user sees it (a wrong value, a missing exception, a crash, a leak, an API that
  didn't exist), not the implementation.
- **At most two sentences** of prose in the whole description, each in its own
  paragraph with a blank line between them. Detail goes in the commits.
- A **Before** block: the Rust declaration and the PHP script of a test of the PR that
  asserts the fix (see below), with trailing `// result` comments on the lines whose
  result matters: the value, `// TypeError: <message>`, `// segfault`,
  `// leaks 1 zend_string`. Copy the results from an actual run on `main`.
- An **After** block with **only the lines whose result changes**, commented with the
  new result.
- **A feature** (a new API, a new macro option): skip Before/After and show the Rust
  code that uses it and the PHP that shows the effect, from the PR's tests.
- **Several bugs in one PR:** one short `###` heading per bug, each with its own
  one-line summary and Before/After pair.
- **No PHP example possible** (CI, build, bindings, tooling, performance): one plain
  line, e.g. "Builds against PHP 8.6.0RC7, whose API number is 20261112." Numbers only
  if they *are* the fix ("`new` on a class with a constructor takes 0.37 s instead of
  0.70 s for 5M objects"), with the benchmark that measured them. Several numbers go
  in a Markdown table under the sentence, not in prose.
- **Breaking change:** one line saying what breaks and that the migration guide covers
  it, with a link to the guide section.
- Keep closing keywords for issues the PR fixes (`Fixes #1234`) on their own line at
  the end of `## Description`, so GitHub links and closes them.
- Leave out: background and motivation, design notes, the implementation walk-through,
  "stacked on / combines …", measurements that aren't the point, review follow-ups,
  test lists, "not run locally" notes, changelog prose, references to files that
  aren't in the repository (local notes, plans), and any AI attribution (no
  "Generated with" footer, no `Co-Authored-By` trailer in the commits).

## Every example is a test that asserts the fix

- Each Before/After example must correspond to a test of the PR, and that test must
  **assert the fix**: it fails on `main` and passes on the branch, because it checks
  exactly the result the After block shows (see "Tests must assert the change" in
  AGENTS.md for how tests silently fail to do that).
- The example is that test's code: the item in the integration module's `mod.rs` and
  the lines of its `.php` script. If the test is long, show only the lines that matter,
  as long as the test asserts what they show. Never show a snippet no test covers.
- If no test makes a short, clear example, add one to the PR first. Specific,
  easy-to-follow examples beat abstract ones.
- **Verify by running that single test** on `main` (must fail) and on the branch (must
  pass): `cargo test -p tests <test_name>`, which builds the `tests` extension and
  filters by test name. Never run the whole suite for this. Copy the results in the
  Before comments from `main`'s actual output (the PHP error message, the dumped
  value). Never write invented or "illustrative" output.
- For a crash or a leak, the Before comment names what happens (`// segfault`,
  `// ASan: heap-use-after-free`), from a real run (skill `zend-memory`).

## Reformatting an existing description (including other people's PRs)

Only when asked. Editing someone else's PR body needs maintainer rights on
extphprs/ext-php-rs; without them (`gh api repos/extphprs/ext-php-rs --jq
.permissions`), post the new text as a suggestion in a comment instead, after the
user approves it.

1. Read the current body, title and diff:
   `gh pr view <n> --repo extphprs/ext-php-rs --json title,body,author,files` and
   `gh pr diff <n> --repo extphprs/ext-php-rs`.
2. Work out the bug(s) actually fixed from the **diff and the tests it adds**, not only
   from the prose. Pick the examples from the PR's tests, and check that those tests
   assert the fix. If none does, or none makes a usable example, tell the user and
   suggest a test to add; don't push tests to someone else's PR unless asked.
3. Keep everything the author wrote that matters to reviewers in the new format: the
   bug summary, issue links / closing keywords, the checklist state, and any explicit
   note about a breaking change or a behaviour change users must know about (one line
   at the end). Drop the rest.
4. Verify the examples by running each one's test on `main` and on the PR head
   (`git fetch origin pull/<n>/head:pr-<n>` and a separate worktree).
5. Show the new body to the user before replacing someone else's. Then:
   `gh pr edit <n> --repo extphprs/ext-php-rs --body-file <file>`. Don't change the
   title unless asked; if it isn't a valid conventional commit or is misleading,
   suggest a new one.
6. Don't touch the code, labels or reviews of the PR as part of this.
