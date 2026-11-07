from pathlib import Path
import sys
installed=Path(sys.argv[1]).read_bytes()
built=Path(sys.argv[2]).read_bytes()
if installed!=built:
    differences=[i for i,(a,b) in enumerate(zip(installed,built)) if a!=b]
    assert len(installed)==len(built) and len(differences)==3,differences[:20]
    offset=differences[0]
    assert differences==list(range(offset,offset+3))
    assert installed[offset:offset+3]==b'NSS' and built[offset:offset+3]==b'UNK'
    assert installed[:offset].endswith(b'__TAURI_BUNDLE_TYPE_VAR_')
    assert installed[:offset]+b'UNK'+installed[offset+3:]==built
print('PAYLOAD PASS: identical except permitted Tauri NSIS bundle marker')
