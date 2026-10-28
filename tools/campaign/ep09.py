"""Episode 9, thwomps and floor guards."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 9:0 crush hour: thwomps over a lane with mines to hop, a guard on the roof on the way back
l = Level("crush hour")
l.fill(1, 12, 27, 12)
for x in (6, 10, 14, 18, 22):
    l.thwomp(x, 13, 1)
for x in (8, 12, 16, 20, 24):
    l.mine_floor(x, 21)
for y in (19, 17, 15, 13):
    for x in range(28, 32):
        l.oneway(x, y)
l.floorguard(14, 11, -1)
l.put(2, 21, "S"); l.put(26, 21, "K"); l.put(2, 11, "X")
for x in (6, 10, 14, 18, 22):
    l.gold(x, 20)
for x, y in ((20, 9), (8, 9), (29, 16), (24, 11)):
    l.gold(x, y)
l.waypoints([(6, 20), (10, 20), (14, 20), (18, 20), (22, 20), (26, 21)], [(29, 16), (29, 12), (24, 11), (20, 9), (14, 9), (8, 9), (2, 11)])
l.write("levels/09-0.txt")

# 9:1 piston: a shaft of ledges, each with a thwomp in the opposite wall waiting to slam across
l = Level("piston")
l.fill(12, 3, 12, 21)
l.fill(18, 3, 18, 20)
l.fill(19, 3, 31, 20)
ledges = [(19, 16), (17, 13), (15, 13), (13, 11)]
rows = (19, 17, 15, 13, 11, 9, 7, 5)
for i, y in enumerate(rows):
    if i % 2 == 0:
        l.fill(16, y, 17, y)
        l.thwomp(13, y - 1, 0)
    else:
        l.fill(13, y, 14, y)
        l.thwomp(17, y - 1, 2)
l.put(30, 21, "S"); l.put(15, 1, "K"); l.put(2, 21, "X")
for i, y in enumerate(rows):
    l.gold(17 if i % 2 == 0 else 13, y - 1)
l.gold(6, 1)
l.waypoints([(16, 21)] + [((17 if i % 2 == 0 else 13), y - 1) for i, y in enumerate(rows)] + [(15, 1)], [(6, 1), (10, 2), (11, 12), (2, 21)])
l.write("levels/09-1.txt")

# 9:2 guard duty: four floors, a guard on each, and a ladder at alternate ends
l = Level("guard duty")
floors = (17, 13, 9, 5)
for i, y in enumerate(floors):
    l.fill(1, y, 31, y)
    gap = (29, 30) if i % 2 == 0 else (2, 3)
    l.clear(gap[0], y, gap[1], y)
    for x in gap:
        l.oneway(x, y + 2)
    l.floorguard(16, y - 1, 1 if i % 2 else -1)
l.floorguard(12, 21, 1)
l.put(2, 21, "S"); l.put(16, 4, "K"); l.put(27, 4, "X")
for i, y in enumerate((20, 16, 12, 8, 4)):
    for x in (9, 23):
        l.gold(x, y)
l.waypoints([(9, 20), (23, 20), (29, 19), (29, 16), (23, 16), (9, 16), (3, 15), (3, 12), (9, 12), (23, 12), (29, 11), (29, 8), (23, 8), (9, 8), (3, 7), (3, 4), (9, 4), (16, 4)],
            [(23, 4), (27, 4)])
l.write("levels/09-2.txt")

# 9:3 stepping on heads: cross a pit of mines on the tops of the thwomps hanging over it
l = Level("stepping on heads")
l.fill(1, 11, 4, 21)
l.fill(27, 11, 31, 21)
for x in range(5, 27):
    l.mine_floor(x, 21)
for x in (8, 13, 18, 23):
    l.thwomp(x, 10, 1)
l.put(2, 10, "S"); l.put(29, 10, "K"); l.put(4, 10, "X")
for x in (8, 13, 18, 23):
    l.gold(x, 7)
l.gold(29, 5)
l.waypoints([(8, 9), (8, 7), (13, 9), (13, 7), (18, 9), (18, 7), (23, 9), (23, 7), (29, 10)], [(29, 5), (23, 9), (18, 9), (13, 9), (8, 9), (4, 10)])
l.write("levels/09-3.txt")

# 9:4 foundry: under the thwomps, past the side thwomps to the key, over the guard to the switch
l = Level("foundry")
l.fill(4, 17, 31, 17)                  # roof of the bottom corridor
for x in (10, 16, 22):
    l.thwomp(x, 18, 1)
for y in (19, 17):
    for x in (1, 2, 3):
        l.oneway(x, y)
l.fill(6, 15, 7, 15)
l.fill(9, 14, 23, 14)                  # the key's platform, swept by two side thwomps
l.thwomp(5, 13, 0)
l.thwomp(27, 13, 2)
for y in (14, 12, 10):
    for x in (29, 30, 31):
        l.oneway(x, y)
l.fill(1, 8, 27, 8)                    # floor of the top
l.floorguard(16, 7, 1)
l.fill(5, 1, 5, 6)
l.door(16, 13, 4, 7, locked=1)
l.put(30, 21, "S"); l.put(2, 7, "K"); l.put(26, 7, "X")
for x, y in ((19, 20), (13, 20), (7, 20), (2, 16), (12, 12), (20, 12), (30, 9), (21, 6), (11, 6)):
    l.gold(x, y)
l.waypoints([(19, 20), (13, 20), (7, 20), (2, 18), (2, 16), (6, 14), (12, 12), (16, 13), (20, 12), (26, 16), (29, 16), (30, 13), (30, 9), (26, 7), (21, 6), (11, 6), (2, 7)], [(11, 6), (21, 6), (26, 7)])
l.write("levels/09-4.txt")
