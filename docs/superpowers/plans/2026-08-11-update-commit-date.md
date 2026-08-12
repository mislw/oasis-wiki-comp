# Update Commit Date Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show the latest GitHub commit date as a local calendar date while preserving the existing one-click Skill update behavior.

**Architecture:** Extend the update status contract with an ISO commit timestamp. The Rust updater extracts and persists it from GitHub's existing commit endpoint, then React formats it as `YYYY-MM-DD` using the local timezone.

**Tech Stack:** Rust, Serde, Tauri 2, React 19, TypeScript, Vite.

---

### Task 1: Persist the GitHub Commit Timestamp

**Files:**
- Modify: `src-tauri/src/updater.rs`
- Modify: `src-tauri/src/config/schema.rs`
- Test: `src-tauri/src/updater.rs`

- [ ] **Step 1: Write the failing deserialization test**

```rust
#[test]
fn reads_latest_commit_author_date() {
    let commit: GithubCommit = serde_json::from_str(
        r#"{\"sha\":\"b3a5997\",\"html_url\":\"https://github.com/mislw/oasis-wiki/commit/b3a5997\",\"commit\":{\"author\":{\"date\":\"2026-08-11T06:30:00Z\"}}}"#,
    )
    .unwrap();

    assert_eq!(commit.commit.author.date, "2026-08-11T06:30:00Z");
}
```

- [ ] **Step 2: Run the focused test and verify it fails because `GithubCommit` has no `commit` field**

Run: `cargo test reads_latest_commit_author_date`

Expected: compilation fails with a missing `commit` field error.

- [ ] **Step 3: Add the nested GitHub commit structs and `latest_revision_date` status/config fields**

```rust
#[derive(Debug, Deserialize)]
struct GithubCommit {
    sha: String,
    html_url: String,
    commit: GithubCommitDetails,
}

#[derive(Debug, Deserialize)]
struct GithubCommitDetails {
    author: GithubCommitAuthor,
}

#[derive(Debug, Deserialize)]
struct GithubCommitAuthor {
    date: String,
}
```

Set `latest_revision_date` from `commit.commit.author.date` for commit-based status, and save it whenever update status is stored.

- [ ] **Step 4: Run Rust tests and verify they pass**

Run: `cargo test`

Expected: all tests pass.

### Task 2: Display a Local Calendar Date

**Files:**
- Modify: `src/types.ts`
- Modify: `src/windows/Settings.tsx`
- Test: `npm run build`

- [ ] **Step 1: Add `latest_revision_date` to the TypeScript Settings and UpdateStatus contracts**

```ts
latest_revision_date: string | null;
```

- [ ] **Step 2: Use the persisted field during reload and format it in the update summary**

```ts
if (update?.latest_revision_date) {
  return `Latest commit: ${formatCommitDate(update.latest_revision_date)}`;
}
```

`formatCommitDate` returns a local `YYYY-MM-DD` value, or the original ISO string when it cannot parse a date.

- [ ] **Step 3: Run the production build and verify it passes**

Run: `npm run build`

Expected: TypeScript type-checking and Vite build both complete successfully.
