# CleanServe API Reference Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create two comprehensive, 250+ line API reference markdown files for the CleanServe Package Manager and Auto-updater.

**Architecture:** Documentation-as-code approach, ensuring all Rust types and module signatures are accurately reflected in the markdown files. The files follow a structured "Overview -> Core Types -> Modules -> Examples -> Errors" flow.

**Tech Stack:** Markdown, Rust (for type reference), Git.

---

### Task 1: Initialize Documentation Files

**Files:**
- Create: `docs/api/package-manager-api.md`
- Create: `docs/api/auto-updater-api.md`

**Step 1: Create the directory if it doesn't exist**

Run: `mkdir -p docs/api`
Expected: Directory `docs/api` exists.

**Step 2: Create initial empty files**

Run: `touch docs/api/package-manager-api.md docs/api/auto-updater-api.md`
Expected: Files created.

**Step 3: Commit**

```bash
git add docs/api/
git commit -m "docs: initialize api reference files"
```

---

### Task 2: Implement `package-manager-api.md` (250+ lines)

**Files:**
- Modify: `docs/api/package-manager-api.md`

**Step 1: Write Overview and Core Types sections**
Include full structs for `Package`, `PackageVersion`, `DownloadInfo`, `PackageRuntime`, and `RuntimeStatus` with field descriptions.

**Step 2: Write Modules and Methods sections**
Detailed breakdown of `PackageRegistry`, `PackageDownloader`, `PackageCache`, and `PackageLifecycle` with signature definitions.

**Step 3: Write Usage Examples and Error Handling**
Provide a complete async example of installing/starting Redis. Define `PackageManagerError` variants.

**Step 4: Verify line count and accuracy**
Run: `wc -l docs/api/package-manager-api.md`
Expected: 250+ lines.

**Step 5: Commit**

```bash
git add docs/api/package-manager-api.md
git commit -m "docs: implement package manager api reference"
```

---

### Task 3: Implement `auto-updater-api.md` (250+ lines)

**Files:**
- Modify: `docs/api/auto-updater-api.md`

**Step 1: Write Overview and Lifecycle sections**
Explain the atomic update process (Check -> Download -> Backup -> Install -> Verify).

**Step 2: Write Core Types and Module Specifications**
Include `UpdateInfo`, `UpdateCheckerError`. Document `UpdateChecker`, `BinaryDownloader`, and `UpdateInstaller`.

**Step 3: Write Platform Support and Usage Examples**
List target triples. Provide a robust update loop example with rollback logic.

**Step 4: Verify line count and accuracy**
Run: `wc -l docs/api/auto-updater-api.md`
Expected: 250+ lines.

**Step 5: Commit**

```bash
git add docs/api/auto-updater-api.md
git commit -m "docs: implement auto-updater api reference"
```

---

### Task 4: Final Review and Linking

**Files:**
- Modify: `docs/README.md`

**Step 1: Update README to link new files**
Ensure the API Reference section points to the new files.

**Step 2: Final verification of all links**
Verify that links within the new files (if any) are valid.

**Step 3: Commit**

```bash
git add docs/README.md
git commit -m "docs: link new api reference files in README"
```
