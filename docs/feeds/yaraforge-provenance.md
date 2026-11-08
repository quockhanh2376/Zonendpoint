# YARA-Forge Core Feed Provenance & Verification

**Document date**: 2026-11-08  
**Author**: Victor (Vic)  
**Branch**: `vic/feed-yaraforge`  
**Scope**: Personal, non-commercial security evaluation (`cá nhân, phi thương mại`)  
**Upstream Project**: [YARAHQ/yara-forge](https://github.com/YARAHQ/yara-forge)  

---

## 1. Upstream Package Release Metadata

| Parameter | Value |
|---|---|
| **Release Tag** | `20261103` |
| **Release Date** | 2026-11-03 |
| **Package Type** | **Core** (`yara-forge-rules-core.zip`) |
| **Download URL** | `https://github.com/YARAHQ/yara-forge/releases/download/20261103/yara-forge-rules-core.zip` |
| **Archive Size** | 1,744,269 bytes (1.66 MiB) |
| **Archive SHA256** | `be0d5a49d2a8cd37713f5a8c4e5a3c4235061242bc12240e18c69934fa47bb59` |
| **Extracted File** | `packages/core/yara-rules-core.yar` |
| **Extracted File Size** | 7,844,156 bytes (7.48 MiB) |
| **Extracted File SHA256** | `4f5cca2cfce626518a0eb0d5af26749f73b79a6bf92f8630b1772dc2b5f4c5a2` |
| **Total Rules in Package** | 5,133 rules |

---

## 2. Package Quality & Selection Criteria

As declared in the YARA-Forge Core header:
- **YARA-Forge Version**: 0.9.1
- **YARA-QA Commit**: `7f1c7f4ad5b5164aa49361bf2a772795202d7e36`
- **Minimum Quality**: 70
- **Force Include Importance Level**: 80
- **Force Exclude Importance Level**: 50
- **Minimum Age**: 1 day
- **Minimum Score**: 65
- **Excluded during forge generation**: 3,250 (age), 733 (quality), 954 (score), 2,039 (importance)

---

## 3. Upstream Licensing & Usage Scope

The YARA-Forge Core package aggregates rules from 34 upstream security repositories. Licenses vary across rule authors:
- **Apache License 2.0**: GCTI, Trellix ARC, Google, etc.
- **BSD 2-Clause / 3-Clause**: FireEye/Mandiant, Volexity.
- **MIT License**: Telekom Security, Elastic.
- **GPLv3**: Cuckoo Sandbox / CAPE Foundation.
- **Custom Permissive**: ESET spol. s r.o.
- **No Explicit License Set**: R3c0nst, DeadBits, Malpedia, Arkbird SOLG, SecuInfra, RussianPanda, CadoSecurity, etc.

### Zonendpoint Usage Scope
Per product instructions, this feed is strictly ingested for **personal, non-commercial security research and staging validation** (`scope: cá nhân, phi thương mại`). It must not be distributed as a commercial feed, bundled into commercial deliverables, or exposed on public feeds.

---

## 4. ClamAV Staging Filtering Summary

The raw package was filtered using `scripts/feeds/filter_yara_clamav.py` to ensure compatibility with ClamAV 1.5.4:

| Metric | Count |
|---|---|
| **Raw Upstream Rules** | 5,133 |
| **Accepted Rules (ClamAV compatible)** | 3,765 |
| **Rejected Rules** | 1,368 |
| **ClamAV Engine Load Status** | 100% loaded (3,765/3,765) with 0 errors and 0 warnings |

Filtered staging artifact: `yara-forge-core-clamav.yar`  
Filter report: `yara-forge-filter-report.json`
