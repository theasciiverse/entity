"""Episode 11, gauss turrets: a crosshair that follows you and fires when it settles."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 11:0 sniper: a turret high in the middle of the room; keep moving, rest behind the walls
l = Level("sniper")
l.fill(14, 4, 18, 4)
l.turret(16, 5)
for x in (6, 12, 20, 26):
    l.fill(x, 18, x, 21)
l.fill(1, 14, 4, 14)
l.fill(28, 14, 31, 14)
l.fill(1, 12, 2, 12)
l.put(16, 21, "S"); l.put(30, 13, "K"); l.put(1, 11, "X")
for x, y in ((23, 21), (29, 21), (3, 21), (16, 12), (8, 12)):
    l.gold(x, y)
l.waypoints([(23, 21), (29, 21), (31, 17), (30, 13)], [(24, 13), (16, 12), (8, 12), (3, 13), (3, 21), (2, 17), (4, 13), (1, 11)])
l.write("levels/11-0.txt")

# 11:1 bunkers: low roofs over the floor under the turret's eye, mines in the open between them
l = Level("bunkers")
roofs = (4, 11, 18, 25)
for x0 in roofs:
    l.fill(x0, 18, x0 + 3, 18)
for x in (9, 16, 23):
    l.mine_floor(x, 21)
l.fill(15, 1, 17, 1)
l.turret(16, 2)
l.fill(1, 13, 2, 13)
for y in (19, 17, 15):
    l.oneway(3, y)
l.put(5, 21, "S"); l.put(30, 21, "K"); l.put(1, 12, "X")
for x0 in roofs:
    l.gold(x0 + 2, 21)
l.gold(3, 16)
l.waypoints([(13, 21), (20, 21), (27, 21), (30, 21)], [(27, 21), (20, 21), (13, 21), (6, 21), (3, 20), (3, 16), (3, 14), (1, 12)])
l.write("levels/11-1.txt")

# 11:2 two snipers: a turret in each top corner; climb the middle behind the walls on the ledges
l = Level("two snipers")
l.turret(2, 2)
l.turret(30, 2)
l.fill(1, 3, 3, 3)
l.fill(29, 3, 31, 3)
rows = (19, 17, 15, 13, 11, 9, 7, 5)
for i, y in enumerate(rows):
    if i % 2 == 0:
        l.fill(11, y, 14, y)
        l.fill(11, y - 1, 11, y - 1)
    else:
        l.fill(18, y, 21, y)
        l.fill(21, y - 1, 21, y - 1)
l.put(16, 21, "S"); l.put(16, 2, "K"); l.put(4, 21, "X")
for i, y in enumerate(rows):
    l.gold(13 if i % 2 == 0 else 19, y - 1)
l.gold(9, 21)
l.waypoints([(13, 18), (19, 16), (13, 14), (19, 12), (13, 10), (19, 8), (13, 6), (19, 4), (16, 2)], [(16, 10), (16, 20), (9, 21), (4, 21)])
l.write("levels/11-2.txt")

# 11:3 from below: a turret in a pit of mines looks up; cross high above on platforms that hide you
l = Level("from below")
for x in range(9, 24):
    if x != 16:
        l.mine_floor(x, 21)
l.turret(16, 21)
l.fill(1, 17, 7, 21)
l.fill(25, 17, 31, 21)
for x0, y in ((8, 15), (13, 13), (19, 13), (24, 15)):
    l.fill(x0, y, x0 + 2, y)
l.fill(1, 14, 3, 14)
l.fill(29, 14, 31, 14)
l.put(3, 16, "S"); l.put(30, 16, "K"); l.put(2, 13, "X")
for x, y in ((9, 14), (14, 12), (20, 12), (25, 14), (17, 10)):
    l.gold(x, y)
l.waypoints([(5, 16), (9, 14), (14, 12), (17, 10), (20, 12), (25, 14), (30, 16)], [(27, 16), (25, 14), (20, 12), (14, 12), (9, 14), (2, 13)])
l.write("levels/11-3.txt")

# 11:4 outpost: crates under the turret's eye to the key, a lift, a drone round a block, and the
# switch behind a locked door
l = Level("outpost")
l.fill(14, 10, 18, 11)
l.turret(16, 9)
for x in (6, 12, 20, 25):
    l.fill(x, 20, x + 1, 21)
l.pad(31, 21)
l.fill(26, 11, 29, 11)
l.fill(19, 8, 23, 8)
l.fill(10, 8, 12, 8)
l.fill(1, 8, 7, 8)
l.fill(8, 3, 9, 4)
l.drone(7, 2, movement=0)
l.fill(4, 1, 4, 6)
l.door(29, 21, 3, 7, locked=1)
l.put(2, 21, "S"); l.put(2, 7, "K"); l.put(22, 7, "X")
for x, y in ((9, 21), (16, 21), (23, 21), (31, 15), (27, 10), (24, 9), (16, 7), (11, 7), (6, 7)):
    l.gold(x, y)
l.waypoints([(9, 21), (16, 21), (23, 21), (29, 21), (31, 15), (28, 10), (27, 10), (24, 9), (21, 7), (16, 7), (11, 7), (6, 7), (2, 7)], [(11, 7), (16, 7), (22, 7)])
l.write("levels/11-4.txt")
