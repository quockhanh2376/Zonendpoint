# Release/installer QA — 01e7449

**PASS for the tested unsigned NSIS0.1.0 current-user install scope.** No production-release or unqualified full-MVP acceptance claim. EICAR through Windows remains blocked by Microsoft Defender; MSI, different-version upgrade, multi-user/system-wide install, signing, resource benchmark and full real-network reset/cancel matrix are NOT RUN.

## Snapshot/artifact
- Product code/config:01e744974b75332592aa5621bbcfc0e12da8b477 (core previously verified8e8204a plus NSIS icon configuration).
- Vic built with `npm.cmd run tauri build -- --bundles nsis`, CARGO_TARGET_DIR=E:\Zonendpoint-tools\qeen-target, default release without native-ui-tests.
- Artifact:E:\Zonendpoint-tools\artifacts\01e7449\Zonendpoint_0.1.0_x64-setup.exe (about3.50MiB).
- SHA256:94336243A891F372B0B05D18C6EB3A180AA1C05E10CC7A54735E1428DDDD14E6.
- Authenticode:NotSigned. Test version0.1.0 intentionally retained; no GitHub release/installer upload or signing purchase.

## Roles and observed packaging fix
Vic/Qeen prompts and plan are in ../superpowers/plans/2026-11-07-release-installer-qa.md. Vic produced initial artifact; Qeen observed generic NSIS setup icon while app icon was branded. Vic set bundle.windows.nsis.installerIcon/uninstallerIcon to existing icons/icon.ico. Fresh static review confirmed schema/path correctness. Qeen tested the final rebuilt artifact, not just the initial package.

The initial strict installed/build hash equality check failed because Tauri replaces one3-byte UNK marker with NSS for NSIS. Investigation isolated all differences to that marker. The retained provenance check permits only that exact contiguous3-byte change with expected prefix and validates all other bytes. It passes for final payload. This was a test-assumption correction, not a product binary discrepancy.

## Final artifact checks
| Check | Observed result |
|---|---|
| Package build/metadata | PASS. x64 NSIS, version0.1.0; App executable PE FileDescription/ProductName Zonendpoint, CompanyName zonendpoint. Installer PE FileDescription/ProductName Zonendpoint, CompanyName blank (not a signing/publisher-trust claim). No displayed Tauri template metadata. Generated NSIS language English. |
| Installer/uninstaller icons | PASS. Extracted32px installer and uninstaller PNGs exactly match app icon PNG; before/after icons retained in release-nsis-01e7449. |
| Fresh current-user sandbox install | PASS exit0 into E:\Zonendpoint-tools\installer-sandbox\app. No pre-existing Zonendpoint install/shortcut identity was found. One uninstall registration, Start Menu and Desktop shortcuts target the exact sandbox executable. |
| Payload provenance | PASS. Installed payload matches release build except permitted NSIS bundle marker. Actual native picker appears despite deliberately set ZONENDPOINT_TEST_FIXTURE bogus path, proving it is ignored in final release. |
| Actual shortcut launch/theme | PASS. Start Menu shortcut launched installed path. Initial candidate first launch Light; switched Dark and reopened through shortcut. Final artifact retained Dark through uninstall/reinstall with private profile. Native titlebar/app branding visually correct. Associated shortcut icon uses branded executable; taskbar grouping was not separately inspected. |
| Final installed native scan | PASS. Actual Windows Folder picker chose dedicated2-file benign fixture. Fixture contents changed before final run to prevent prior content-cache reuse. Real ClamAV via endpoint127.0.0.1:13310 returned Scan complete, processed2/clean2, threats/errors/skipped/cancelled0; green/check All Good and auto-grown result window observed. Dark logos/white slogan retained. |
| Release engine offline/cache | PASS. Exact installed release CLI: online same file Clean, exit0; closed unused loopback endpoint Error, exit1, never Clean despite cached clean result. |
| Same-version repair/reinstall | PASS exit0. Removed only owned sandbox executable after target/reparse validation, reran installer, restored byte-identical packaged payload. This is not a different-version upgrade test. |
| Uninstall/cleanup | PASS exit0. Owned executable, per-user uninstall registration, Desktop and Start Menu shortcuts removed. Private user-data sentinel retained. Existing demo cache SHA256 unchanged2DE45CDEC359752C767752891E45B95E0B9BDB2ABCBCA75D63EF3576F331EA5E. No app remained installed by the test. |

Qeen used private LOCALAPPDATA and WEBVIEW2_USER_DATA_FOLDER when launching, with dedicated benign fixtures. Actual user cache hash was checked before/after. Existing user profile/cache were not reset. The installer script was inspected: app-data removal requires explicit checkbox; silent uninstall did not select it. No AV policy, exclusions or production configuration changed.

## Evidence and limits
JSON results, manifest, before/after icon PNGs and machine-specific test scripts are in release-nsis-01e7449/. Source build logs:E:\Zonendpoint-tools\release-nsis-build.log and release-nsis-final.log. Native desktop observations are reported above; no fabricated screenshot artifact is claimed. Test scripts contain this machine's sandbox paths/baseline guards and must be inspected before reuse elsewhere.

Core history:Rust20/native mock11/real lab6 at8e8204a; actual Folder/Cancel/rescan follow-up PASS. Those are prior unchanged-core results, not rerun unit-suite claims for01e7449. Final release was compiled and installed/exercised as stated above.

Microsoft Defender attribution is confirmed by Operational events1116/1117 for Virus:DOS/EICAR_Test_File on the exact earlier QA fixture, Real-Time Protection and Quarantine. The app's os error225 was expected interference, never CLEAN. Separate real ClamAV positive detection used a harmless fixture with temporary lab-only signature (removed afterward). No claim that Zonendpoint detected Windows EICAR before Defender.

PR remains draft pending owner's merge decision/acceptance of remaining scope limitations. No main merge, signed/public release or production-ready claim.

Final fresh-context static review independently verified code diff, installer SHA/version/NotSigned, evidence JSON and strict payload provenance check; no additional product finding. Report metadata wording was clarified to distinguish app CompanyName from blank setup CompanyName. GUI observations remain the executed Qeen session's evidence, not an additional reviewer run.

