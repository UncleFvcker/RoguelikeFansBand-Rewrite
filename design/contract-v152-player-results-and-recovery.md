# Contract v152: player results and recovery

Status: Phase 17 Gate 4 complete.

Contract v152 gives authoritative death, Warrens victory, and retirement their own player-facing result flow. It also makes recoverable and corrupt native saves actionable before session creation. Protocol remains `1.123`, the save container remains v1, state hash Schema remains `55`, and the built-in content remains `1.142.0` with hash `dd7a374770e13e923ac7c2be0648e3fea2793bcec5b78c81adf90f3d30783c36`. The active baseline remains 455 exact fixtures with zero waivers.

## Authoritative result selection

The frontend derives presentation only from existing authoritative fields:

- `player.isDead` selects the death result and takes priority over campaign presentation;
- `campaign.status == victorious` selects the victory-return result;
- `campaign.status == retired` selects the final retirement result;
- active, living play has no result overlay.

The result shows career, known session seed, turn, score, conquered dungeons, and completed tasks. A same-update `combat.player-death`, player status/item death, `campaign.victorious`, or `campaign.retired` event supplies the specific detail. A loaded terminal save has no event batch, so it uses an honest localized state summary instead of inventing a cause.

## Result actions

| State | Continue return | Restart same setup | New setup | Load | Main menu | Exit |
| --- | --- | --- | --- | --- | --- | --- |
| victorious | yes | if the current session request is known | yes | yes | yes | yes |
| dead | no | if the current session request is known | yes | yes | yes | yes |
| retired | no | if the current session request is known | yes | yes | yes | yes |

Victory remains a playable core state. Its result appears once per loaded or newly reached victorious state; choosing Continue dismisses the overlay for the normal upward route and it does not reopen on every movement command. Retirement then replaces it with the terminal result. Death and retirement remain blocking because `AppState.commandBlocked` already follows authoritative death/retirement.

Restart uses the exact active `{ buildId, seed }` and replaces both the native session and replay segment. Saves created before a session seed was persisted do not expose that seed, so their result page disables Restart same setup with a localized explanation; New setup remains available and begins with a newly generated seed.

## Shell and native-save recovery

Result navigation reuses the existing pre-session shell rather than creating a parallel menu implementation. New setup, native-load view, title, and exit stay outside gameplay command dispatch.

The title/load view now:

- resolves Warrens location metadata through its existing content localization key;
- labels a recoverable entry with the exact backup ordinal and provides a Recover action;
- keeps corrupt entries visible and provides a confirmed Delete corrupt save action;
- reports deletion or storage failure in the shell without creating a throwaway game session.

## Compatibility and evidence

No core rule, content definition, protocol DTO, save payload, replay format, state-hash field, or contract fixture result changes. Fixtures 32 and 455 remain the authoritative death and victory/retirement rule evidence. Frontend tests fix result priority, relevant-event selection, one-time victory acknowledgement, terminal retirement, and unknown-seed recovery actions. Tauri coverage fixes same-setup session/replay replacement, while the desktop E2E continues to cover cold start, session creation, native save/load, rendering, localization, and crash diagnostics.
