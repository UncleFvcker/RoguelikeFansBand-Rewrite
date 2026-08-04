# Project Working Rules

- The project is in active development and test sessions always start from a new save. Do not add or preserve compatibility for older development saves unless the user explicitly requests it.
- Use focused tests, content validation, type checking, and relevant builds for routine changes. Do not run the full desktop or other large E2E suites by default; run them only when investigating a related failure, when the user requests them, or for an explicit milestone acceptance pass.
- When producing a playable desktop build, use the standalone Tauri build command. A plain Cargo build may still depend on the Vite development server.
- Keep contract fixtures focused on one minimal behavior. Do not add movement commands when movement is not the subject; use the direct player-position precondition for location-dependent actions, and do not combine visits to multiple facilities in one fixture.
- Generic shop purchase fixtures should select the first projected stock entry instead of binding a generated item instance ID. Bind a specific item only when that item's identity or behavior is the subject of the test.
- For routine contract changes, run `rfb-contract verify-category` only for the affected fixture categories. Use `verify-all`, `refresh-all`, or the ignored full replay test only for shared protocol projections, global content/state hashes, common initialization or RNG changes, and explicit milestone acceptance.
