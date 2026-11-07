# Notebook reconciliation — 2026-11-07

Source: [ZonEndpoint Notebook](https://notebook.google.com/notebook/dd80d8b6-75bb-4f83-ba9a-fa97c6c0e510), viewed after owner login. Read the visible latest recommendations and full Studio `SourceDatabase-v2.md` (nine source groups, five-tier lookup and ingestion checklist). This is a scoped review, not an export of all Notebook chat history or certification of its12 uploaded sources. No Notebook content, sharing or accounts were modified. Zon confirmed **internal company use first**.

Notebook-generated recommendations are inputs, not implementation authorization. Approved current scope and observed code/QA take precedence. Preserve original specifications at their original relative names; restored from source checkout because recovery branch omitted them. Architecture addendum is appended; Ed25519 spec is unmodified. Earlier architecture review remains explicitly historical. Source checkout and its uncommitted files remain untouched.

## Reconciliation map

| Notebook proposal | Current finding / disposition | Roadmap task |
| --- | --- | --- |
| Supplementary Windows user-space scanner alongside Defender | Approved position; no execution blocking or replacement claim | P0/P1 |
| Nine antivirus/threat-intel source groups | Catalog candidates; only existing ClamAV lab evidence established, no new sources enabled | FEED-LAB |
| Five-tier whitelist/blacklist/cache/engine/public-API flow | Not adopted: whitelist-first CLEAN and automatic quarantine conflict with approved guarantees | IOC-SYNC decision |
| Signed/versioned hash deltas, zstd, SQLite batch dedup | Future design; require authenticity, rollback/expiry, provenance and bounded decompression | IOC-SYNC |
| Guard, Battle Mode, IP blocking and session/permanent override | Proposed later; permissions, event source/attribution and reversible enforcement unresolved | GUARD/NETWORK/RESPONSE |
| Process DACL, DLL search restrictions, ASLR/DEP/CFG | Hardening candidates, not proof of tamper resistance | HARDENING |
| SQLCipher, DPAPI, zeroize | No new dependency; model data, identity/recovery and migration before implementation | DATA |
| BYOK AI explanation/script analysis | Optional unimplemented triage; data/secret/tool boundaries required | AI |
| 60–75% versus85–90% detection;75–125MB RAM;CPU<15%;<1ms lookup;2–5MB delta/<200ms ingest | Unsupported product claims; define corpus, hardware, measurement methodology and repeatable results first | PERF |

## Source catalog and checks before use

These are the Notebook's nine groups (URLhaus/Feodo grouped together there), not nine configured integrations. No signup, purchase, API call with private data or downloaded malware was performed.

| Candidate | Proposed placement | Required check |
| --- | --- | --- |
| Cisco Talos/ClamAV official databases | Existing server engine baseline | Audit actual freshclam/update/reload health and engine identity |
| SecuriteInfo | Optional server signatures | Company/deployment/redistribution entitlement; credentials and update schedule |
| YARA Rules / YARA Forge | Staged compatible server rules | Rule license, provenance, exact ClamAV dialect and compile/load regression |
| MalwareBazaar | Future SHA-256 IOC metadata | Auth/use terms, source age/withdrawal; no sample download |
| ThreatFox | Future typed hash/IP/domain IOC metadata | Auth/use terms, timestamps, expiry, confidence and attribution |
| URLhaus / Feodo Tracker | Future typed URL/domain/IP metadata | Source-specific eligibility/auth, normalization and expiry; no naive domain→file-hash signature conversion |
| AlienVault OTX | Future source-attributed IOC metadata | Current service/API entitlement, license, provenance and pulse quality; not verified here |
| VirusTotal | Optional separately approved enrichment | Public API business restrictions; company use is not a blanket free entitlement; no automatic lookup/upload |
| urlscan.io | Optional separately approved URL analysis | Submission visibility/data disclosure, current terms/rate/license; not verified here; no automatic URL submission |

## Corrections verified against primary references

- ClamAV accepts a restricted YARA dialect and does not load `yarac` precompiled rules. A generic YARA compile pass alone cannot certify the deployed ClamAV; stage raw compatible rules, validate against that engine, retain known-good set and test rollback. [ClamAV YARA documentation](https://docs.clamav.net/manual/Signatures/YaraRules.html).
- VirusTotal Public API documents4 requests/minute and500/day plus commercial/business-workflow restrictions. Treat eligibility as a gate even for an internal company deployment. This does not authorize submitting files to qualify for a service. [Public vs Premium](https://docs.virustotal.com/reference/public-vs-premium-api).
- URLhaus documents an Auth-Key requirement and fair-use/commercial conditions. Do not copy the Notebook's blanket “free/open API” assumption into deployment. [URLhaus Community API](https://urlhaus.abuse.ch/api).
- SecuriteInfo advertises separate plans and explicit reseller rights for embedding signatures in products/services; confirm the correct entitlement before deployment rather than assuming a personal/free account covers company use. No subscription was selected. [Vendor offering](https://www.securiteinfo.com/clamav-antivirus/improve-detection-rate-of-zero-day-malwares-for-clamav-en.shtml).
- Microsoft documents administrative/UAC requirements for `SetTcpEntry` and limits its state change to deleting a TCP control block. It is not a universal IP/UDP/IPv6 prevention mechanism. Future network enforcement needs its own design and lab; do not elevate the existing scanner to add it. [SetTcpEntry](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-settcpentry).
- Process access is governed by Windows security descriptors and privileges; changing a DACL is not equivalent to becoming a protected process. Define a threat model and supported recovery instead of claiming the app cannot be killed. [Process security](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights).

## Decisions already fixed / still open

Fixed:internal pilot first; Windowsx64; manual initiation; native Quick13 core+drivers; Guard OFF; no automatic quarantine/public submission/feed deployment; no AV exclusions; incomplete/offline never CLEAN. Observed protected `drivers\DriverData` access denial is not a reason to silently skip it, relabel the scan clean or auto-elevate.

Open before later implementation:pilot enrollment/distribution/signing policy; acceptable measured resource/cancel-latency budgets; chosen feed entitlement and false-positive process; transport/key rotation/revocation; whether SQLCipher protects enough sensitive data to justify migration/recovery complexity; AI provider/data consent. Record an approved decision when needed by its task rather than blocking current documentation or building all proposals together.

Recommended next action is V/Q-RELEASE-CURRENT, then the real-network/engine-ops/performance tasks in [roadmap](roadmap.md). Source lists and proposed schemas do not justify weakening content/engine-bound cache validation or hiding a detection behind an allow override.

## Documentation validation

Qeen performed a read-only static documentation/code comparison of4704aa8 plus this documentation change; no new runtime acceptance was claimed. Corrected README's old “single-use” picker-token description:the native token is opaque/short-lived and can be reused within300 seconds for rescan. Relative file links in the changed documentation resolve. Original architecture bytes remain an exact prefix before the new addendum, and Ed25519 SHA256 matches the source file. No application code/dependency or environment policy changed in this documentation task; product QA remains the previously recorded snapshot-specific evidence.
