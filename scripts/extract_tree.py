# Slim the GGG skilltree-export data.json into the tree file the app bundles.
# Usage: python3 scripts/extract_tree.py <data.json> <version> <RePoE Default.json> > src-tauri/data/tree.json
#   data.json: grindinggear/skilltree-export at the <version> tag
#   Default.json: RePoE passive_skill_trees/Default.json for the same patch, for the
#   string ids Client.txt prints (life_regeneration1087 → 55906)
# Keeps main-tree nodes only (no ascendancy, no proxies), with positions precomputed;
# the other hashes are listed under 'skip'.
import sys, json, math

data = json.load(open(sys.argv[1]))
version = sys.argv[2]
repoe = json.load(open(sys.argv[3]))['passives']
nodes = {k: v for k, v in data['nodes'].items() if k != 'root'}
groups, C = data['groups'], data['constants']
radii, per = C['orbitRadii'], C['skillsPerOrbit']
# orbits with 16 and 40 slots aren't evenly spaced
A16 = [0, 30, 45, 60, 90, 120, 135, 150, 180, 210, 225, 240, 270, 300, 315, 330]
A40 = [0, 10, 20, 30, 40, 45, 50, 60, 70, 80, 90, 100, 110, 120, 130, 135, 140, 150, 160, 170,
       180, 190, 200, 210, 220, 225, 230, 240, 250, 260, 270, 280, 290, 300, 310, 315, 320, 330, 340, 350]


def pos(v):
    g = groups[str(v['group'])]
    o, i = v.get('orbit', 0), v.get('orbitIndex', 0)
    n = per[o]
    a = math.radians(A16[i] if n == 16 else A40[i] if n == 40 else 360 * i / n)
    return round(g['x'] + radii[o] * math.sin(a)), round(g['y'] - radii[o] * math.cos(a))


def kind(v):
    if 'classStartIndex' in v: return 'start'
    if v.get('isKeystone'): return 'keystone'
    if v.get('isMastery'): return 'mastery'
    if v.get('isJewelSocket'): return 'jewel'
    if v.get('isNotable'): return 'notable'
    return 'small'


main = {k for k, v in nodes.items() if 'group' in v and not v.get('ascendancyName') and not v.get('isProxy')}
out = {}
for k in sorted(main, key=int):
    v = nodes[k]
    x, y = pos(v)
    edges = sorted({int(o) for o in v.get('out', []) + v.get('in', []) if str(o) in main}
                   | {int(j) for j, w in nodes.items() if j in main and k in map(str, w.get('out', []))})
    n = {'n': v.get('name', ''), 'k': kind(v), 'x': x, 'y': y, 'e': edges, 'st': v.get('stats', [])}
    if 'classStartIndex' in v: n['c'] = v['classStartIndex']
    if v.get('masteryEffects'): n['m'] = {str(e['effect']): e['stats'] for e in v['masteryEffects']}
    out[k] = n
# ascendancy and proxy hashes, so the app can tell them apart from nodes a patch removed
skip = sorted(int(k) for k in nodes if k not in main)
ids = {p['id']: p['hash'] for p in repoe.values() if str(p['hash']) in main}
json.dump({'version': version, 'skip': skip, 'ids': ids, 'nodes': out}, sys.stdout, separators=(',', ':'), ensure_ascii=False)
