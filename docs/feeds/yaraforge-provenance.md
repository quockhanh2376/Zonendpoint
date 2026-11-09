# YARA-Forge Core Feed Provenance & Verification

**Document date**: 2026-11-08  
**Author**: Victor (Vic)  
**Branch**: `vic/yara-live`  
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

## 4. ClamAV Staging Filtering Summary (v2 — V-15 Update)

The raw package was filtered using `scripts/feeds/filter_yara_clamav.py` to ensure compatibility with ClamAV 1.5.4. In task V-15, all heuristic `SUSP_` rules were eliminated after QA Task Q-12 identified a False Positive on `Windows PowerShell.evtx`:

| Metric | Count |
|---|---|
| **Raw Upstream Rules** | 5,133 |
| **Accepted Rules (ClamAV compatible, V-15)** | **3,678** (3,765 - 87 = 3,678) |
| **Rejected Rules** | **1,455** (1,368 + 87 = 1,455) |
| **ClamAV Engine Load Status** | 100% loaded (3,678/3,678) with 0 errors and 0 warnings |

### Rejection Categories Breakdown
- `empty_meta_value`: 557 rules
- `unsupported_endian_function_in_condition`: 367 rules
- `clamav_engine_load_error`: 163 rules
- `heuristic_susp_rule`: **132 rules** (all rules containing `SUSP_`)
- `uses_imported_module_pe`: 121 rules
- `uses_imported_module_hash`: 34 rules
- `uses_imported_module_dotnet`: 8 rules
- `external_var_or_rule_ref`: 8 rules
- `global_or_private_rule`: 5 rules
- `missing_condition_block`: 4 rules
- `unsupported_string_modifier` / syntax issues: 56 rules

Filtered artifact: `yara-forge-core-clamav.yar` (5,828,643 bytes, SHA256: `f341d83a2ae0dae55f17ace41d2ed59d951fd2b79543b0dac130f81cec960ce6`)  
Filter report: `yara-forge-filter-report.json`

---

## 5. Danh sách các quy tắc `SUSP_` bị loại trừ (132 rules)

Toàn bộ các quy tắc mang tiền tố hoặc chứa chuỗi `SUSP_` bị loại bỏ nhằm triệt tiêu nguy cơ nhận diện nhầm trên dữ liệu nhật ký, tệp script quản trị hệ thống:

```
COD3NYM_SUSP_Direct_Syscall_Shellcode_Invocation_Jan24
COD3NYM_SUSP_NET_Shellcode_Loader_Indicators_Jan24
COD3NYM_SUSP_OBF_NET_Reactor_Native_Stub_Jan24
DELIVRTO_SUSP_HTML_WASM_Smuggling
DELIVRTO_SUSP_PDF_MHT_Activemime_Sept23
SECUINFRA_SUSP_Discord_Attachments_URL
SECUINFRA_SUSP_DOTNET_PE_Download_To_Specialfolder
SECUINFRA_SUSP_DOTNET_PE_List_AV
SECUINFRA_SUSP_LNK_Staging_Directory
SECUINFRA_SUSP_Netsh_Firewall_Command
SECUINFRA_SUSP_Powershell_Download_Temp_Rundll
SECUINFRA_SUSP_Powershell_Download_Temp_Rundll_1
SECUINFRA_SUSP_Reverse_Run_Key
SIGNATURE_BASE_APT_SUSP_NK_3CX_Malicious_Samples_Mar23_1
SIGNATURE_BASE_APT_SUSP_NK_3CX_RC4_Key_Mar23_1
SIGNATURE_BASE_EXPL_SUSP_JS_POC_Dec25
SIGNATURE_BASE_EXPL_SUSP_JS_POC_RSC_Detector_Payloads_Dec25
SIGNATURE_BASE_EXPL_SUSP_Outlook_CVE_2023_23397_Exfil_IP_Mar23
SIGNATURE_BASE_EXT_SUSP_OBFUSC_Macos_Roothelper_Obfuscated
SIGNATURE_BASE_LOG_EXPL_SUSP_Teamcity_CVE_2023_42793_Oct23_1
SIGNATURE_BASE_LOG_EXPL_SUSP_Teamcity_Oct23_1
SIGNATURE_BASE_MAL_SUSP_RANSOM_Lazy_Ransomnote_Feb24
SIGNATURE_BASE_MAL_SUSP_RANSOM_Lockbit_Ransomnote_Feb24
SIGNATURE_BASE_MSIL_SUSP_OBFUSC_Xorstringsnet
SIGNATURE_BASE_SUSP_3CX_App_Signed_Binary_Mar23_1
SIGNATURE_BASE_SUSP_Anydesk_Compromised_Certificate_Jan24_1
SIGNATURE_BASE_SUSP_Anydesk_Compromised_Certificate_Jan24_2
SIGNATURE_BASE_SUSP_Anydesk_Compromised_Certificate_Jan24_3
SIGNATURE_BASE_SUSP_Appdomaininjection_Keyword_May26
SIGNATURE_BASE_SUSP_APT_MAL_NK_3CX_Malicious_Samples_Mar23_1
SIGNATURE_BASE_SUSP_APT_MAL_VEILEDSIGNAL_Backdoor_Apr23
SIGNATURE_BASE_SUSP_Base64_Encoded_Exploit_Indicators_Dec21
SIGNATURE_BASE_SUSP_Base64_Encoded_Hacktool_Dev
SIGNATURE_BASE_SUSP_BAT_Aux_Jan20_1
SIGNATURE_BASE_SUSP_BAT_OBFUSC_Jul24_1
SIGNATURE_BASE_SUSP_BAT_OBFUSC_Jul24_2
SIGNATURE_BASE_SUSP_BAT_OBFUSC_Jul24_3
SIGNATURE_BASE_SUSP_Claude_Redacted_Thinking_Magic_String_Jan26_1
SIGNATURE_BASE_SUSP_Claude_Redacted_Thinking_Magic_String_Jan26_2
SIGNATURE_BASE_SUSP_Claude_Refusal_Magic_String_Jan26
SIGNATURE_BASE_SUSP_Command_Line_Combos_Feb24_2
SIGNATURE_BASE_SUSP_Disable_ETW_Jun20_1
SIGNATURE_BASE_SUSP_DOC_RTF_Externalresource_EMAIL_Jun22
SIGNATURE_BASE_SUSP_Doc_RTF_Externalresource_May22
SIGNATURE_BASE_SUSP_Doc_RTF_Ole2Link_EMAIL_Jun22
SIGNATURE_BASE_SUSP_Doc_RTF_Ole2Link_Jun22
SIGNATURE_BASE_SUSP_Doc_Windowsinstaller_Call_Feb22_1
SIGNATURE_BASE_SUSP_Doc_Wordxmlrels_May22
SIGNATURE_BASE_SUSP_Email_Redirection_Spoofing_Feb25
SIGNATURE_BASE_SUSP_Email_Suspicious_Onenote_Attachment_Jan23_1
SIGNATURE_BASE_SUSP_Email_Suspicious_Onenote_Attachment_Jan23_2
SIGNATURE_BASE_SUSP_Encoded_Discord_Attachment_Oct21_1
SIGNATURE_BASE_SUSP_ENV_Folder_Root_File_Jan23_1
SIGNATURE_BASE_SUSP_Esxiargs_Endpoint_Conf_Aug23
SIGNATURE_BASE_SUSP_EXPL_Cleo_Exploitation_Log_Indicators_Dec24_1
SIGNATURE_BASE_SUSP_EXPL_Cleo_Exploitation_Log_Indicators_Dec24_2
SIGNATURE_BASE_SUSP_EXPL_Cleo_Exploitation_XML_Indicators_Dec24_1
SIGNATURE_BASE_SUSP_EXPL_Cleo_Exploitation_XML_Indicators_Dec24_2
SIGNATURE_BASE_SUSP_EXPL_Commvault_CVE_2025_57791_Aug25_2
SIGNATURE_BASE_SUSP_EXPL_JAR_Indicators_Dec24
SIGNATURE_BASE_SUSP_EXPL_LIBCUE_CVE_2023_43641_Oct23_1
SIGNATURE_BASE_SUSP_EXPL_LNX_CUPS_CVE_2024_47177_Sep24
SIGNATURE_BASE_SUSP_Fake_AMSI_DLL_Jun23_1
SIGNATURE_BASE_SUSP_Fscan_Port_Scanner_Output_Jun23
SIGNATURE_BASE_SUSP_Gobfuscate_May21
SIGNATURE_BASE_SUSP_Hxd_Icon_Anomaly_May23_1
SIGNATURE_BASE_SUSP_IIS_Config_Proxyshell_Artifacts
SIGNATURE_BASE_SUSP_Imphash_Mar23_2
SIGNATURE_BASE_SUSP_JAVA_Loader_Indicators_Aug25
SIGNATURE_BASE_SUSP_Jdniexploit_Error_Indicators_Dec21_1
SIGNATURE_BASE_SUSP_Jdniexploit_Indicators_Dec21
SIGNATURE_BASE_SUSP_JS_Dropper_Mar26
SIGNATURE_BASE_SUSP_JS_NPM_Setupscript_Nov25
SIGNATURE_BASE_SUSP_JS_NPM_Sha1_Hulud_Nov25
SIGNATURE_BASE_SUSP_Katz_Log_May25
SIGNATURE_BASE_SUSP_LNK_Embedded_Worddoc
SIGNATURE_BASE_SUSP_LNK_Follina_Jun22
SIGNATURE_BASE_SUSP_LNK_Smallscreensize
SIGNATURE_BASE_SUSP_LNK_Suspicious_Folders_Jan25
SIGNATURE_BASE_SUSP_LNX_ARCH_ALPM_Hook_Jun26
SIGNATURE_BASE_SUSP_LNX_ARCH_Install_Hook_Jun26
SIGNATURE_BASE_SUSP_LNX_Base64_Download_Exec_Apr24
SIGNATURE_BASE_SUSP_LNX_Base64_Exec_Apr24
SIGNATURE_BASE_SUSP_LNX_Byteencoder_Jan25
SIGNATURE_BASE_SUSP_LNX_Linux_Malware_Indicators_Aug20_1
SIGNATURE_BASE_SUSP_LNX_SH_Cryptominer_Indicators_Dec20_1
SIGNATURE_BASE_SUSP_LNX_SH_Disk_Wiper_Script_Jun25
SIGNATURE_BASE_SUSP_LNX_Sindoor_Desktopfile_Aug25
SIGNATURE_BASE_SUSP_LNX_Sindoor_ELF_Obfuscation_Aug25
SIGNATURE_BASE_SUSP_LNX_Stackstring_Technique_Jan25
SIGNATURE_BASE_SUSP_Macro_Sheet_Obfuscated_Char
SIGNATURE_BASE_SUSP_Maldoc_Excelmacro
SIGNATURE_BASE_SUSP_MAL_EXFIL_Stealer_Output_Characteristics_Sep22_1
SIGNATURE_BASE_SUSP_MAL_Signingcert_Feb24_1
SIGNATURE_BASE_SUSP_Msdt_Artefact_Jun22_2
SIGNATURE_BASE_SUSP_NET_Msil_Suspicious_Use_Strreverse
SIGNATURE_BASE_SUSP_Nimbus_Manticore_PDF_Indicators_May26
SIGNATURE_BASE_SUSP_NK_MAL_M_Hunting_POOLRAT
SIGNATURE_BASE_SUSP_Nullsoftinst_Combo_Oct20_1
SIGNATURE_BASE_SUSP_NVIDIA_LAPSUS_Leak_Compromised_Cert_Mar22_1
SIGNATURE_BASE_SUSP_OBFUSC_Indiators_XML_Officedoc_Sep21_1
SIGNATURE_BASE_SUSP_OBFUSC_Indiators_XML_Officedoc_Sep21_2
SIGNATURE_BASE_SUSP_OBFUSC_JS_Sept21_2
SIGNATURE_BASE_SUSP_OBFUSC_Powershell_True_Jun20_1
SIGNATURE_BASE_SUSP_Onenote_Embedded_Filedatastoreobject_Type_Jan23_1
SIGNATURE_BASE_SUSP_Onenote_Embedded_Filedatastoreobject_Type_Jan23_2
SIGNATURE_BASE_SUSP_PE_Contains_Encrypted_Executable_May26
SIGNATURE_BASE_SUSP_PE_Discord_Attachment_Oct21_1
SIGNATURE_BASE_SUSP_PS1_Combo_Transfersh_Feb24
SIGNATURE_BASE_SUSP_PS1_Frombase64String_Content_Indicator
SIGNATURE_BASE_SUSP_PS1_JAB_Pattern_Jun22_1
SIGNATURE_BASE_SUSP_PS1_Msdt_Execution_May22
SIGNATURE_BASE_SUSP_PY_Import_Statement_Apr24_1
SIGNATURE_BASE_SUSP_PY_Pyinstaller_Swiper_Jun25
SIGNATURE_BASE_SUSP_PY_Shell_Spawn_Jun23_1
SIGNATURE_BASE_SUSP_RAR_NTFS_ADS
SIGNATURE_BASE_SUSP_RDP_File_Indicators_Oct24_1
SIGNATURE_BASE_SUSP_Reversed_Base64_Encoded_EXE
SIGNATURE_BASE_SUSP_Reversed_Hacktool_Author
SIGNATURE_BASE_SUSP_Scheduledtasks_Nimbus_Manticore_Persistence_May26
SIGNATURE_BASE_SUSP_Screenconnect_Exploitation_Artefacts_Feb24
SIGNATURE_BASE_SUSP_Screenconnect_User_Poc_Com_Unused_Feb24
SIGNATURE_BASE_SUSP_Screenconnect_User_Poc_Com_Used_Feb24
SIGNATURE_BASE_SUSP_Script_Base64_Blocks_Jun20_1
SIGNATURE_BASE_SUSP_Sysinternals_Desktops_Anomaly_Feb25
SIGNATURE_BASE_SUSP_TH_APT_UNC4736_Tradingtech_Cert_Apr23_1
SIGNATURE_BASE_SUSP_Themebleed_Theme_Sep23
SIGNATURE_BASE_SUSP_THOR_Unsigned_Oct23_1
SIGNATURE_BASE_SUSP_VEST_Encryption_Core_Accumulator_Jan21
SIGNATURE_BASE_SUSP_Vulndriver_HP_Hardware_Diagnostics_Etdsupp_May23
SIGNATURE_BASE_SUSP_ZIP_ISO_Phishattachment_Pattern_Jun22_1
SIGNATURE_BASE_SUSP_ZIP_LNK_Phishattachment_Pattern_Jun22_1
```
