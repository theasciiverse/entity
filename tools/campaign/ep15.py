"""Episode 15, mixes I: the hazards together, a little tighter each level."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

def ladder(l, xs, ys):
    for y in ys:
        for x in xs:
            l.oneway(x, y)

# 15:0 gauntlet: three lanes, a different hazard at every step
l = Level("gauntlet")
l.fill(1, 14, 27, 14)
l.fill(4, 7, 31, 7)
for x in (6, 7, 23, 24):
    l.mine_floor(x, 21)
l.thwomp(12, 15, 1)
l.floorguard(18, 21, 1)
ladder(l, (28, 29, 30, 31), (19, 17, 15))
l.fill(14, 10, 16, 11)
l.drone(13, 9, movement=0, weapon=2)
ladder(l, (1, 2, 3), (12, 10, 8))
for x0 in (10, 20):
    l.fill(x0, 3, x0 + 1, 4)
l.launcher(30, 1)
l.put(2, 21, "S"); l.put(5, 13, "K"); l.put(30, 6, "X")
for x, y in ((10, 21), (15, 21), (21, 21), (29, 16), (22, 13), (9, 13), (2, 9), (8, 6), (15, 6), (25, 6)):
    l.gold(x, y)
l.waypoints([(10, 21), (15, 21), (21, 21), (26, 21), (29, 20), (29, 16), (26, 13), (22, 13), (15, 13), (9, 13), (5, 13)], [(2, 12), (2, 9), (5, 6), (8, 6), (15, 6), (25, 6), (30, 6)])
l.write("levels/15-0.txt")

# 15:1 the well: slide down a shaft past mines on its walls, cross under a laser's windows, climb out
l = Level("the well")
l.fill(1, 3, 4, 3)
l.fill(5, 3, 5, 17)
l.fill(9, 3, 9, 17)
l.fill(21, 3, 21, 17)
l.fill(25, 3, 25, 17)
l.fill(10, 3, 20, 3)
l.fill(26, 3, 31, 3)
for x, y in ((9, 10), (21, 12)):
    l.clear(x, y, x, y + 1)
l.fill(13, 8, 17, 12)
l.drone(12, 7, movement=0, weapon=1)
for x, y in ((6, 8), (8, 12), (6, 15)):
    l.mine(x, y)
ladder(l, (22, 23, 24), range(19, 4, -2))
l.put(2, 2, "S"); l.put(16, 21, "K"); l.put(29, 2, "X")
for x, y in ((7, 5), (7, 10), (7, 18), (12, 21), (20, 21), (23, 16), (23, 10), (23, 6)):
    l.gold(x, y)
l.waypoints([(7, 2), (7, 5), (7, 10), (7, 18), (12, 21), (16, 21)], [(20, 21), (23, 20), (23, 16), (23, 10), (23, 6), (23, 4), (26, 2), (29, 2)])
l.write("levels/15-1.txt")

# 15:2 orbit: two drones round a central block; a pad and bounce blocks take you over them
l = Level("orbit")
l.fill(13, 9, 19, 13)
l.drone(12, 8, movement=0)
l.drone(20, 14, movement=1)
l.pad(5, 21)
l.fill(1, 12, 3, 12)
l.bounce(8, 10)
l.bounce(11, 8)
l.fill(14, 6, 18, 6)
l.bounce(21, 8)
l.bounce(24, 10)
l.fill(27, 12, 31, 12)
l.fill(28, 16, 31, 16)
l.put(2, 21, "S"); l.put(16, 5, "K"); l.put(30, 21, "X")
for x, y in ((5, 15), (2, 11), (8, 8), (11, 6), (21, 6), (24, 8), (29, 11), (29, 15), (16, 21), (9, 21)):
    l.gold(x, y)
l.waypoints([(9, 21), (5, 21), (5, 15), (2, 11), (8, 8), (11, 6), (16, 5)], [(21, 6), (24, 8), (29, 11), (29, 15), (24, 21), (16, 21), (30, 21)])
l.write("levels/15-2.txt")

# 15:3 bridge: a broken bridge over a field of mines, guards on it, a launcher above
l = Level("bridge")
for x0, x1 in ((1, 6), (9, 14), (17, 22), (25, 31)):
    l.fill(x0, 12, x1, 12)
for x in range(1, 32):
    l.mine_floor(x, 21)
l.floorguard(12, 11, 1)
l.floorguard(19, 11, -1)
l.launcher(16, 2)
l.fill(15, 1, 17, 1)
l.put(2, 11, "S"); l.put(30, 11, "K"); l.put(4, 11, "X")
for x in (8, 16, 24):
    l.gold(x, 9)
for x in (11, 20, 28):
    l.gold(x, 11)
l.waypoints([(8, 9), (11, 11), (16, 9), (20, 11), (24, 9), (28, 11), (30, 11)], [(24, 10), (20, 11), (16, 10), (11, 11), (8, 10), (4, 11)])
l.write("levels/15-3.txt")

# 15:4 citadel: drones behind trek doors, the key, a lift, a turret over the roof, a locked door
l = Level("citadel")
for w in (10, 20):
    l.fill(w, 15, w, 20)
    l.door(0, 0, w - 1, 21)
l.fill(10, 14, 20, 14)
l.drone(15, 18, movement=2)
l.pad(30, 21)
l.fill(25, 12, 29, 12)
l.fill(1, 12, 22, 12)
l.turret(16, 2)
l.fill(15, 1, 17, 1)
l.fill(5, 1, 5, 10)
l.door(25, 21, 4, 11, locked=1)
for x0 in (11, 17):
    l.fill(x0, 9, x0 + 1, 10)
l.put(2, 21, "S"); l.put(2, 11, "K"); l.put(9, 11, "X")
for x, y in ((6, 21), (13, 21), (17, 21), (24, 21), (30, 15), (27, 11), (20, 11), (14, 11), (8, 11)):
    l.gold(x, y)
l.waypoints([(6, 21), (13, 21), (17, 21), (24, 21), (25, 21), (30, 21), (30, 15), (28, 11), (27, 11), (20, 11), (14, 11), (8, 11), (2, 11)], [(9, 11)])
l.write("levels/15-4.txt")
