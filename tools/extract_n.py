#!/usr/bin/env python3
"""Extract every level of Metanet's N v1.2 (Flash) from Nv12.swf into JSON.

usage: extract_n.py path/to/Nv12.swf data/n-levels.json

The SWF is Flash 6 (ActionScript 1/2 bytecode). The three DoAction bodies that
define BuildGameData_Set0/1/2 build each level as
    new LevelData(id, data, name)
and group levels with levListK.push(lev_x) / episodeList.push(new EpisodeData(...)).
Only the Python standard library is used.

Level data string layout, as observed in the data:
    <713 tile chars, COLUMN-major: 31 columns of 23 rows>|<objects>
    tile char = chr(48 + tile_code), codes 0..33
    objects   = type^p1,p2,...!type^p1,...   (numbers, mostly integers)
Object type ids are the OBJTYPE_* constants assigned in the same bytecode.
"""
import hashlib, json, re, struct, sys, zlib

COLS, ROWS = 31, 23


def load_tags(path):
    d = open(path, 'rb').read()
    sig = d[:3]
    if sig == b'CWS':
        b = d[:8] + zlib.decompress(d[8:])
    elif sig == b'FWS':
        b = d
    else:
        sys.exit('not a SWF')
    nbits = b[8] >> 3
    p = 8 + (5 + 4 * nbits + 7) // 8 + 4
    tags = []
    while p < len(b):
        h = struct.unpack('<H', b[p:p + 2])[0]
        p += 2
        t, l = h >> 6, h & 63
        if l == 63:
            l = struct.unpack('<I', b[p:p + 4])[0]
            p += 4
        tags.append((t, b[p:p + l]))
        p += l
    return d, tags


def pushes(a):
    """Yield the decoded values of every ActionPush; constant refs are resolved."""
    pool = []
    i = 0
    while i < len(a):
        op = a[i]
        i += 1
        if op < 0x80:
            continue
        ln = struct.unpack('<H', a[i:i + 2])[0]
        i += 2
        if op == 0x88:
            n = struct.unpack('<H', a[i:i + 2])[0]
            pool = [s.decode('latin1') for s in a[i + 2:i + ln].split(b'\x00')[:n]]
        elif op == 0x96:
            j, end, vals = i, i + ln, []
            while j < end:
                t = a[j]
                j += 1
                if t == 0:
                    k = a.index(b'\x00', j)
                    vals.append(a[j:k].decode('latin1'))
                    j = k + 1
                elif t == 1:
                    vals.append(struct.unpack('<f', a[j:j + 4])[0]); j += 4
                elif t in (2, 3):
                    vals.append(None)
                elif t == 4:
                    vals.append(('reg', a[j])); j += 1
                elif t == 5:
                    vals.append(bool(a[j])); j += 1
                elif t == 6:
                    vals.append(struct.unpack('<d', a[j + 4:j + 8] + a[j:j + 4])[0]); j += 8
                elif t == 7:
                    vals.append(struct.unpack('<i', a[j:j + 4])[0]); j += 4
                elif t == 8:
                    vals.append(pool[a[j]]); j += 1
                elif t == 9:
                    vals.append(pool[struct.unpack('<H', a[j:j + 2])[0]]); j += 2
                else:
                    raise ValueError('push type %d' % t)
            yield vals
        i += ln


def objtype_names(tags):
    """OBJTYPE_* assignments: Push ['OBJTYPE_X', n]."""
    out = {}
    for t, a in tags:
        if t != 12:
            continue
        for v in pushes(a):
            if len(v) == 2 and isinstance(v[0], str) and re.fullmatch(r'OBJTYPE_[A-Z]+', v[0]) \
                    and isinstance(v[1], (int, float)):
                out[int(v[1])] = v[0][len('OBJTYPE_'):].lower()
    return out


def num(x):
    try:
        return int(x)
    except ValueError:
        return float(x)


def parse_level(data, types):
    assert data[713] == '|' and len(data) > 713, 'bad level data'
    tiles = data[:713]
    rows = [''.join(tiles[c * ROWS + r] for c in range(COLS)) for r in range(ROWS)]
    objects = []
    for rec in filter(None, data[714:].split('!')):
        t, _, rest = rec.partition('^')
        params = [num(x) for x in rest.split(',')] if rest else []
        objects.append({'type': int(t), 'name': types.get(int(t), '?'), 'params': params})
    return rows, objects


def main(src, dst):
    raw, tags = load_tags(src)
    types = objtype_names(tags)
    levels, episodes = {}, []
    for t, a in tags:
        if t != 12:
            continue
        vals = list(pushes(a))
        if not any(isinstance(x, str) and x.startswith('BuildGameData_Set') for v in vals for x in v):
            # the bodies that define a set contain levels; the registry body lists names only
            pass
        pending = []
        for v in vals:
            if len(v) >= 4 and isinstance(v[0], str) and v[0].startswith('lev_') and v[-1] == 'LevelData' \
                    and isinstance(v[1], str) and len(v[1]) > 713:
                lid, data, name = v[0][4:], v[1], v[2]
                rows, objects = parse_level(data, types)
                levels[lid] = {'id': lid, 'name': name, 'tiles': rows, 'objects': objects}
            elif len(v) == 1 and isinstance(v[0], str) and v[0].startswith('lev_'):
                pending.append(v[0][4:])
            elif v and v[-1] == 'EpisodeData':
                k = next(i for i, x in enumerate(v) if isinstance(x, str) and re.fullmatch(r'episode \d+', x))
                episodes.append({'index': int(v[k].split()[1]), 'name': v[k - 1], 'levels': pending[-5:]})
                pending = []
    episodes.sort(key=lambda e: e['index'])
    for e in episodes:
        for slot, lid in enumerate(e['levels']):
            levels[lid]['episode'], levels[lid]['slot'] = e['index'], slot
    out = {
        'source': {'file': src.split('/')[-1], 'sha256': hashlib.sha256(raw).hexdigest(),
                   'game': 'N v1.2 (Metanet Software, 2005), Flash'},
        'format': {
            'cols': COLS, 'rows': ROWS, 'tile_units': 24,
            'note': 'tiles are rows of 31 chars (source is column-major); tile code = ord(char) - 48. '
                    'Objects use pixel units with a 24 unit implicit frame, so tile (c, r) spans '
                    'x 24*(c+1)..24*(c+2), y 24*(r+1)..24*(r+2).',
        },
        'object_types': {str(k): v for k, v in sorted(types.items())},
        'episodes': episodes,
        'levels': [levels[l] for e in episodes for l in e['levels']],
    }
    json.dump(out, open(dst, 'w'), indent=1)
    print('levels', len(out['levels']), 'episodes', len(episodes), '->', dst)


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])
