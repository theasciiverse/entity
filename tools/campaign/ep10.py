"""Episode 10, laser drones: see, aim, charge, fire. Cover is everything."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

def laser(l, x, y, movement=0, d=0):
    l.drone(x, y, movement=movement, weapon=1, d=d)

# 10:0 searchlight: a laser circling a central block; low walls around the room to hide behind
l = Level("searchlight")
l.fill(13, 8, 19, 12)
laser(l, 12, 7)
for x in (5, 11, 21, 27):
    l.fill(x, 19, x, 21)
for x0, y in ((2, 14), (25, 14), (2, 6), (26, 6)):
    l.fill(x0, y, x0 + 3, y)
    l.fill(x0 + (3 if x0 < 16 else 0), y - 2, x0 + (3 if x0 < 16 else 0), y - 1)
l.put(16, 21, "S"); l.put(28, 5, "K"); l.put(3, 5, "X")
for x, y in ((8, 21), (24, 21), (3, 13), (28, 13), (16, 7)):
    l.gold(x, y)
l.waypoints([(24, 21), (28, 13), (28, 5)], [(16, 7), (8, 21), (3, 13), (3, 5)])
l.write("levels/10-0.txt")

# 10:1 blinds: a long room of pillars, a laser at the far end; dash from pillar to pillar
l = Level("blinds")
for x in (7, 12, 17, 22):
    l.fill(x, 15, x + 1, 17)
l.fill(26, 12, 31, 12)
laser(l, 29, 10, movement=2)
l.fill(1, 12, 4, 12)
for y in (19, 17, 15, 13):
    l.oneway(5, y)
l.put(2, 21, "S"); l.put(28, 21, "K"); l.put(2, 11, "X")
for x in (9, 14, 19, 24):
    l.gold(x, 21)
l.waypoints([(9, 21), (14, 21), (19, 21), (24, 21), (28, 21)], [(19, 21), (14, 21), (9, 21), (6, 20), (5, 16), (5, 12), (2, 11)])
l.write("levels/10-1.txt")

# 10:2 crossbeams: climb between two walls with windows; a laser beyond each wall looks in
l = Level("crossbeams")
for w in (9, 23):
    l.fill(w, 3, w, 21)
    l.clear(w, 15, w, 16)
    l.clear(w, 8, w, 9)
for x0, y in ((3, 12), (26, 12)):
    l.fill(x0, y, x0 + 3, y + 1)
laser(l, 2, 11, movement=0)
laser(l, 30, 14, movement=0)
rows = (19, 17, 15, 13, 11, 9, 7, 5)
for i, y in enumerate(rows):
    x0 = 10 if i % 2 == 0 else 19
    l.fill(x0, y, x0 + 3, y)
l.put(16, 21, "S"); l.put(16, 2, "K"); l.put(14, 21, "X")
for i, y in enumerate(rows):
    l.gold(11 if i % 2 == 0 else 21, y - 1)
l.waypoints([(11, 18), (21, 16), (11, 14), (21, 12), (11, 10), (21, 8), (11, 6), (21, 4), (16, 2)], [(16, 6), (16, 14), (14, 21)])
l.write("levels/10-2.txt")

# 10:3 shelter: a laser sweeps the open room; shelves over the floor give shade between mines
l = Level("shelter")
shelves = (4, 10, 16, 22)
for x0 in shelves:
    l.fill(x0, 17, x0 + 3, 17)
for x in (8, 9, 14, 15, 20, 21, 26, 27):
    l.mine_floor(x, 21)
laser(l, 16, 2, movement=2, d=0)
l.fill(1, 12, 2, 12)
for y in (19, 17, 15, 13):
    l.oneway(3, y)
l.put(2, 21, "S"); l.put(30, 21, "K"); l.put(1, 11, "X")
for x0 in shelves:
    l.gold(x0 + 2, 20)
l.gold(3, 16)
l.waypoints([(6, 20), (12, 20), (18, 20), (24, 20), (30, 21)], [(24, 20), (18, 20), (12, 20), (6, 20), (3, 20), (3, 16), (3, 12), (1, 11)])
l.write("levels/10-3.txt")

# 10:4 spotlight: a laser over a hall of cover to the key, a ladder, bounce blocks over mines
# under a second laser, and the switch behind the locked door
l = Level("spotlight")
l.fill(1, 12, 27, 12)
for x in (6, 14, 22):
    l.fill(x, 16, x + 1, 17)
for x in (10, 11, 18, 19):
    l.mine_floor(x, 21)
laser(l, 14, 13, movement=2)
for y in (19, 17, 15, 13):
    for x in range(28, 32):
        l.oneway(x, y)
for x in range(10, 22):
    l.mine_floor(x, 11)
for x in (12, 16, 20):
    l.bounce(x, 9)
l.fill(14, 3, 16, 4)
laser(l, 13, 2, movement=0)
l.fill(5, 1, 5, 10)
l.door(26, 21, 4, 11, locked=1)
l.put(2, 21, "S"); l.put(2, 11, "K"); l.put(7, 11, "X")
for x, y in ((4, 20), (12, 20), (20, 20), (29, 16), (24, 11), (14, 7), (18, 7), (8, 11)):
    l.gold(x, y)
l.waypoints([(4, 20), (12, 20), (20, 20), (26, 21), (29, 18), (29, 16), (29, 12), (24, 11), (20, 8), (18, 7), (16, 8), (14, 7), (12, 8), (8, 11), (2, 11)], [(7, 11)])
l.write("levels/10-4.txt")
