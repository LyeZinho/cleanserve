# Design: Move Internal Pages to `.cleanserve/pages/`

**Date:** 2026-03-21  
**Status:** Approved  
**Objective:** Move 404, error, and HMR test pages from `public/` to `.cleanserve/pages/` to avoid polluting user's project directory while enhancing functionality.

## Problem Statement

Currently, CleanServe generates `404.html` and `error.html` directly in `public/`, which:
- Pollutes the user's project directory with internal CleanServe files
- Makes the `public/` folder confusing (which files are user's vs. system?)
- Could interfere with user's existing 404 routing logic
- Doesn't include helpful context (project name, error details)

## Solution Overview

Move all internal pages to `.cleanserve/pages/` during initialization:
- **404.html** — Served when static file not found (respects user PHP routing)
- **error.html** — Served on PHP runtime errors with error context
- **hmr-test.html** — Internal page for HMR testing (hidden from users)

## Architecture

### Directory Structure

```
.cleanserve/
├── pages/
│   ├── 404.html          # Static file not found
│   ├── error.html        # PHP runtime errors
│   └── hmr-test.html     # Internal HMR testing (hidden)
├── php/                  # Existing PHP runtime
└── ... (other files)
```

### File Serving Flow

1. **Request to `/something`:**
   - Proxy checks if static file exists in `public/`
   - If NOT found → Serve `.cleanserve/pages/404.html`
   - If found → Serve the static file

2. **PHP Error Occurs:**
   - Error overlay injects error details
   - Falls back to `.cleanserve/pages/error.html` with context

3. **HMR Test Page:**
   - NOT exposed to users
   - Used internally for debugging HMR WebSocket connection
   - Accessible only through direct development

### Page Enhancements

**404.html:**
- Keep: Brutal design (zinc-950 bg, emerald accents)
- Keep: Large "404" display
- Keep: HTTP response example
- Add: "Requested path: `/path-user-tried`"
- Add: Link back to homepage

**error.html:**
- Keep: Brutal design aesthetic
- Add: Full error details (file, line, message)
- Add: Stack trace with code context
- Add: Project name in header
- Add: PHP version in footer
- Add: "Refresh to retry" button

**hmr-test.html:**
- Shows WebSocket connection status (connected/disconnected)
- Displays HMR events in real-time (PHP reload, CSS updates)
- Shows request/response details for debugging
- Internal testing only, not for user-facing display

## Implementation Strategy

### Phase 1: Page Generation
1. Enhance `html_pages.rs`:
   - Improve `generate_default_404()` with path context
   - Improve `generate_default_error()` with error details
   - Add `generate_hmr_test_page()` function

### Phase 2: Initialization
1. Update `init/mod.rs`:
   - Create `.cleanserve/pages/` directory during `cleanserve init`
   - Call new functions to write pages to `.cleanserve/pages/`
   - Keep `index.html` in `public/` only

### Phase 3: Proxy Integration
1. Update `server.rs`:
   - Change 404 handler to serve from `.cleanserve/pages/404.html`
   - Change error handler to serve from `.cleanserve/pages/error.html`
   - Pass request path/error context to pages

## Non-Goals

- Don't interfere with user's PHP routing (only serve 404.html for truly missing files)
- Don't break existing projects (404.html/error.html in `public/` are optional)
- Don't require runtime generation (all pages generated at init time)

## Success Criteria

- ✅ `.cleanserve/pages/` directory created during `cleanserve init`
- ✅ Pages are enhanced with helpful context
- ✅ `public/` contains only `index.html` (user's app entry point)
- ✅ User PHP routing is never interfered with
- ✅ 404 and error pages display correctly in browser
- ✅ HMR test page works internally but isn't exposed to users
