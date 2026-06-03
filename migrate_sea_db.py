#!/usr/bin/env python3
"""
Step 12/13 sea_db & db_path 迁移:
  &state.sea_db       → state.harness.db()
  &app_state.sea_db   → app_state.harness.db()
  state.sea_db.clone() → state.harness.db().clone()
  app_state.sea_db.clone() → app_state.harness.db().clone()
  &state.db_path      → state.harness.db_path()
  &app_state.db_path  → app_state.harness.db_path()
"""

import sys
import re
from pathlib import Path

RULES = [
    # sea_db
    (re.compile(r'\bstate\.sea_db\.clone\(\)'),  'state.harness.db().clone()'),
    (re.compile(r'\bapp_state\.sea_db\.clone\(\)'), 'app_state.harness.db().clone()'),
    (re.compile(r'&state\.sea_db'),  'state.harness.db()'),
    (re.compile(r'&app_state\.sea_db'), 'app_state.harness.db()'),
    # db_path
    (re.compile(r'&state\.db_path'),  'state.harness.db_path()'),
    (re.compile(r'&app_state\.db_path'), 'app_state.harness.db_path()'),
]


def transform(text):
    total = 0
    for pat, repl in RULES:
        text, n = pat.subn(repl, text)
        total += n
    return text, total


def process_file(path):
    raw = path.read_bytes()
    if raw.startswith(b'\xef\xbb\xbf'):
        bom = b'\xef\xbb\xbf'
        raw = raw[3:]
    else:
        bom = b''
    text = raw.decode('utf-8', errors='replace')
    new, n = transform(text)
    if n == 0:
        return 0
    out = bom + new.encode('utf-8', errors='replace')
    path.write_bytes(out)
    return n


def main():
    paths = sys.argv[1:]
    files = []
    for p in paths:
        path = Path(p)
        if path.is_dir():
            files.extend(path.rglob('*.rs'))
        else:
            files.append(path)

    total_n = 0
    files_n = 0
    for f in files:
        if '.bak' in f.name or '.tmp' in f.name or 'target' in f.parts:
            continue
        n = process_file(f)
        if n:
            files_n += 1
            total_n += n
            print(f'{f}: {n}')
    print(f'=== {files_n} files, {total_n} replacements ===')


if __name__ == '__main__':
    main()
