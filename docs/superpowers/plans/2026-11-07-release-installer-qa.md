# Vic -> Qeen release/installer plan

Scope: build current MVP as unsigned Windows NSIS test package; keep current0.1.0 version and existing stack. No signing/publishing, no AV policy changes, no main merge.

Vic prompt: inspect clean branch/status/scope and packaging config. Build default release without native-ui-tests. Fix only observed packaging failures; retain source assets and user changes. Produce installer SHA256, exact code SHA, toolchain and command evidence. Do not call test-hook build a release artifact.

Qeen prompt: use exact built artifact in isolated app-data/fixture directories. Inspect installed program/shortcut/icon, native launch/theme persistence, real file/folder selection, clean/offline outcomes where possible, update/relaunch and uninstall. Preserve existing installations and user data. Test own per-user install only; no system policies or security exclusions. Record observed PASS/FAIL/BLOCKED/NOT RUN. No production-ready or signed-release claim.

Plan steps:
1. Baseline/config check and release NSIS build; record version, hash and output. No new product features planned.
2. If build fails, reproduce and fix the concrete root cause with relevant regression/validation; rebuild.
3. Inspect installer, detect existing installation before executing; choose isolated per-user install path and test desktop session. Verify actual launch and paths.
4. Exercise benign native picker/engine flow with isolated fixtures; close/reopen, inspect shortcut/icon. Uninstall only the owned test install, confirm cleanup and existing user data preserved.
5. Save concise handoff/report, review any code changes, push authorized PR branch and update draft status. Keep EICAR AV interference explicit. Never merge without explicit merge instruction.

Rulings/observations: NSIS unsigned0.1.0 is test-only; no signing/public release. Initial install/reinstall/uninstall and native shortcut/theme/real scan passed. Qeen found generic NSIS setup icon; Vic set installerIcon/uninstallerIcon to existing icons/icon.ico and final rebuild and exact-artifact retest PASS (01e7449). Payload provenance comparison permits only Tauri's known UNK->NSS3-byte bundle marker; other byte differences fail. User cache hash preserved.

Closeout: all five steps completed for scoped unsigned NSIS current-user packaging. See docs/qa/release-installer-01e7449.md; no main merge or public release.
