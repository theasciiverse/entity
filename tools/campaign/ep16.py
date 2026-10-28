"""Episode 16, mixes II: longer combinations, less shelter."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

def ladder(l, xs, ys):
    for y in ys:
        for x in xs:
            l.oneway(x, y)

# 16:0 staircase: down a long slope under hanging mines with drones running along it, and back up
l = Level("staircase")
l.fill(1, 3, 3, 21)
for i in range(18):
    l.put(4 + i, 3 + i, "2")
    l.fill(4 + i, 4 + i, 4 + i, 21)
for i in (5, 9, 13):
    l.mine(4 + i, 3 + i - 2)
l.drone(8, 6, movement=0)
l.drone(18, 16, movement=1)
l.fill(27, 19, 31, 19)
l.fill(24, 20, 25, 20)
l.put(2, 2, "S"); l.put(29, 21, "K"); l.put(29, 18, "X")
for x, y in ((7, 4), (11, 8), (15, 12), (19, 16), (23, 21), (27, 18)):
    l.gold(x, y)
l.waypoints([(7, 4), (11, 8), (15, 12), (19, 16), (23, 21), (29, 21)], [(25, 19), (27, 18), (29, 18)])
l.write("levels/16-0.txt")

# 16:1 pendulum: three lanes; thwomps lie on their floors and slam along them; jump onto them
l = Level("pendulum")
for y, gap in ((8, (29, 30)), (15, (2, 3))):
    l.fill(1, y, 31, y)
    l.clear(gap[0], y, gap[1], y)
l.thwomp(26, 21, 2)
l.thwomp(6, 14, 0)
l.thwomp(26, 7, 2)
ladder(l, (2, 3), (20, 18, 16))
ladder(l, (29, 30), (13, 11, 9))
l.put(14, 21, "S"); l.put(2, 7, "K"); l.put(16, 7, "X")
for x in (6, 20):
    l.gold(x, 21)
for x in (10, 22):
    l.gold(x, 14)
for x in (8, 22):
    l.gold(x, 7)
l.waypoints([(20, 21), (6, 21), (3, 20), (3, 16), (10, 14), (22, 14), (29, 13), (29, 9), (22, 7), (8, 7), (2, 7)], [(8, 7), (16, 7)])
l.write("levels/16-1.txt")

# 16:2 elevator: pads up a shaft from ledge to ledge; lasers outside look in through windows
l = Level("elevator")
l.fill(10, 3, 10, 21)
l.fill(22, 3, 22, 21)
for y in (17, 9):
    l.clear(10, y, 10, y + 1)
for y in (13, 5):
    l.clear(22, y, 22, y + 1)
l.fill(3, 11, 6, 13)
l.drone(2, 10, movement=0, weapon=1)
l.fill(26, 9, 29, 11)
l.drone(25, 8, movement=1, weapon=1)
l.pad(12, 21)
for x0, y, pad in ((17, 13, 20), (11, 6, 13)):
    l.fill(x0, y, x0 + 3, y)
l.pad(20, 12)
l.put(16, 21, "S"); l.put(13, 5, "K"); l.put(18, 21, "X")
for x, y in ((12, 16), (18, 12), (20, 8), (13, 3), (21, 12), (21, 18), (14, 21)):
    l.gold(x, y)
l.waypoints([(14, 21), (12, 21), (12, 16), (18, 12), (20, 12), (20, 8), (13, 5)], [(13, 3), (21, 5), (21, 12), (21, 18), (18, 21)])
l.write("levels/16-2.txt")

# 16:3 hive: a honeycomb of short blocks to climb, drones running round them
l = Level("hive")
a_cols, b_cols = (5, 13, 21), (9, 17, 25)
for i, y in enumerate(range(20, 3, -2)):
    for x0 in (a_cols if i % 2 == 0 else b_cols):
        l.fill(x0, y, x0 + 2, y)
l.fill(1, 3, 3, 3)
l.fill(29, 3, 31, 3)
l.drone(8, 16, movement=0)
l.drone(20, 12, movement=1)
l.drone(16, 6, movement=0)
l.drone(28, 20, movement=0, chaser=1)
l.put(2, 21, "S"); l.put(30, 2, "K"); l.put(2, 2, "X")
for x, y in ((6, 19), (10, 17), (14, 15), (18, 13), (22, 11), (26, 9), (22, 7), (18, 5), (10, 5)):
    l.gold(x, y)
l.waypoints([(6, 19), (10, 17), (14, 15), (18, 13), (22, 11), (26, 9), (22, 7), (26, 5), (22, 3), (30, 2)], [(22, 3), (18, 5), (14, 3), (10, 5), (6, 3), (2, 2)])
l.write("levels/16-3.txt")

# 16:4 keep: chasers behind trek doors to the key, a ladder, a turret and a launcher over the
# roof, mines on it, the switch behind the locked door
l = Level("keep")
l.fill(1, 12, 27, 12)
for w in (9, 18):
    l.fill(w, 13, w, 20)
    l.door(0, 0, w - 1, 21)
l.drone(13, 17, movement=0, chaser=1)
l.drone(22, 17, movement=1, chaser=1)
ladder(l, (28, 29, 30, 31), (19, 17, 15, 13))
l.turret(16, 2)
l.fill(15, 1, 17, 1)
l.launcher(30, 2)
for x0 in (10, 20):
    l.fill(x0, 8, x0 + 1, 9)
for x in (15, 16):
    l.mine_floor(x, 11)
l.fill(5, 1, 5, 10)
l.door(26, 21, 4, 11, locked=1)
l.put(2, 21, "S"); l.put(2, 11, "K"); l.put(8, 11, "X")
for x, y in ((5, 21), (13, 21), (22, 21), (29, 16), (24, 11), (19, 11), (12, 11), (9, 11)):
    l.gold(x, y)
l.waypoints([(5, 21), (13, 21), (22, 21), (26, 21), (29, 18), (29, 16), (29, 12), (24, 11), (19, 11), (12, 11), (9, 11), (2, 11)], [(8, 11)])
l.write("levels/16-4.txt")
