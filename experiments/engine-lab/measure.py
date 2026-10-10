#!/usr/bin/env python3
"""Serial, offline build measurements. Never deletes existing build directories."""
import argparse
import datetime
import json
from pathlib import Path
import platform
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('--label', default=datetime.datetime.now().strftime('%Y%m%d-%H%M%S'))
parser.add_argument('--repeats', type=int, default=3)
parser.add_argument('--timeout', type=int, default=900)
args = parser.parse_args()
if args.repeats < 1 or Path(args.label).name != args.label or args.label in ('.', '..'):
    parser.error('use a simple label and at least one repetition')
out = ROOT / 'results' / args.label
out.mkdir(parents=True, exist_ok=False)
report = {'platform': platform.platform(), 'compiler': subprocess.check_output(['rustc', '-Vv'], text=True),
          'profile': 'dev: app opt=1, dependencies opt=2, dependency debug assertions off',
          'scope': 'static mesh; no simulation, animation, UI, or runtime performance comparison', 'runs': []}

def run(package, kind, target):
    command = ['cargo', 'build', '--offline', '--locked', '--manifest-path', str(ROOT / 'Cargo.toml'),
               '-p', package, '--target-dir', str(target)]
    start = time.perf_counter()
    with (out / f'{package}-{kind}.log').open('w') as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            returncode = process.wait(timeout=args.timeout)
        except subprocess.TimeoutExpired:
            import os
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()
            returncode = 124
    entry = {'package': package, 'kind': kind, 'seconds': time.perf_counter() - start,
             'exit_code': returncode, 'command': command}
    report['runs'].append(entry)
    (out / 'builds.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(entry), flush=True)
    if returncode:
        raise SystemExit(f'Build failed; see {out}')

for package, source in [('genesis-lab-custom', 'custom/src/main.rs'), ('genesis-lab-bevy', 'bevy-view/src/main.rs')]:
    target = ROOT / 'target' / args.label / package
    if target.exists():
        raise SystemExit(f'Refusing to call an existing target directory a cold build: {target}')
    run(package, 'cold-artifacts', target)
    for i in range(args.repeats):
        run(package, f'noop-{i}', target)
    path = ROOT / source
    original = path.read_bytes()
    try:
        for i in range(args.repeats):
            # A semantic edit to a visible string, not a whitespace-only rebuild.
            changed = original.replace(b'Genesis engine lab', f'Genesis engine lab {i}'.encode())
            if changed == original:
                raise SystemExit('Measurement marker not found')
            path.write_bytes(changed)
            run(package, f'app-edit-{i}', target)
    finally:
        path.write_bytes(original)
    run(package, 'restore', target)
print(f'Results: {out / "builds.json"}')
