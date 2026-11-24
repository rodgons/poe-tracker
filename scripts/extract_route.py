# Slim exile-leveling's campaign route and game data into the route file the app bundles.
# Usage: python3 scripts/extract_route.py <exile-leveling checkout> > src-tauri/data/route.json
#   The checkout: https://github.com/HeartofPhos/exile-leveling (MIT) at the commit to bundle;
#   its commit is recorded in the output. Regenerate each league, alongside the tree.
# Keeps the route as lines, each with its fragments, #sub hints and the innermost #ifdef /
# #ifndef around it (the app applies league start / bandit choice and groups the lines into
# Guide steps at runtime, as exile-leveling does). Fragments are [kind, args...] lists, plain
# text is a string, # comments are dropped. Also keeps the slim tables the app needs: areas
# (name, act, level, town), quests' gem reward offers in order, each class's starting gems, gems (name, required level) and the
# id lookups that map Path of Building gem ids onto them.
import json, re, subprocess, sys
from pathlib import Path

repo = Path(sys.argv[1])
data = repo / 'common' / 'data'
commit = subprocess.run(['git', '-C', str(repo), 'rev-parse', 'HEAD'], capture_output=True, text=True, check=True).stdout.strip()
areas = json.loads((data / 'json' / 'areas.json').read_text())
quests = json.loads((data / 'json' / 'quests.json').read_text())
gems = json.loads((data / 'json' / 'gems.json').read_text())

FRAGMENT = re.compile(r'( *#.*)|([^{#]+)|\{(.+?)\}')


def fragments(text):
    out, pos = [], 0
    while pos < len(text):
        m = FRAGMENT.match(text, pos)
        if not m:
            sys.exit(f'invalid syntax: {text!r}')
        pos = m.end()
        if m.group(1) is not None:
            break  # a comment runs to the end of the line
        if m.group(2) is not None:
            out.append(m.group(2))
        else:
            kind, *args = m.group(3).split('|')
            if kind in ('enter', 'area') or (kind == 'waypoint' and args):
                if args[0] not in areas:
                    sys.exit(f'unknown area {args[0]} in {text!r}')
            if kind == 'quest' and args[0] not in quests:
                sys.exit(f'unknown quest {args[0]} in {text!r}')
            out.append([kind, *args])
    return out


lines = []
for act in range(1, 11):
    stack = []  # (symbol, wanted) per open #ifdef / #ifndef
    for raw in (data / 'routes' / f'act-{act}.txt').read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith('#section'):
            continue
        if line.startswith('#endif'):
            stack.pop()
            continue
        m = re.match(r'#if(n?)def *(\S+)', line)
        if m:
            stack.append((m.group(2), m.group(1) == ''))
            continue
        # like exile-leveling, only the innermost condition decides whether a line counts
        when = list(stack[-1]) if stack else None
        if line.startswith('#sub'):
            parts = fragments(line[4:].strip())
            if parts:
                lines[-1]['subs'].append(parts)
            continue
        parts = fragments(line)
        if parts:
            lines.append({'act': act, 'when': when, 'parts': parts, 'subs': []})
    if stack:
        sys.exit(f'act {act}: expected #endif')

characters = json.loads((data / 'json' / 'characters.json').read_text())
gem_ids = set()
slim_quests = {}
for qid, q in quests.items():
    offers = []
    for oid, o in q['reward_offers'].items():
        take = {g: r['classes'] for g, r in o.get('quest', {}).items()}
        buy = {g: {'classes': r['classes'], 'npc': r.get('npc', '')} for g, r in o.get('vendor', {}).items()}
        gem_ids.update(take, buy)
        offers.append([oid, {'npc': o.get('quest_npc', ''), 'take': take, 'buy': buy}])
    # a list: which gem a hand-in gives depends on the offers' order
    slim_quests[qid] = {'name': q['name'], 'offers': offers}

process = (repo / 'web' / 'src' / 'state' / 'pob' / 'process.ts').read_text()
remap_block = process[process.index('GEM_ID_REMAP'):]
remap_block = remap_block[:remap_block.index('};')]
remap = dict(re.findall(r'"([^"]+)":\s*"([^"]+)"', remap_block))

out = {
    'source': f'https://github.com/HeartofPhos/exile-leveling/tree/{commit}',
    'lines': lines,
    'areas': {a['id']: {'name': a['name'], 'act': a['act'], 'level': a['level'], 'town': a['is_town_area']} for a in areas.values()},
    'quests': slim_quests,
    # each class's starting gem and the gem in Lioneye's first chest: never a quest reward to pick
    'characters': {c: [v['start_gem_id'], v['chest_gem_id']] for c, v in characters.items()},
    'gems': {g: {'name': gems[g]['name'], 'level': gems[g]['required_level']} for g in sorted(gem_ids) if g in gems},
    'remap': remap,
    'vaal': json.loads((data / 'json' / 'vaal-gem-lookup.json').read_text()),
    'awakened': json.loads((data / 'json' / 'awakened-gem-lookup.json').read_text()),
}
json.dump(out, sys.stdout, ensure_ascii=False, separators=(',', ':'))
