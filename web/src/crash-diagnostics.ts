// SPDX-License-Identifier: MPL-2.0

import { invoke } from "@tauri-apps/api/core";

export type CrashReason = "unclean-exit" | "rust-panic" | "frontend-error";

export interface CrashDiagnosticStatus {
  reportCreated: boolean;
  reportFileName: string | null;
  reason: CrashReason | null;
}

export class DesktopCrashDiagnostics {
  status(): Promise<CrashDiagnosticStatus> {
    return invoke<CrashDiagnosticStatus>("crash_diagnostic_status");
  }

  updateContext(contentId: string, contentHash: string, rendererBackend: string): Promise<void> {
    return invoke<void>("update_crash_diagnostic_context", {
      contentId,
      contentHash,
      rendererBackend,
    });
  }

  recordFrontendCrash(kind: "window-error" | "unhandled-rejection"): Promise<CrashDiagnosticStatus> {
    return invoke<CrashDiagnosticStatus>("record_frontend_crash", { kind });
  }
}
