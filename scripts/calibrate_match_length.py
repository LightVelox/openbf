#!/usr/bin/env python3
"""calibrate_match_length.py : measure Instant Action match length and refit the menu's model.

Re-run after bot changes, from the repository root (any checkout or worktree):

    python3 scripts/calibrate_match_length.py

It builds the release `round_to_outcome` example (through scripts/heavy.sh), plays every conquest map/era entry of
`game::conquest_defaults::TABLE` (both games, 61 today) on a grid of Team size % and Reinforcements % (13 cells, one
round per entry per cell: 793 rounds), each round through heavy.sh, then fits the model and rewrites
engine/crates/ui/src/match_length.rs, which the Team size / Reinforcements help text reads. Commit that file.

Rounds: Normal bots, one Good proxy (SWEEP_PROXY, it takes one slot), SWEEP_THREADS=1, the map's defaults scaled by
SWEEP_TEAM_PCT / SWEEP_TICKET_PCT exactly as the game scales them. Each entry keeps the same seed (1-3) and proxy side
in every cell, so cells differ only by the two percentages. Limit 90,000 ticks (50 minutes); an unfinished round is
reported and its cell is left out of the fit.

Model (plain): minutes(team %, reinforcements %) = TEAM_MINUTES(team %) x (reinforcements % / 100) ^ TICKET_EXPONENT.
TEAM_MINUTES is the measured mean at 100 % reinforcements, interpolated on log-log axes between the measured team
percentages (flat above the last one: every map is at 32 per side by 200 %); TICKET_EXPONENT is the least-squares
slope of log minutes against log reinforcements % at 100 % team size. The other cells test the model; the report
prints measured against predicted for every cell.

Options: --jobs N (parallel rounds, default 3; heavy.sh still holds them to one at a time while the maintainer plays),
--fit-only (refit from the last rounds.csv), --resume (after an interruption: keep the rounds already logged in
raw/, same binary only), --out DIR (default engine/artifacts/match_length, untracked).
Outputs in --out: rounds.csv (one row per round), cells.csv, report.md, raw/ (each round's stdout/stderr), the copied
binary and its SHA256."""
import argparse, concurrent.futures as cf, csv, hashlib, math, os, re, shutil, subprocess, sys, time

ROOT = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
ENGINE = os.path.join(ROOT, 'engine')
TABLE_RS = os.path.join(ENGINE, 'crates/game/src/conquest_defaults.rs')
MODEL_RS = os.path.join(ENGINE, 'crates/ui/src/match_length.rs')
TICKS = 90_000
TEAM_AXIS = [10, 25, 50, 75, 100, 150, 200]  # at 100 % reinforcements
TICKET_AXIS = [50, 200, 300]                 # at 100 % team size
CHECKS = [(50, 200), (200, 50), (200, 200)]  # not fitted: they test the model
CELLS = [(t, 100) for t in TEAM_AXIS] + [(100, r) for r in TICKET_AXIS] + CHECKS


def heavy_sh():
    """The main checkout's heavy.sh (its locks are the machine's), also from a worktree."""
    common = subprocess.run(['git', '-C', ROOT, 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                            capture_output=True, text=True, check=True).stdout.strip()
    return os.path.join(os.path.dirname(common), 'scripts', 'heavy.sh')


def table():
    """(map, cw, combatants, tickets) for every TABLE entry, read from the Rust source."""
    pat = re.compile(r'entry\("(\w+)", (true|false), \[(\d+), (\d+)\], \[(\d+), (\d+)\]\)')
    rows = [(m, cw == 'true', [int(a), int(b)], [int(c), int(d)]) for m, cw, a, b, c, d in pat.findall(open(TABLE_RS).read())]
    if len(rows) < 2:
        sys.exit('no TABLE entries found in ' + TABLE_RS)
    return rows


def team_counts(defaults, pct):
    """ui::population::team_counts: halves up, the pair stops at 32 together."""
    largest, pct = max(max(defaults), 1), min(max(pct, 10), 500)
    num, den = (32, largest) if largest * pct > 3200 else (pct, 100)
    return [max((n * num + den // 2) // den, 1) for n in defaults]


def tickets(defaults, pct):
    """ui::population::tickets."""
    return [max((n * min(max(pct, 10), 500) + 50) // 100, 1) for n in defaults]


def build(heavy):
    subprocess.run(['bash', heavy, 'cargo', 'build', '--release', '-p', 'game', '--example', 'round_to_outcome'],
                   cwd=ENGINE, check=True)
    return os.path.join(ENGINE, 'target/release/examples/round_to_outcome')


def play(*args):
    """One round; a failed round is retried once (a lock or launch hiccup), then stops the run."""
    try:
        return play_once(*args)
    except RuntimeError as e:
        print('retrying once:', e, flush=True)
        return play_once(*args[:3], False, *args[4:])


def play_once(heavy, binary, out, resume, entry, index, team_pct, ticket_pct):
    m, cw, comb, tick = entry
    seed, side = 1 + index % 3, (index // 3) % 2
    name = f'{m}_{"cw" if cw else "gcw"}_t{team_pct}_r{ticket_pct}'
    env = {k: v for k, v in os.environ.items() if not k.startswith(('SWEEP_', 'SLOT0', 'BLACKBOX', 'MATCHLOG'))}
    env.update(SWEEP_THREADS='1', SWEEP_DIFFICULTY='normal', SWEEP_PROXY=f'{side}:good',
               SWEEP_ERA='cw' if cw else 'gcw', SWEEP_TEAM_PCT=str(team_pct), SWEEP_TICKET_PCT=str(ticket_pct),
               OPENBF_BLACKBOX='0', OPENBF_MATCHLOG='0', HEAVY_TIMEOUT='900')
    raw = os.path.join(out, 'raw', name)
    want = team_counts(comb, team_pct) + tickets(tick, ticket_pct)

    def parse(stdout, stderr):
        setup = re.search(r'setup \S+ combatants (\d+)/(\d+) tickets (\d+)/(\d+)', stdout)
        proxy = re.search(r'^round .* proxy (\d) good', stderr, re.M)
        ok = bool(setup and proxy and int(proxy[1]) == side and [int(x) for x in setup.groups()] == want)
        return ok, setup

    ok = False
    if resume and os.path.exists(raw + '.out') and os.path.exists(raw + '.err'):
        stdout, stderr = open(raw + '.out').read(), open(raw + '.err').read()
        ok, setup = parse(stdout, stderr)
    if not ok:
        p = subprocess.run(['bash', heavy, binary, str(TICKS), str(seed), m], cwd=ENGINE, env=env,
                           capture_output=True, text=True)
        stdout, stderr = p.stdout, p.stderr
        with open(raw + '.out', 'w') as f:
            f.write(stdout)
        with open(raw + '.err', 'w') as f:
            f.write(stderr)
        ok, setup = parse(stdout, stderr)
        if p.returncode or not ok:
            raise RuntimeError(f'{name}: exit {p.returncode}, setup {setup and setup.groups()} want {want}; see raw/{name}.*')
    win = re.search(r'team (\d) wins at tick (\d+)', stdout)
    return dict(map=m, era='cw' if cw else 'gcw', team_pct=team_pct, ticket_pct=ticket_pct, seed=seed, proxy_side=side,
                combatants=f'{want[0]}/{want[1]}', tickets=f'{want[2]}/{want[3]}', finished=int(bool(win)),
                minutes=f'{int(win[2]) / 1800:.4f}' if win else '', winner=win[1] if win else '')


def measure(out, jobs, resume):
    heavy = heavy_sh()
    entries = table()
    os.makedirs(os.path.join(out, 'raw'), exist_ok=True)
    binary = os.path.join(out, 'round_to_outcome')
    sha_file = os.path.join(out, 'binary_sha256.txt')
    sha = lambda f: hashlib.sha256(open(f, 'rb').read()).hexdigest()
    if resume:  # the interrupted run's own copied binary, unchanged
        if not os.path.exists(binary) or sha(binary) != open(sha_file).read().strip():
            sys.exit('--resume needs the interrupted run\'s binary in ' + out + '; run without --resume')
    else:
        shutil.copy2(build(heavy), binary)
        with open(sha_file, 'w') as f:
            f.write(sha(binary) + '\n')
    work = [(e, i, t, r) for (t, r) in CELLS for i, e in enumerate(entries)]
    rows, start = [], time.monotonic()
    with cf.ThreadPoolExecutor(max_workers=jobs) as pool:
        futs = [pool.submit(play, heavy, binary, out, resume, *w) for w in work]
        for n, fut in enumerate(cf.as_completed(futs), 1):
            try:
                rows.append(fut.result())
            except Exception:
                pool.shutdown(cancel_futures=True)
                raise
            if n % 25 == 0 or n == len(work):
                print(f'{n}/{len(work)} rounds, {time.monotonic() - start:.0f} s', flush=True)
    rows.sort(key=lambda r: (CELLS.index((r['team_pct'], r['ticket_pct'])), r['map'], r['era']))
    with open(os.path.join(out, 'rounds.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)


def interp(axis, x):
    """Log-log interpolation through [(pct, minutes)], flat past the last point, the first slope below the first."""
    if x >= axis[-1][0]:
        return axis[-1][1]
    i = max(j for j in range(len(axis) - 1) if axis[j][0] <= x) if x >= axis[0][0] else 0
    (x0, y0), (x1, y1) = axis[i], axis[i + 1]
    s = math.log(y1 / y0) / math.log(x1 / x0)
    return y0 * (x / x0) ** s


def predict(team_axis, exponent, team_pct, ticket_pct):
    return interp(team_axis, team_pct) * (ticket_pct / 100) ** exponent


def fit(out):
    rows = list(csv.DictReader(open(os.path.join(out, 'rounds.csv'))))
    cells = []
    for t, r in CELLS:
        rs = [x for x in rows if (int(x['team_pct']), int(x['ticket_pct'])) == (t, r)]
        done = [float(x['minutes']) for x in rs if x['finished'] == '1']
        mean = sum(done) / len(done) if done else float('nan')
        sd = math.sqrt(sum((v - mean) ** 2 for v in done) / (len(done) - 1)) if len(done) > 1 else float('nan')
        team = sum(sum(map(int, x['combatants'].split('/'))) / 2 for x in rs) / len(rs)
        tick = sum(sum(map(int, x['tickets'].split('/'))) / 2 for x in rs) / len(rs)
        cells.append(dict(team_pct=t, ticket_pct=r, rounds=len(rs), unfinished=len(rs) - len(done), mean_minutes=mean,
                          se_minutes=sd / math.sqrt(len(done)) if done else float('nan'), team=team, tickets=tick))
    cell = {(c['team_pct'], c['ticket_pct']): c for c in cells}
    usable = lambda k: k in cell and cell[k]['unfinished'] == 0
    team_axis = [(t, cell[(t, 100)]['mean_minutes']) for t in TEAM_AXIS if usable((t, 100))]
    base = cell[(100, 100)]['mean_minutes']
    pts = [(math.log(r / 100), math.log(cell[(100, r)]['mean_minutes'] / base)) for r in TICKET_AXIS if usable((100, r))]
    exponent = sum(x * y for x, y in pts) / sum(x * x for x, _ in pts)
    for c in cells:
        c['predicted'] = predict(team_axis, exponent, c['team_pct'], c['ticket_pct'])
        c['error_pct'] = 100 * (c['predicted'] / c['mean_minutes'] - 1)
        c['role'] = 'check' if (c['team_pct'], c['ticket_pct']) in CHECKS else 'fit'
    with open(os.path.join(out, 'cells.csv'), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(cells[0]))
        w.writeheader()
        w.writerows(cells)
    with open(MODEL_RS, 'w') as f:
        f.write(render(team_axis, exponent, cells))
    lines = ['# Match length calibration', '',
             f'binary sha256 `{open(os.path.join(out, "binary_sha256.txt")).read().strip()}`; {len(rows)} rounds; '
             f'model: minutes = TEAM_MINUTES(team %) x (reinforcements % / 100)^{exponent:.3f}', '',
             '| team % | reinf. % | role | rounds | unfinished | mean team | mean tickets | measured min (se) | predicted | error |',
             '|---|---|---|---|---|---|---|---|---|---|']
    for c in cells:
        lines.append(f'| {c["team_pct"]} | {c["ticket_pct"]} | {c["role"]} | {c["rounds"]} | {c["unfinished"]} | '
                     f'{c["team"]:.1f} | {c["tickets"]:.0f} | {c["mean_minutes"]:.2f} ({c["se_minutes"]:.2f}) | '
                     f'{c["predicted"]:.2f} | {c["error_pct"]:+.1f}% |')
    open(os.path.join(out, 'report.md'), 'w').write('\n'.join(lines) + '\n')
    print('\n'.join(lines))
    print('wrote', MODEL_RS)


def render(team_axis, exponent, cells):
    """The Rust file the menu reads (its test re-checks MEASURED against the model)."""
    s = ['//! Generated by `scripts/calibrate_match_length.py`; do not edit by hand, re-run the script.',
         '//! Mean Instant Action conquest match length over every map/era default (Normal bots, one Good proxy).',
         '//! minutes = TEAM_MINUTES(team %) x (reinforcements % / 100) ^ TICKET_EXPONENT.', '',
         '/// Measured mean minutes at 100% reinforcements: (team size %, minutes).',
         f'pub const TEAM_MINUTES: [(u32, f32); {len(team_axis)}] = [']
    s += [f'    ({t}, {m:.3f}),' for t, m in team_axis]
    s += ['];', '', '/// Least-squares slope of log minutes against log reinforcements % at 100% team size.',
          f'pub const TICKET_EXPONENT: f32 = {exponent:.4f};', '',
          '/// Every measured cell: (team %, reinforcements %, rounds, unfinished, mean minutes).',
          f'pub const MEASURED: [(u32, u32, u32, u32, f32); {len(cells)}] = [']
    s += [f'    ({c["team_pct"]}, {c["ticket_pct"]}, {c["rounds"]}, {c["unfinished"]}, {c["mean_minutes"]:.3f}),'
          for c in cells]
    s += ['];', '']
    return '\n'.join(s)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--jobs', type=int, default=3)
    ap.add_argument('--fit-only', action='store_true')
    ap.add_argument('--resume', action='store_true')
    ap.add_argument('--out', default=os.path.join(ENGINE, 'artifacts', 'match_length'))
    a = ap.parse_args()
    if not a.fit_only:
        measure(a.out, a.jobs, a.resume)
    fit(a.out)


if __name__ == '__main__':
    main()
