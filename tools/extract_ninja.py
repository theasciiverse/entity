"""Extract the ninja's animation from Metanet's N v1.2 SWF into data/n-ninja.json.

    python3 tools/extract_ninja.py path/to/Nv12.swf data/n-ninja.json

The in-game ninja is DefineSprite 582 (exported as testNinjaMCm, created by the player
code in DoAction tag 2707). Each of its frames places rigid limb clips with their own
matrices: thighs (137) and upper arms (140) are lines, shins (139) and forearms (142)
are thin filled wedges, the torso is two lines (143, 144) and the head (146) is a small
filled triangle. The clip is drawn at 20% (mc._xscale = r * 2), so clip units x 0.2 are
game pixels, origin at the collision circle centre, y down, facing right.

Output: for each of frames 0..103 (STAND 0..10, SKID 11, RUN 12..83, JUMP 84..102,
WALLSLIDE 103) a list of line segments [x1, y1, x2, y2] (each wedge becomes the line
from its tip to the middle of its base) and the head triangle. The celebrations after
frame 103 are not extracted."""
import json, os, struct, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import swf


class Bits:
    def __init__(s, b, p):
        s.b, s.bp = b, p * 8

    def u(s, n):
        v = 0
        for _ in range(n):
            v = (v << 1) | ((s.b[s.bp >> 3] >> (7 - (s.bp & 7))) & 1)
            s.bp += 1
        return v

    def s(s, n):
        v = s.u(n)
        if n and v >> (n - 1):
            v -= 1 << n
        return v

    def align(s):
        s.bp = (s.bp + 7) // 8 * 8


def shape_points(t, d):
    """Vertices of a DefineShape's edges in order (all the ninja's limbs are straight)."""
    r = Bits(d, 2)
    n = r.u(5)
    for _ in range(4):
        r.s(n)
    r.align()
    p = r.bp // 8

    def styles(p):
        fc = d[p]; p += 1
        if fc == 0xff:
            fc = struct.unpack('<H', d[p:p + 2])[0]; p += 2
        for _ in range(fc):
            if d[p] != 0:
                raise ValueError('only solid fills expected')
            p += 1 + (4 if t >= 32 else 3)
        lc = d[p]; p += 1
        if lc == 0xff:
            lc = struct.unpack('<H', d[p:p + 2])[0]; p += 2
        p += lc * (2 + (4 if t >= 32 else 3))
        return fc, p

    fc, p = styles(p)
    r = Bits(d, p)
    nf, nl = r.u(4), r.u(4)
    x = y = 0
    pts = []
    while True:
        if r.u(1) == 0:
            fl = r.u(5)
            if fl == 0:
                break
            if fl & 1:
                n = r.u(5); x = r.s(n); y = r.s(n)
            if fl & 2: r.u(nf)
            if fl & 4: r.u(nf)
            if fl & 8: r.u(nl)
            if fl & 16:
                raise ValueError('new styles not expected')
        elif r.u(1):
            n = r.u(4) + 2
            if r.u(1):
                dx, dy = r.s(n), r.s(n)
            elif r.u(1):
                dx, dy = 0, r.s(n)
            else:
                dx, dy = r.s(n), 0
            pts.append(((x / 20, y / 20), ((x + dx) / 20, (y + dy) / 20)))
            x += dx; y += dy
        else:
            raise ValueError('curves not expected in the ninja')
    return fc > 0, pts


def matrix(b, p):
    r = Bits(b, p)
    sx = sy = 1.0; r0 = r1 = 0.0
    if r.u(1):
        n = r.u(5); sx = r.s(n) / 65536; sy = r.s(n) / 65536
    if r.u(1):
        n = r.u(5); r0 = r.s(n) / 65536; r1 = r.s(n) / 65536
    n = r.u(5)
    tx = r.s(n) / 20; ty = r.s(n) / 20
    r.align()
    return (sx, r0, r1, sy, tx, ty), r.bp // 8


def place(d):
    """PlaceObject2: depth, move flag, character id and matrix (colour transforms skipped)."""
    f = d[0]; p = 1
    o = {'depth': struct.unpack('<H', d[p:p + 2])[0], 'move': bool(f & 1)}
    p += 2
    if f & 2:
        o['cid'] = struct.unpack('<H', d[p:p + 2])[0]; p += 2
    if f & 4:
        o['m'], p = matrix(d, p)
    return o


def timeline(d):
    """Display list after each frame of a DefineSprite body."""
    p = 4
    dl, frames = {}, []
    while p < len(d):
        h = struct.unpack('<H', d[p:p + 2])[0]; p += 2
        t, l = h >> 6, h & 63
        if l == 63:
            l = struct.unpack('<I', d[p:p + 4])[0]; p += 4
        body = d[p:p + l]; p += l
        if t == 1:
            frames.append({k: dict(v) for k, v in dl.items()})
        elif t == 28:
            del dl[struct.unpack('<H', body[:2])[0]]
        elif t == 26:
            o = place(body)
            if o['move'] and o['depth'] in dl:
                dl[o['depth']].update({k: o[k] for k in ('cid', 'm') if k in o})
            else:
                dl[o['depth']] = {'cid': o['cid'], 'm': o.get('m', (1, 0, 0, 1, 0, 0))}
    return frames


def mul(A, B):
    a, b, c, d, e, f = A
    a2, b2, c2, d2, e2, f2 = B
    return (a * a2 + c * b2, b * a2 + d * b2, a * c2 + c * d2, b * c2 + d * d2, a * e2 + c * f2 + e, b * e2 + d * f2 + f)


def ap(M, q):
    a, b, c, d, e, f = M
    return (a * q[0] + c * q[1] + e, b * q[0] + d * q[1] + f)


NINJA, MARKER, HEAD = 582, 135, 146
LIMBS = {137, 139, 140, 142, 143, 144, HEAD}


def main(src, dst):
    chars = {}
    for t, d in swf.load(src):
        if t in (2, 22, 32, 39):
            chars[struct.unpack('<H', d[:2])[0]] = (t, d)
    sprites = {}

    def flatten(cid, M, limb):
        t, d = chars[cid]
        if t != 39:
            filled, edges = shape_points(t, d)
            pts = [ap(M, a) for a, _ in edges] if filled else [ap(M, edges[0][0]), ap(M, edges[0][1])]
            yield limb, filled, pts
            return
        if cid not in sprites:
            sprites[cid] = timeline(d)
        for dep, e in sorted(sprites[cid][0].items()):
            if e['cid'] != MARKER:
                yield from flatten(e['cid'], mul(M, e['m']), limb or (e['cid'] if e['cid'] in LIMBS else None))

    frames = timeline(chars[NINJA][1])
    out = []
    S = (0.2, 0, 0, 0.2, 0, 0)
    for f in range(104):
        lines, head = [], None
        for dep, e in sorted(frames[f].items()):
            if e['cid'] == MARKER:
                continue
            for limb, filled, pts in flatten(e['cid'], mul(S, e['m']), e['cid'] if e['cid'] in LIMBS else None):
                if limb == HEAD:
                    head = pts
                elif filled:
                    # a wedge: the line from its tip to the middle of its shortest side
                    k = min(range(3), key=lambda i: (pts[i][0] - pts[(i + 1) % 3][0]) ** 2 + (pts[i][1] - pts[(i + 1) % 3][1]) ** 2)
                    a, b, tip = pts[k], pts[(k + 1) % 3], pts[(k + 2) % 3]
                    lines.append([tip[0], tip[1], (a[0] + b[0]) / 2, (a[1] + b[1]) / 2])
                else:
                    lines.append([pts[0][0], pts[0][1], pts[1][0], pts[1][1]])
        assert head and len(head) == 3 and len(lines) == 10, (f, len(lines))
        out.append({'lines': [[round(v, 2) for v in l] for l in lines], 'head': [[round(v, 2) for v in q] for q in head]})
    doc = {
        'source': 'Metanet N v1.2, Nv12.swf, DefineSprite 582 (testNinjaMCm), frames 0..103',
        'units': 'game pixels, origin at the collision circle centre (radius 10), y down, facing right',
        'labels': {'stand': [0, 10], 'skid': [11, 11], 'run': [12, 83], 'air': [84, 102], 'wallslide': [103, 103]},
        'frames': out,
    }
    with open(dst, 'w') as fo:
        json.dump(doc, fo, separators=(',', ':'))
        fo.write('\n')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
