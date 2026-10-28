"""Episode 19, the finale."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

def ladder(l, xs, ys):
    for y in ys:
        for x in xs:
            l.oneway(x, y)

def lanes(l, floors):
    """Floors across the level with a ladder gap at alternate ends, the first gap on the right."""
    for i, y in enumerate(floors):
        l.fill(1, y, 31, y)
        gap = (29, 30) if i % 2 == 0 else (2, 3)
        l.clear(gap[0], y, gap[1], y)
        ladder(l, gap, (y + 2,))

# 19:0 ascent: five tiers, a different danger on each, the switch at the very top
l = Level("ascent")
lanes(l, (17, 13, 9, 5))
for x in range(6, 26):
    if x % 5 in (2, 3):
        l.mine_floor(x, 21)
l.floorguard(16, 21, 1)
for x in (10, 20):
    l.thwomp(x, 14, 1)
l.drone(16, 11, movement=2, d=0)
l.launcher(30, 6)
l.turret(16, 2)
l.fill(15, 1, 17, 1)
l.fill(8, 3, 9, 4)
l.fill(22, 3, 23, 4)
l.put(2, 21, "S"); l.put(30, 4, "K"); l.put(16, 4, "X")
for x, y in ((9, 20), (24, 20), (24, 16), (8, 16), (8, 12), (24, 12), (24, 8), (8, 8), (12, 4), (26, 4)):
    l.gold(x, y)
l.waypoints([(9, 20), (24, 20), (29, 19), (29, 16), (24, 16), (15, 16), (8, 16), (3, 15), (3, 12), (8, 12), (24, 12), (29, 11), (29, 8), (24, 8), (8, 8), (3, 7), (3, 4), (12, 4), (20, 4), (26, 4), (30, 4)], [(20, 4), (16, 4)])
l.write("levels/19-0.txt")

# 19:1 cascade: down ledge by ledge, left and right, past thwomps, mines, a laser and chasers
l = Level("cascade")
ledges = [(1, 9, 4), (14, 23, 8), (6, 15, 12), (18, 27, 16)]
for x0, x1, y in ledges:
    l.fill(x0, y, x1, y)
l.thwomp(18, 5, 1)
l.mine_floor(11, 11)
l.mine_floor(20, 7)
l.fill(26, 2, 28, 3)
l.drone(25, 1, movement=0, weapon=1)
l.drone(10, 15, movement=2, chaser=1)
for x in range(2, 17):
    if x % 4 == 0:
        l.mine_floor(x, 21)
l.put(2, 3, "S"); l.put(4, 21, "K"); l.put(29, 21, "X")
for x, y in ((6, 3), (16, 7), (22, 7), (8, 11), (13, 11), (20, 15), (25, 15), (10, 21), (22, 21)):
    l.gold(x, y)
l.waypoints([(6, 3), (11, 6), (16, 7), (22, 7), (13, 10), (8, 11), (13, 11), (16, 11), (20, 15), (25, 15), (22, 21), (10, 21), (4, 21)], [(10, 21), (22, 21), (29, 21)])
l.write("levels/19-1.txt")

# 19:2 labyrinth: four rooms, two locked doors, a one-way ladder and a drop; a laser and a
# chaingun keep the upper rooms
l = Level("labyrinth")
l.fill(16, 1, 16, 20)
l.fill(1, 11, 31, 11)
l.door(1, 14, 15, 21, locked=1)
l.door(24, 16, 15, 10, locked=1)
l.clear(16, 10, 16, 10)
for x0, x1, y in ((11, 12, 20), (8, 9, 19), (4, 6, 17), (1, 2, 15)):
    l.fill(x0, y, x1, y)
for x0, x1, y in ((18, 19, 20), (20, 21, 19), (23, 25, 17)):
    l.fill(x0, y, x1, y)
l.clear(29, 11, 30, 11)
ladder(l, (29, 30), (19, 17, 15, 13))
l.clear(2, 11, 3, 11)
l.fill(22, 5, 24, 6)
l.drone(21, 4, movement=0, weapon=1)
l.fill(6, 4, 8, 5)
l.drone(5, 3, movement=1, weapon=2)
l.put(9, 21, "S"); l.put(8, 10, "K"); l.put(14, 21, "X")
for x, y in ((8, 18), (5, 16), (20, 21), (24, 21), (29, 16), (26, 10), (20, 10), (12, 10), (3, 9)):
    l.gold(x, y)
l.waypoints([(12, 19), (8, 18), (5, 16), (1, 14), (14, 21), (20, 21), (19, 19), (20, 18), (24, 16), (24, 21), (29, 20), (29, 16), (29, 12), (29, 10), (26, 10), (20, 10), (12, 10), (8, 10)], [(4, 10), (3, 9), (2, 10), (2, 14), (6, 21), (14, 21)])
l.write("levels/19-2.txt")

# 19:3 siege: two turrets and two launchers over a field of bunkers, a walkway back over the top
l = Level("siege")
roofs = (4, 10, 16, 22)
for x0 in roofs:
    l.fill(x0, 18, x0 + 3, 18)
for x in (9, 15, 21, 27):
    l.mine_floor(x, 21)
l.turret(8, 1)
l.turret(24, 1)
l.launcher(2, 8)
l.launcher(30, 8)
l.fill(6, 11, 26, 11)
for x0 in (11, 20):
    l.fill(x0, 9, x0 + 1, 10)
ladder(l, (28, 29), (19, 17, 15, 13, 11))
l.fill(1, 11, 3, 11)
l.put(2, 21, "S"); l.put(30, 21, "K"); l.put(2, 10, "X")
for x0 in roofs:
    l.gold(x0 + 2, 21)
for x in (24, 16, 8):
    l.gold(x, 10)
l.waypoints([(6, 21), (12, 21), (18, 21), (24, 21), (30, 21)], [(28, 20), (28, 14), (28, 10), (24, 10), (16, 10), (8, 10), (4, 9), (2, 10)])
l.write("levels/19-3.txt")

# 19:4 entity: the whole game in one level
l = Level("entity")
for y, gap in ((14, (29, 30)), (7, (2, 3))):
    l.fill(1, y, 31, y)
    l.clear(gap[0], y, gap[1], y)
ladder(l, (29, 30), (19, 17, 15))
ladder(l, (2, 3), (12, 10, 8))
for x in range(5, 25):
    if x % 5 in (1, 2):
        l.mine_floor(x, 21)
l.drone(8, 15, movement=2, chaser=1)
l.thwomp(20, 15, 1)
for x in range(9, 22):
    l.mine_floor(x, 13)
for x in (11, 15, 19):
    l.bounce(x, 11)
l.fill(25, 9, 26, 10)
l.drone(24, 8, movement=0, weapon=1)
for x in (6, 12):
    for y in range(8, 14):
        l.oneway(x, y, 2)
l.turret(16, 2)
l.fill(15, 1, 17, 1)
l.launcher(30, 1)
for x0 in (9, 21):
    l.fill(x0, 4, x0 + 1, 5)
l.fill(27, 1, 27, 5)
l.door(26, 21, 26, 6, locked=1)
l.put(2, 21, "S"); l.put(30, 6, "K"); l.put(16, 6, "X")
for x, y in ((4, 21), (9, 21), (14, 21), (19, 21), (24, 21), (29, 16), (23, 13), (15, 9), (8, 13), (3, 9), (8, 6), (22, 6)):
    l.gold(x, y)
l.waypoints([(4, 21), (9, 21), (14, 21), (19, 21), (24, 21), (26, 21), (29, 20), (29, 16), (27, 13), (23, 13), (19, 10), (15, 9), (11, 10), (8, 13), (4, 13), (3, 12), (3, 9), (5, 6), (8, 6), (16, 6), (22, 6), (30, 6)], [(22, 6), (16, 6)])
l.write("levels/19-4.txt")
