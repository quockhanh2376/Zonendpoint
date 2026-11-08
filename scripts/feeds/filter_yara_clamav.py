#!/usr/bin/env python3
"""
Filter YARA rules for ClamAV engine compatibility.

Enforces constraints:
  - No imported modules (pe, dotnet, hash, math, cuckoo, elf, etc.) or import statements
  - No global or private rules
  - No external variables
  - String count <= 64 per rule
  - Every string pattern length >= 2 bytes
  - ClamAV-specific compatibility:
      * No unsupported string modifiers (base64, base64wide, xor)
      * No empty string values in meta (e.g. tags = "")
      * No big-endian integer functions (uint16be, uint32be, int16be, int32be)
      * No inline comments inside hex patterns { ... }
      * Hex patterns must have at least one 2-byte contiguous static chunk
      * Regexes must have at least one 2-byte static subpattern
  - Authoritative ClamAV load-loop validation:
      * Runs clamscan -d on the ruleset
      * Identifies any rule causing ClamAV load warnings or errors
      * Prunes them into rejected log until ClamAV loads 100% cleanly with 0 errors
"""

import sys
import os
import re
import json
import subprocess
import argparse
from typing import List, Tuple, Dict, Any, Optional, Set

MODULES = {
    'pe', 'dotnet', 'hash', 'math', 'cuckoo', 'magic', 'elf', 'time', 'console'
}

UNSUPPORTED_MODIFIERS = re.compile(r'\b(base64|base64wide|xor)\b')
EMPTY_META_PATTERN = re.compile(r'^\s*[a-zA-Z0-9_]+\s*=\s*""\s*$', re.MULTILINE)
UNSUPPORTED_FUNCS = re.compile(r'\b(uint16be|uint32be|int16be|int32be|uint8be|int8be)\b')

YARA_KEYWORDS = {
    'all', 'any', 'of', 'them', 'at', 'in', 'filesize', 'entrypoint',
    'int8', 'int16', 'int32', 'uint8', 'uint16', 'uint32',
    'and', 'or', 'not', 'true', 'false', 'matches', 'contains', 'icontains',
    'startswith', 'istartswith', 'endswith', 'iendswith', 'iequals', 'defined',
    'for', 'each', 'to'
}


def parse_yara_file(content: str) -> List[Dict[str, Any]]:
    """Parse YARA file into individual rule records using brace matching."""
    rules = []
    n = len(content)
    
    rule_start_re = re.compile(
        r'(?:^|\n)\s*((?:(?:global|private)\s+)?rule\s+([A-Za-z0-9_]+)(?:\s*:[^{]*)?)\s*\{'
    )
    
    for m in rule_start_re.finditer(content):
        start_brace = m.end() - 1
        rule_hdr = m.group(1).strip()
        rule_name = m.group(2).strip()
        
        brace_depth = 1
        i = start_brace + 1
        in_string = False
        in_hex = False
        in_comment = False
        in_line_comment = False
        
        while i < n and brace_depth > 0:
            c = content[i]
            if in_line_comment:
                if c == '\n':
                    in_line_comment = False
            elif in_comment:
                if c == '*' and i + 1 < n and content[i + 1] == '/':
                    in_comment = False
                    i += 1
            elif in_string:
                if c == '\\':
                    i += 1
                elif c == '"':
                    in_string = False
            elif in_hex:
                if c == '}':
                    in_hex = False
            else:
                if c == '/' and i + 1 < n and content[i + 1] == '/':
                    in_line_comment = True
                    i += 1
                elif c == '/' and i + 1 < n and content[i + 1] == '*':
                    in_comment = True
                    i += 1
                elif c == '"':
                    in_string = True
                elif c == '{':
                    in_hex = True
                elif c == '}':
                    brace_depth -= 1
            i += 1
            
        full_text = content[m.start():i].strip()
        body = content[start_brace + 1:i - 1]
        
        rules.append({
            'name': rule_name,
            'header': rule_hdr,
            'body': body,
            'full_text': full_text
        })
        
    return rules


def evaluate_string_validity(var_name: str, val: str) -> Tuple[bool, Optional[str]]:
    val = val.strip()
    
    if UNSUPPORTED_MODIFIERS.search(val):
        return False, f"unsupported_string_modifier in '{var_name}'"
        
    # 1. Text string: "..."
    if val.startswith('"'):
        m = re.match(r'^"((?:\\.|[^"\\])*)"', val)
        if not m:
            return False, f"malformed text string in '{var_name}'"
        text_val = m.group(1)
        if len(text_val) == 0:
            return False, f"empty string in '{var_name}'"
        try:
            raw_bytes = text_val.encode('utf-8').decode('unicode_escape').encode('latin1')
            byte_len = len(raw_bytes)
        except Exception:
            byte_len = len(re.sub(r'\\.', 'X', text_val))
            
        if byte_len < 2:
            return False, f"string '{var_name}' text length {byte_len} < 2 bytes"
            
    # 2. Hex string: { ... }
    elif val.startswith('{'):
        m = re.match(r'^\{([^}]+)\}', val)
        if not m:
            return False, f"malformed hex string in '{var_name}'"
        hex_body = m.group(1).strip()
        
        # Check for comments inside hex pattern
        if '//' in hex_body or '/*' in hex_body:
            return False, f"comment inside hex pattern in '{var_name}'"
            
        # Strip jumps [x-y] and wildcards
        cleaned_hex = re.sub(r'\[\s*\d+\s*(?:-\s*\d+)?\s*\]', ' ', hex_body)
        cleaned_hex = re.sub(r'\([^\)]+\)', ' ', cleaned_hex)
        chunks = re.split(r'\?+', cleaned_hex)
        max_contiguous_bytes = 0
        for ch in chunks:
            hex_bytes = re.findall(r'[0-9a-fA-F]{2}', ch)
            if len(hex_bytes) > max_contiguous_bytes:
                max_contiguous_bytes = len(hex_bytes)
                
        if max_contiguous_bytes < 2:
            return False, f"hex pattern '{var_name}' lacks contiguous 2-byte static subpattern (max: {max_contiguous_bytes})"
            
    # 3. Regex string: /.../
    elif val.startswith('/'):
        m = re.match(r'^/((?:\\.|[^/\\])*)/', val)
        if not m:
            return False, f"malformed regex in '{var_name}'"
        regex_body = m.group(1)
        clean = re.sub(r'\\.', 'X', regex_body)
        if len(clean) < 2:
            return False, f"regex '{var_name}' length < 2 bytes"
            
        static_chunks = re.split(r'[\.\*\+\?\{\}\[\]\(\)\|\^\$\\]', regex_body)
        max_chunk = max([len(c) for c in static_chunks if c], default=0)
        if max_chunk < 2:
            return False, f"regex '{var_name}' lacks static subpattern of length 2"
            
    return True, None


def inspect_rule(rule: Dict[str, Any]) -> Tuple[bool, str, Dict[str, Any]]:
    """Inspect rule against ClamAV constraints."""
    hdr = rule['header']
    body = rule['body']
    name = rule['name']
    
    # 0. Heuristic SUSP_ rules
    if 'SUSP_' in name:
        return False, "heuristic_susp_rule", {}
        
    # 1. Global / Private
    if hdr.startswith('global ') or hdr.startswith('private '):
        return False, "global_or_private_rule", {}
        
    # 2. Meta block: no empty string value (e.g. tags = "")
    meta_m = re.search(r'\bmeta\s*:\s*(.*?)(?=\b(?:strings|condition)\s*:|\Z)', body, re.DOTALL)
    if meta_m and EMPTY_META_PATTERN.search(meta_m.group(1)):
        return False, "empty_meta_value", {}
        
    # 3. Condition block existence
    cond_match = re.search(r'\bcondition\s*:\s*(.*?)\Z', body, re.DOTALL)
    if not cond_match:
        return False, "missing_condition_block", {}
    condition_str = cond_match.group(1).strip()
    
    # 4. Imported modules in condition
    for mod in MODULES:
        if re.search(r'\b' + mod + r'\.', condition_str):
            return False, f"uses_imported_module_{mod}", {}
            
    # 5. Unsupported endian functions
    if UNSUPPORTED_FUNCS.search(condition_str):
        return False, "unsupported_endian_function_in_condition", {}
        
    # 6. Extract strings block
    strings_match = re.search(r'\bstrings\s*:\s*(.*?)(?=\bcondition\s*:|\Z)', body, re.DOTALL)
    str_defs = []
    if strings_match:
        strings_text = strings_match.group(1)
        raw_strs = re.findall(
            r'(\$[a-zA-Z0-9_]*)\s*=\s*(.*?)(?=(?:\n\s*\$[a-zA-Z0-9_]*\s*=|\Z))',
            strings_text,
            re.DOTALL
        )
        str_defs = [(v.strip(), val.strip()) for v, val in raw_strs]
        
    # 7. String count <= 64
    if len(str_defs) > 64:
        return False, f"string_count_{len(str_defs)}_exceeds_64", {'string_count': len(str_defs)}
        
    # 8. Evaluate each string pattern
    for var_name, var_val in str_defs:
        valid, reason = evaluate_string_validity(var_name, var_val)
        if not valid:
            return False, reason or "invalid_string_pattern", {'string_count': len(str_defs)}
            
    # 9. External variables check in condition
    cond_no_strings = re.sub(r'[\$#@!][a-zA-Z0-9_*]*', '', condition_str)
    cond_no_nums = re.sub(r'\b0x[0-9a-fA-F]+\b|\b\d+\b', '', cond_no_strings)
    remaining_tokens = re.findall(r'\b([a-zA-Z_][a-zA-Z0-9_]*)\b', cond_no_nums)
    ext_vars = []
    for t in remaining_tokens:
        t_low = t.lower()
        if t_low not in YARA_KEYWORDS and t not in ('KB', 'MB', 'GB'):
            ext_vars.append(t)
            
    if ext_vars:
        return False, f"external_var_or_rule_ref: {ext_vars[0]}", {'ext_vars': ext_vars}
        
    return True, "accepted", {'string_count': len(str_defs)}


def write_yara_file(rules: List[Dict[str, Any]], output_path: str):
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write("/*\n")
        f.write(" * Zonendpoint Filtered YARA Ruleset (ClamAV-Compatible)\n")
        f.write(f" * Total rules: {len(rules)}\n")
        f.write(" * Generated by scripts/feeds/filter_yara_clamav.py\n")
        f.write(" * Constraints: no imports, no global/private, no ext vars, <=64 strings, string >=2 bytes\n")
        f.write(" */\n\n")
        for r in rules:
            f.write(r['full_text'])
            f.write("\n\n")


def run_clamscan_check(yar_path: str) -> Tuple[int, int, List[str]]:
    """Run clamscan and return (loaded_count, failed_count, error_lines)."""
    cmd = ['clamscan', '-d', yar_path, '/etc/hosts']
    res = subprocess.run(cmd, capture_output=True, text=True)
    
    loaded = 0
    failed = 0
    err_lines = []
    
    for line in res.stderr.splitlines():
        if 'LibClamAV' in line:
            err_lines.append(line)
        m = re.search(r'failed to parse or load (\d+) yara rules.*successfully loaded (\d+) rules', line)
        if m:
            failed = int(m.group(1))
            loaded = int(m.group(2))
            
    # Also check known viruses from summary
    m_known = re.search(r'Known viruses:\s*(\d+)', res.stdout)
    if m_known and loaded == 0:
        loaded = int(m_known.group(1))
        
    return loaded, failed, err_lines


def identify_failed_rules(err_lines: List[str], current_rules: List[Dict[str, Any]], yar_path: str) -> Set[str]:
    """Identify which rules caused ClamAV errors/warnings."""
    failed_names = set()
    
    # 1. Check for explicit rule name in warning/error
    for line in err_lines:
        m = re.search(r'yara rule ([A-Za-z0-9_]+)', line)
        if m:
            failed_names.add(m.group(1))
        m2 = re.search(r'skipping YARA\.([A-Za-z0-9_]+)', line)
        if m2:
            failed_names.add(m2.group(1))
            
    # 2. Check for line number errors (yyerror)
    line_nums = []
    for line in err_lines:
        m = re.search(r'yyerror\(\): .*? line (\d+)', line)
        if m:
            line_nums.append(int(m.group(1)))
            
    if line_nums:
        with open(yar_path, 'r', encoding='utf-8', errors='ignore') as f:
            all_lines = f.readlines()
            
        for lno in line_nums:
            # Look backwards from lno to find rule name
            for idx in range(min(lno - 1, len(all_lines) - 1), max(-1, lno - 100), -1):
                rm = re.search(r'\brule\s+([A-Za-z0-9_]+)', all_lines[idx])
                if rm:
                    failed_names.add(rm.group(1))
                    break
                    
    return failed_names


def filter_and_validate(input_path: str, output_yar_path: str, report_json_path: str, run_validation: bool = False):
    print(f"Reading: {input_path}")
    with open(input_path, 'r', encoding='utf-8', errors='ignore') as f:
        content = f.read()
        
    rules = parse_yara_file(content)
    print(f"Total rules discovered in source: {len(rules):,}")
    
    seen_names: Set[str] = set()
    accepted = []
    rejected = []
    stats_reasons: Dict[str, int] = {}
    
    for r in rules:
        name = r['name']
        if name in seen_names:
            reason = "duplicate_rule_name"
            rejected.append({'name': name, 'reason': reason})
            stats_reasons[reason] = stats_reasons.get(reason, 0) + 1
            continue
            
        ok, reason, meta = inspect_rule(r)
        if ok:
            seen_names.add(name)
            accepted.append(r)
        else:
            cat = reason.split(':')[0].strip()
            rejected.append({'name': name, 'reason': reason, 'metadata': meta})
            stats_reasons[cat] = stats_reasons.get(cat, 0) + 1
            
    print(f"\nStatic filter results:")
    print(f"  Accepted: {len(accepted):,}")
    print(f"  Rejected: {len(rejected):,}")
    for cat, count in sorted(stats_reasons.items(), key=lambda x: -x[1]):
        print(f"    - {cat}: {count}")
        
    write_yara_file(accepted, output_yar_path)
    print(f"Wrote {len(accepted)} initial rules to: {output_yar_path}")
    
    if run_validation:
        print("\n=== ClamAV Dynamic Load Validation Loop ===")
        iteration = 1
        max_iterations = 10
        
        while iteration <= max_iterations:
            loaded, failed, err_lines = run_clamscan_check(output_yar_path)
            print(f"Iteration {iteration}: ClamAV loaded={loaded}, failed={failed}, error_lines={len(err_lines)}")
            
            if failed == 0 and len(err_lines) == 0:
                print(f">> SUCCESS: ClamAV loaded all {loaded} rules with ZERO errors and ZERO warnings!")
                break
                
            problem_names = identify_failed_rules(err_lines, accepted, output_yar_path)
            if not problem_names:
                print(">> Warning: could not isolate specific rule names from ClamAV errors.")
                break
                
            print(f"Pruning {len(problem_names)} rules identified by ClamAV load failures...")
            still_accepted = []
            for r in accepted:
                if r['name'] in problem_names:
                    reason = "clamav_engine_load_error"
                    rejected.append({'name': r['name'], 'reason': reason})
                    stats_reasons[reason] = stats_reasons.get(reason, 0) + 1
                else:
                    still_accepted.append(r)
                    
            accepted = still_accepted
            write_yara_file(accepted, output_yar_path)
            iteration += 1
            
    # Final write of report
    report = {
        'source_file': input_path,
        'total_parsed': len(rules),
        'accepted_count': len(accepted),
        'rejected_count': len(rejected),
        'rejection_breakdown': stats_reasons,
        'accepted_rule_names': [r['name'] for r in accepted],
        'rejected_rules': rejected
    }
    os.makedirs(os.path.dirname(os.path.abspath(report_json_path)), exist_ok=True)
    with open(report_json_path, 'w', encoding='utf-8') as f:
        json.dump(report, f, indent=2)
    print(f"\nWrote final rejection report to: {report_json_path}")
    print(f"Final accepted rules count: {len(accepted)}")


def main():
    parser = argparse.ArgumentParser(description="Filter YARA rules for ClamAV engine")
    parser.add_argument("-i", "--input", required=True, help="Input YARA file")
    parser.add_argument("-o", "--output", required=True, help="Output filtered YARA file")
    parser.add_argument("-r", "--report", required=True, help="Output report JSON")
    parser.add_argument("--validate", action="store_true", help="Run ClamAV validation loop")
    args = parser.parse_args()
    
    filter_and_validate(args.input, args.output, args.report, run_validation=args.validate)


if __name__ == '__main__':
    main()
