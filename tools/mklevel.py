#!/usr/bin/env python3
"""Write a campaign level file from a short description in tile coordinates.

A level is built on the 33x25 grid the campaign uses (solid frame, playfield rows
1 to 21, solid from row 22 down). Tiles and objects are given in tile columns and
rows of that grid; this turns them into the text format `Level::parse` reads, with
object positions in pixels (tile centre = 24 * tile + 12).

    from mklevel import Level
    l = Level("long jump")
    l.fill(1, 18, 5, 21)          # solid rectangle, inclusive
    l.put(2, 21, "S")             # one grid character (S, X, K, # or a tile code letter)
    l.gold(8, 14)
    l.mine_floor(7, 21)           # a mine resting on the floor of tile (7, 21)
    l.waypoints([(8, 14)], [(2, 17)])
    l.write("levels/04-0.txt")

Then `entity --prove levels/04-0.txt` finds and stores the route.
"""

W, H = 33, 25
T = 24


def px(t):
    return t * T + 12


class Level:
    def __init__(self, name):
        self.name = name
        self.grid = [[" "] * W for _ in range(H)]
        for y in range(H):
            for x in range(W):
                if x in (0, W - 1) or y == 0 or y >= 22:
                    self.grid[y][x] = "#"
        self.objects = []
        self.extra = []
        self.ways = None

    def fill(self, x0, y0, x1, y1, ch="#"):
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                self.grid[y][x] = ch

    def clear(self, x0, y0, x1, y1):
        self.fill(x0, y0, x1, y1, " ")

    def put(self, x, y, ch):
        self.grid[y][x] = ch

    def obj(self, line):
        self.objects.append(line)

    def gold(self, x, y):
        self.obj(f"gold {px(x)} {px(y)}")

    def mine(self, x, y):
        self.obj(f"mine {px(x)} {px(y)}")

    def mine_floor(self, x, y):
        """A mine resting on the floor at the bottom of tile (x, y)."""
        self.obj(f"mine {px(x)} {y * T + T - 4}")

    def mine_ceiling(self, x, y):
        """A mine hanging from the ceiling at the top of tile (x, y)."""
        self.obj(f"mine {px(x)} {y * T + 4}")

    def drone(self, x, y, movement=0, chaser=0, weapon=0, d=0):
        self.obj(f"drone {px(x)} {px(y)} {movement} {chaser} {weapon} {d}")

    def turret(self, x, y):
        self.obj(f"turret {px(x)} {px(y)}")

    def launcher(self, x, y):
        self.obj(f"hominglauncher {px(x)} {px(y)}")

    def floorguard(self, x, y, d=1):
        self.obj(f"floorguard {px(x)} {y * T + T - 6} {d}")

    def thwomp(self, x, y, d=1):
        self.obj(f"thwomp {px(x)} {px(y)} {d}")

    def bounce(self, x, y):
        self.obj(f"bounceblock {px(x)} {px(y)}")

    def pad(self, x, y, nx=0, ny=-1):
        """A launch pad on the floor of tile (x, y), pointing along (nx, ny)."""
        self.obj(f"launchpad {px(x)} {y * T + T} {nx} {ny}")

    def oneway(self, x, y, d=3):
        self.obj(f"oneway {px(x)} {px(y)} {d}")

    def door(self, tx, ty, ix, iy, vertical=0, trap=0, locked=0, at=None):
        """A door on the right edge of tile (ix, iy) (bottom edge when vertical), with its
        trigger, key or switch at tile (tx, ty), or at pixel position `at`."""
        x, y = at or (px(tx), px(ty))
        self.obj(f"door {x} {y} {vertical} {trap} {ix} {iy} {locked} 0 0")

    def waypoints(self, to_switch, to_door):
        leg = lambda pts: ";".join(f"{px(x)},{px(y)}" for x, y in pts)
        self.ways = leg(to_switch) + "|" + leg(to_door)

    def text(self):
        out = ["".join(r) for r in self.grid]
        out.append("")
        out.append(f"name {self.name}")
        out += self.objects + self.extra
        if self.ways:
            out.append(f"waypoints {self.ways}")
        return "\n".join(out) + "\n"

    def write(self, path):
        """Write the level, keeping the route `entity --prove` stored when nothing else changed."""
        text = self.text()
        try:
            old = open(path).read().splitlines(keepends=True)
            route = [l for l in old if l.startswith("route ")]
            if route and "".join(l for l in old if not l.startswith("route ")) == text:
                text += "".join(route)
        except FileNotFoundError:
            pass
        with open(path, "w") as f:
            f.write(text)
