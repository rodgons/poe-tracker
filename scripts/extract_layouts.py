# Turn Exile-UI's Act-Decoder pictures into the Zone layouts the app bundles: our own simplified line drawings,
# with our own markers, no picture or icon from the source.
# Usage:
#   python3 scripts/extract_layouts.py <Exile-UI checkout> > src-tauri/data/layouts.json
#   python3 scripts/extract_layouts.py <Exile-UI checkout> --suggest   # add marker guesses for new pictures to scripts/layout-markers.json
#   python3 scripts/extract_layouts.py <Exile-UI checkout> --review out.html   # each source picture beside our drawing
# The checkout: https://github.com/Lailloken/Exile-UI (MIT, (c) Lailloken) at the commit to bundle; its commit is recorded
# in the output. Needs ImageMagick (`magick`) to decode the JPGs. Rerun each league: --suggest lists pictures that are new
# or changed since the markers were last checked, and those need their markers checked again by hand.
#
# Pictures are `img/GUI/act-decoder/zones/<area id> <pick>.jpg`, where <pick> is a path through the zone's pick tree
# ("2", "2_1", "2_1_1"): a picture with children is a clue picture (the player's start, to match against the minimap),
# one without is a full layout. Pictures named with `x` are Act-Decoder's inconclusive samples and are left out, as
# are the zones the map ruled text-only. The white outline becomes simplified polylines, green clue lines their own
# polylines; icons are cut out before tracing and only their positions are kept, as marker guesses.
import hashlib, json, re, subprocess, sys
from pathlib import Path

ZONES = 'img/GUI/act-decoder/zones'
# zones whose layout is fixed or one of a few known ones (research for ticket #74); every other zone is random
# and gets the Guide step's text hints only. Add a zone here when a league makes its layouts known.
LAYOUT_ZONES = {
    '1_1_2', '1_1_3', '1_1_4_1', '1_1_6', '1_1_7_1', '1_1_7_2', '1_1_9', '1_2_3',
    '1_2_6_1', '1_2_6_2', '1_2_5_1', '1_2_5_2', '1_2_10', '1_2_12', '1_2_8', '1_3_2',
    '1_3_3_1', '1_3_10_1', '1_3_6', '1_3_8_1', '1_3_8_2', '1_3_9', '1_4_3_2', '1_4_3_3',
    '1_4_4_3', '1_5_6', '2_6_2', '2_6_4', '2_6_6', '2_6_7_1', '2_6_15', '2_7_2',
    '2_7_5_1', '2_7_5_2', '2_7_7', '2_7_11', '2_8_9', '2_8_13', '2_8_7_2', '2_9_5',
    '2_9_8', '2_10_1', '2_10_2', '2_10_6',
}
# text only (map #72, ticket #76): rotated in PoE1, half random, or no source pictures
TEXT_ONLY = {'1_1_2a', '1_3_14_2', '2_8_12_2', '1_2_2a', '2_8_10'}
SIZE = 600            # source pictures are 600x600
SCALE = 4             # trace at 150x150: doubled strokes merge, small gaps close, small wiggles go
EPSILON = 0.9         # Douglas-Peucker tolerance at trace scale (~0.6% of the picture): heavy simplification
MARKERS = Path(__file__).with_name('layout-markers.json')


def load(path):
    out = subprocess.run(['magick', str(path), '-depth', '8', 'rgb:-'], capture_output=True, check=True).stdout
    return out  # SIZE*SIZE*3 bytes


def classify(px):
    """Per pixel: 'w' outline white, 'g' clue green, 'p' purple icon fill, 'o' orange, 'y' grey/other icon, '' black."""
    cls = bytearray(SIZE * SIZE)
    for i in range(SIZE * SIZE):
        r, g, b = px[3 * i], px[3 * i + 1], px[3 * i + 2]
        hi, lo = max(r, g, b), min(r, g, b)
        if hi < 45:
            c = 0
        elif lo > 85 and hi - lo < 40:
            c = 1  # white or grey: the outline is anti-aliased, mostly 90-200
        elif g > 120 and r < 130 and b < 130 and g - max(r, b) > 50:
            c = 2  # green
        elif r > 25 and b > 25 and g < 0.55 * min(r, b) and hi < 140:
            c = 3  # purple
        elif r > 140 and g < 0.75 * r and b < 0.5 * r:
            c = 4  # orange
        else:
            c = 5  # other icon colour (grey metal, skull yellow, dark green...)
        cls[i] = c
    return cls


def components(mask, w, h, min_px=1):
    seen, out = bytearray(w * h), []
    for s in range(w * h):
        if not mask[s] or seen[s]:
            continue
        stack, comp = [s], []
        seen[s] = 1
        while stack:
            p = stack.pop()
            comp.append(p)
            x, y = p % w, p // w
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    q = p + dy * w + dx
                    if (dx or dy) and 0 <= x + dx < w and 0 <= y + dy < h and mask[q] and not seen[q]:
                        seen[q] = 1
                        stack.append(q)
        if len(comp) >= min_px:
            out.append(comp)
    return out


def icons(cls):
    """Marker guesses: purple squares (dots inside = suggested order, none = start), grey/metal icons, door boxes."""
    found, cut = [], []
    for kind_cls in (3, 5):
        mask = bytearray(1 if c == kind_cls else 0 for c in cls)
        for comp in components(mask, SIZE, SIZE, 120):
            xs, ys = [p % SIZE for p in comp], [p // SIZE for p in comp]
            x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
            bw, bh = x1 - x0 + 1, y1 - y0 + 1
            if not (18 <= bw <= 75 and 18 <= bh <= 75) or max(bw, bh) > 1.8 * min(bw, bh):
                continue  # halos, long lines and specks are not icons
            box = (x0 - 4, y0 - 4, x1 + 4, y1 + 4)
            inner = [cls[y * SIZE + x] for y in range(y0 + 4, y1 - 3) for x in range(x0 + 4, x1 - 3)]
            orange = sum(1 for c in inner if c == 4) / max(1, len(inner))
            dots = 0
            if kind_cls == 3:
                wmask = bytearray(SIZE * SIZE)
                for y in range(y0 + 5, y1 - 4):
                    for x in range(x0 + 5, x1 - 4):
                        if cls[y * SIZE + x] == 1:
                            wmask[y * SIZE + x] = 1
                dots = len(components(wmask, SIZE, SIZE, 4))
            if kind_cls == 3 and orange > .15:
                guess = 'exit'
            elif kind_cls == 3:
                guess = 'entrance' if dots == 0 else 'objective'
            else:
                guess = 'waypoint'
            found.append({'kind': guess, 'order': dots or None, 'at': [round((x0 + x1) / 2 / SIZE, 3), round((y0 + y1) / 2 / SIZE, 3)]})
            cut.append(box)
    # the same icon found twice (a door box is purple and orange): keep the first, which is the better guess
    kept = []
    for f in found:
        if all(abs(f['at'][0] - k['at'][0]) + abs(f['at'][1] - k['at'][1]) > .03 for k in kept):
            kept.append(f)
    return kept, cut


def thin(img, w, h):
    """Zhang-Suen thinning, in place on a 0/1 bytearray."""
    changed = True
    while changed:
        changed = False
        for step in (0, 1):
            kill = []
            for y in range(1, h - 1):
                row = y * w
                for x in range(1, w - 1):
                    if not img[row + x]:
                        continue
                    p2, p3, p4 = img[row - w + x], img[row - w + x + 1], img[row + x + 1]
                    p5, p6, p7 = img[row + w + x + 1], img[row + w + x], img[row + w + x - 1]
                    p8, p9 = img[row + x - 1], img[row - w + x - 1]
                    n = p2 + p3 + p4 + p5 + p6 + p7 + p8 + p9
                    if n < 2 or n > 6:
                        continue
                    seq = (p2, p3, p4, p5, p6, p7, p8, p9, p2)
                    if sum(1 for i in range(8) if not seq[i] and seq[i + 1]) != 1:
                        continue
                    if step == 0 and (p2 * p4 * p6 or p4 * p6 * p8):
                        continue
                    if step == 1 and (p2 * p4 * p8 or p2 * p6 * p8):
                        continue
                    kill.append(row + x)
            for p in kill:
                img[p] = 0
            changed = changed or bool(kill)


def tidy(img, w, h, spur=4):
    """After thinning: turn staircase corners into diagonals, then cut dead-end spurs shorter than `spur` px."""
    for y in range(1, h - 1):
        for x in range(1, w - 1):
            p = y * w + x
            if not img[p]:
                continue
            n, e, s, wst = img[p - w], img[p + 1], img[p + w], img[p - 1]
            if (n and e and not s and not wst and not img[p + w - 1]) or (e and s and not n and not wst and not img[p - w - 1]) \
                    or (s and wst and not n and not e and not img[p - w + 1]) or (wst and n and not s and not e and not img[p + w + 1]):
                img[p] = 0
    nb = lambda p: [p + dy * w + dx for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dx or dy) and img[p + dy * w + dx]]
    for p in range(w + 1, w * (h - 1) - 1):
        if not img[p] or len(nb(p)) != 1:
            continue
        walk, prev, cur = [p], None, p
        while len(walk) <= spur:
            nxt = [q for q in nb(cur) if q != prev]
            if len(nxt) != 1:
                break
            prev, cur = cur, nxt[0]
            if len(nb(cur)) >= 3:  # reached a junction: what we walked is a spur
                for q in walk:
                    img[q] = 0
                break
            walk.append(cur)


def chains(img, w, h):
    """Walk a 1px skeleton into polylines, breaking at ends and junctions."""
    nb = lambda p: [p + dy * w + dx for dy in (-1, 0, 1) for dx in (-1, 0, 1) if (dx or dy) and img[p + dy * w + dx]]
    used, out = set(), []
    pts = [p for p in range(w + 1, w * (h - 1) - 1) if img[p]]
    deg = {p: len(nb(p)) for p in pts}
    starts = [p for p in pts if deg[p] != 2] + pts  # ends and junctions first, then whatever loops remain
    for s in starts:
        for q in nb(s):
            if (min(s, q), max(s, q)) in used:
                continue
            line, prev, cur = [s], s, q
            used.add((min(s, q), max(s, q)))
            while True:
                line.append(cur)
                if deg[cur] != 2:
                    break
                nxt = [r for r in nb(cur) if r != prev and (min(cur, r), max(cur, r)) not in used]
                if not nxt:
                    break
                used.add((min(cur, nxt[0]), max(cur, nxt[0])))
                prev, cur = cur, nxt[0]
            out.append([(p % w, p // w) for p in line])
    return out


def simplify(pts, eps):
    if len(pts) < 3:
        return pts
    (x0, y0), (x1, y1) = pts[0], pts[-1]
    dx, dy = x1 - x0, y1 - y0
    norm = (dx * dx + dy * dy) ** .5 or 1
    far, idx = 0, 0
    for i in range(1, len(pts) - 1):
        d = abs(dy * (pts[i][0] - x0) - dx * (pts[i][1] - y0)) / norm if (dx or dy) else ((pts[i][0] - x0) ** 2 + (pts[i][1] - y0) ** 2) ** .5
        if d > far:
            far, idx = d, i
    if far <= eps:
        return [pts[0], pts[-1]]
    return simplify(pts[:idx + 1], eps)[:-1] + simplify(pts[idx:], eps)


def trace(cls, want, cut):
    """Polylines for one pixel class at trace scale, icons cut out, normalised to 0..1."""
    w = h = SIZE // SCALE
    img = bytearray(w * h)
    for y in range(SIZE):
        for x in range(SIZE):
            if cls[y * SIZE + x] == want and not any(b[0] <= x <= b[2] and b[1] <= y <= b[3] for b in cut):
                img[(y // SCALE) * w + x // SCALE] = 1
    for y in range(h):
        for x in range(w):
            if min(x, y, w - 1 - x, h - 1 - y) < 3:
                img[y * w + x] = 0  # the source's picture frame
    thin(img, w, h)
    tidy(img, w, h)
    lines = []
    for line in join([[list(p) for p in c] for c in chains(img, w, h)]):
        s = simplify([tuple(p) for p in line], EPSILON)
        length = sum(((s[i + 1][0] - s[i][0]) ** 2 + (s[i + 1][1] - s[i][1]) ** 2) ** .5 for i in range(len(s) - 1))
        xs, ys = [x for x, _ in s], [y for _, y in s]
        if length < 3 or (s[0] == s[-1] and max(max(xs) - min(xs), max(ys) - min(ys)) < .06 * w):
            continue  # specks, the dots of a dotted line, small closed shapes (building interiors)
        lines.append([[round(x / w, 3), round(y / h, 3)] for x, y in s])
    return lines


def join(lines):
    """Glue polylines that end where another starts, so the drawing has fewer, longer strokes."""
    lines = [l[:] for l in lines]
    merged = True
    while merged:
        merged = False
        for i in range(len(lines)):
            for j in range(len(lines)):
                if i == j or not lines[i] or not lines[j]:
                    continue
                a, b = lines[i], lines[j]
                if a[-1] == b[0]:
                    a.extend(b[1:]); lines[j] = []; merged = True
                elif a[-1] == b[-1]:
                    a.extend(reversed(b[:-1])); lines[j] = []; merged = True
                elif a[0] == b[-1]:
                    a[:0] = b[:-1]; lines[j] = []; merged = True
                elif a[0] == b[0]:
                    a[:0] = list(reversed(b[1:])); lines[j] = []; merged = True
        lines = [l for l in lines if l]
    return lines


def picture(path):
    px = load(path)
    cls = classify(px)
    marks, cut = icons(cls)
    return {'lines': trace(cls, 1, cut), 'clues': trace(cls, 2, cut)}, marks


def tree(files):
    """{pick id: path} -> nested nodes in pick order."""
    def kids(prefix):
        ids = sorted((k for k in files if re.fullmatch((prefix + '_' if prefix else '') + r'\d+', k)), key=lambda k: int(k.rsplit('_', 1)[-1]))
        return [{'id': k, 'children': kids(k)} for k in ids]
    return kids('')


def main():
    repo = Path(sys.argv[1])
    args = sys.argv[2:]
    commit = subprocess.run(['git', '-C', str(repo), 'rev-parse', 'HEAD'], capture_output=True, text=True, check=True).stdout.strip()
    route = json.loads((Path(__file__).parent.parent / 'src-tauri/data/route.json').read_text())
    entered = {p[1] for l in route['lines'] for p in l['parts'] if isinstance(p, list) and p[0] == 'enter'}
    towns = {k for k, a in route.get('areas', {}).items() if a.get('town')}
    by_zone = {}
    for f in sorted((repo / ZONES).glob('*.jpg')):
        m = re.fullmatch(r'(\S+) ([\d_]+)\.jpg', f.name)
        if m and m[1] in LAYOUT_ZONES and m[1] in entered and m[1] not in towns and m[1] not in TEXT_ONLY:
            by_zone.setdefault(m[1], {})[m[2]] = f
    markers = json.loads(MARKERS.read_text()) if MARKERS.exists() else {}

    if '--suggest' in args:
        fresh = 0
        for zone, files in by_zone.items():
            for pick, f in files.items():
                digest = hashlib.sha1(f.read_bytes()).hexdigest()[:12]
                known = markers.get(zone, {}).get(pick)
                is_leaf = not any(k.startswith(pick + '_') for k in files)
                if known and known.get('source') == digest:
                    continue
                _, marks = picture(f)
                markers.setdefault(zone, {})[pick] = {'source': digest, 'checked': False, 'leaf': is_leaf, 'marks': marks if is_leaf else []}
                fresh += 1
                print(f'new or changed: {zone} {pick}', file=sys.stderr)
        MARKERS.write_text(json.dumps(markers, indent=1, ensure_ascii=False) + '\n')
        print(f'{fresh} pictures to check in {MARKERS.name}', file=sys.stderr)
        return

    zones, review = {}, []
    for zone, files in by_zone.items():
        def build(node):
            drawing, guesses = picture(files[node['id']])
            out = {'id': node['id'], **drawing}
            if node['children']:
                # a start picture: where the player arrives, for its "you are here" dot (the plain
                # purple square; only a position, so the guess needs no hand check)
                start = next((g['at'] for g in guesses if g['kind'] == 'entrance'), None)
                if start:
                    out['start'] = start
                out['children'] = [build(c) for c in node['children']]
            else:
                m = markers.get(zone, {}).get(node['id'])
                if not m or not m.get('checked'):
                    print(f'unchecked markers: {zone} {node["id"]}', file=sys.stderr)
                out['marks'] = [{k: v for k, v in x.items() if v is not None} for x in (m or {}).get('marks', [])]
            review.append((zone, node['id'], files[node['id']], out))
            return out
        zones[zone] = [build(n) for n in tree(files)]
        print(f'{zone}: {len(files)} pictures', file=sys.stderr)

    if '--review' in args:
        Path(args[args.index('--review') + 1]).write_text(review_page(review))
        return
    json.dump({'source': f'https://github.com/Lailloken/Exile-UI/tree/{commit}', 'zones': zones}, sys.stdout, separators=(',', ':'))


def svg(d, size=300):
    pl = lambda l, col, wd: f'<polyline points="{" ".join(f"{x * size:.1f},{y * size:.1f}" for x, y in l)}" fill="none" stroke="{col}" stroke-width="{wd}" stroke-linejoin="round"/>'
    marks = ''.join(f'<circle cx="{m["at"][0] * size}" cy="{m["at"][1] * size}" r="6" fill="none" stroke="{dict(entrance="#7fbf6a", waypoint="#e0d6c3", exit="#d9b45a").get(m["kind"], "#8888ff")}" stroke-width="2"/><text x="{m["at"][0] * size + 8}" y="{m["at"][1] * size + 4}" fill="#e0d6c3" font-size="10">{m["kind"]}{" " + str(m["order"]) if m.get("order") else ""}{" → " + m["to"] if m.get("to") else ""}{" " + m["name"] if m.get("name") else ""}</text>' for m in d.get('marks', []))
    return f'<svg width="{size}" height="{size}" style="background:#111"><rect width="{size}" height="{size}" fill="#111"/>{"".join(pl(l, "#c8bfae", 1.2) for l in d["lines"])}{"".join(pl(l, "#5fbf4a", 1.5) for l in d["clues"])}{marks}</svg>'


def review_page(rows):
    body = ''.join(f'<section><h3>{z} · {p}{"" if "children" in d else " · layout"}</h3><img src="file://{f}" width="300" height="300"> {svg(d)}</section>' for z, p, f, d in rows)
    return f'<!doctype html><meta charset="utf-8"><title>Zone layout review</title><style>body{{background:#000;color:#ccc;font:12px system-ui}}section{{display:inline-block;margin:8px}}</style>{body}'


if __name__ == '__main__':
    main()
