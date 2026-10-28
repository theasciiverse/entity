"""Episode 6, mines: the same mines, with far less room between them."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 6:0 eggshells: a floor of mines with small islands to land on
l = Level("eggshells")
for x in range(5, 29):
    if x % 4 in (1, 2):
        l.mine_floor(x, 21)
l.fill(29, 18, 31, 21)
l.put(3, 21, "S"); l.put(30, 17, "K"); l.put(1, 18, "X")
l.fill(1, 19, 2, 19)
for x in (7, 11, 15, 19, 23, 27):
    l.gold(x, 19)
islands = [(3, 21), (8, 21), (12, 21), (16, 21), (20, 21), (24, 21)]
l.waypoints([(7, 19)] + [p for i, p in enumerate(islands[1:]) for p in ((p), (11 + 4 * i, 19))][:-1] + [(27, 19), (30, 17)], list(reversed(islands)) + [(1, 18)])
l.write("levels/06-0.txt")

# 6:1 mine shaft: a wide shaft with mines on its walls; climb on the clean stretches
l = Level("mine shaft")
l.fill(1, 1, 11, 21)
l.fill(16, 1, 31, 21)
l.clear(12, 1, 15, 21)
l.clear(16, 1, 31, 3)                  # a gallery at the top
l.clear(20, 1, 30, 5)
for y in (18, 17, 11, 10):
    l.mine(12, y)
for y in (14, 13, 7, 6):
    l.mine(15, y)
l.put(13, 21, "S"); l.put(29, 5, "K"); l.put(18, 3, "X")
gold = [(14, 19), (13, 15), (14, 12), (13, 8), (14, 4)]
for x, y in gold:
    l.gold(x, y)
l.waypoints(gold + [(24, 5), (29, 5)], [(24, 5), (18, 3)])
l.write("levels/06-1.txt")

# 6:2 constellation: mines hanging in the sky over the gaps between a rising row of platforms
l = Level("constellation")
steps = [(1, 18), (7, 16), (13, 14), (19, 12), (25, 10)]
for x0, y in steps:
    l.fill(x0, y, x0 + 3, y)
for (x0, y) in steps[:-1]:
    l.mine(x0 + 5, y - 4)
    l.mine(x0 + 4, y + 2)
for x in range(4, 31, 2):
    l.mine_floor(x, 21)
l.put(1, 17, "S"); l.put(28, 9, "K"); l.put(4, 17, "X")
for x0, y in steps[:-1]:
    l.gold(x0 + 5, y - 2)
l.waypoints([(6, 16), (12, 14), (18, 12), (24, 10), (28, 9)], [(22, 9), (18, 11), (16, 13), (10, 15), (4, 17)])
l.write("levels/06-2.txt")

# 6:3 fuse: two corridors whose mines rise and fall, so every jump has its own height
l = Level("fuse")
l.fill(1, 12, 29, 12)                  # floor of the upper corridor
for x in range(4, 29):
    if x % 3 == 1:
        l.mine_floor(x, 21)
    if x % 3 == 2:
        l.mine_floor(x, 11)
for x, h in ((6, 17), (9, 18), (12, 16), (15, 18), (18, 17), (21, 16), (24, 18), (27, 17)):
    l.mine(x, h)
    l.mine(x - 1, h - 10)
for y in (19, 17, 15, 13):
    for x in (30, 31):
        l.oneway(x, y)
l.put(2, 21, "S"); l.put(29, 21, "K"); l.put(2, 11, "X")
for x in (8, 14, 20, 26):
    l.gold(x, 20)
    l.gold(x - 1, 10)
l.waypoints([(8, 20), (14, 20), (20, 20), (26, 20), (29, 21)], [(30, 14), (29, 11), (25, 10), (19, 10), (13, 10), (7, 10), (2, 11)])
l.write("levels/06-3.txt")

# 6:4 minesweeper: islands under a mined ceiling to a key, up a ladder past mined walls, back
# over the top through floating mines to the switch behind the locked door
l = Level("minesweeper")
l.fill(1, 13, 27, 17)                  # the block between the corridor and the top
for x in range(6, 25):
    if x % 4 in (2, 3):
        l.mine_floor(x, 21)
for x in range(5, 26, 4):
    l.mine_ceiling(x, 18)
for y in (19, 17, 15, 13):
    for x in (28, 29):
        l.oneway(x, y)
for y in (19, 16, 15):
    l.mine(31, y)
l.fill(1, 8, 5, 8)                     # the switch's room
l.fill(5, 9, 5, 11)
l.door(26, 21, 4, 12, locked=1)
for x, y in ((10, 10), (14, 11), (18, 9), (21, 11), (13, 6), (17, 5)):
    l.mine(x, y)
l.put(2, 21, "S"); l.put(2, 12, "K"); l.put(24, 12, "X")
for x, y in ((4, 20), (8, 20), (12, 20), (16, 20), (20, 20), (24, 20), (30, 14), (16, 10), (8, 12), (12, 8)):
    l.gold(x, y)
l.waypoints([(4, 20), (8, 20), (12, 20), (16, 20), (20, 20), (24, 20), (26, 21), (28, 18), (29, 14), (30, 14), (27, 12), (16, 10), (12, 8), (8, 12), (2, 12)], [(8, 12), (16, 12), (24, 12)])
l.write("levels/06-4.txt")
