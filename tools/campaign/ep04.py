"""Episode 4, fine control: the movement skills again, with less room for error."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level

# 4:0 long jump: blocks over pits of mines, the gaps widening; gold at the top of each jump
l = Level("long jump")
for x0, x1 in [(1, 5), (10, 12), (18, 20), (27, 31)]:
    l.fill(x0, 18, x1, 21)
for x in list(range(6, 10)) + list(range(13, 18)) + list(range(21, 27)):
    l.mine_floor(x, 21)
l.put(2, 17, "S"); l.put(30, 17, "K"); l.put(4, 17, "X")
for x, y in [(8, 14), (15, 14), (24, 14)]:
    l.gold(x, y)
l.waypoints([(8, 14), (11, 17), (15, 14), (19, 17), (24, 14), (30, 17)], [(19, 17), (11, 17), (4, 17)])
l.write("levels/04-0.txt")

# 4:1 slow descent: climb a shaft, then a drop far too deep to survive without sliding a wall
l = Level("slow descent")
l.fill(4, 4, 4, 21)            # the climb: between the frame and this wall
l.fill(7, 4, 31, 21)           # the block the drop runs beside
l.put(2, 21, "S"); l.put(29, 3, "K"); l.put(5, 21, "X")
for y in (17, 12, 7):
    l.gold(2, y)
for x in (12, 18, 24):
    l.gold(x, 2)
l.waypoints([(2, 17), (2, 12), (2, 7), (3, 2), (12, 2), (18, 2), (24, 2), (29, 3)], [(6, 2), (5, 21)])
l.write("levels/04-1.txt")

# 4:2 ski jump: down a long 45 degree slope, up a ramp and over a pit of mines
l = Level("ski jump")
l.fill(1, 6, 3, 21)                    # the start, high on the left
for i in range(13):                    # slope from (4, 6) down to (16, 18)
    l.put(4 + i, 6 + i, "2")
    l.fill(4 + i, 7 + i, 4 + i, 21)
l.fill(17, 19, 19, 21)                 # the run out
for i in range(3):                     # ramp up from (20, 18) to (22, 16)
    l.put(20 + i, 18 - i, "3")
    l.fill(20 + i, 19 - i, 20 + i, 21)
for x in range(23, 29):
    l.mine_floor(x, 21)
l.fill(29, 15, 31, 21)                 # landing
l.put(2, 5, "S"); l.put(30, 14, "K"); l.put(18, 18, "X")
for x, y in [(10, 9), (24, 12), (27, 12)]:
    l.gold(x, y)
l.waypoints([(10, 9), (18, 18), (24, 12), (27, 12), (30, 14)], [(26, 12), (21, 16), (18, 18)])
l.write("levels/04-2.txt")

# 4:3 low ceiling: corridors with mines overhead, pits in the floor; only short hops fit
l = Level("low ceiling")
l.fill(3, 6, 31, 9)                    # between the top and middle corridors
l.clear(1, 2, 2, 13)                   # the shaft on the left
l.fill(1, 14, 29, 16)                  # between the corridors
l.fill(1, 21, 31, 21)                  # lower floor
l.clear(30, 10, 31, 20)                # the shaft joining them
pits_low, pits_up = (6, 12, 18, 24), (8, 15, 22)
for p in pits_low:
    l.clear(p, 21, p + 1, 21)
    for x in (p, p + 1):
        l.mine_floor(x, 21)
for p in pits_up:
    l.clear(p, 14, p + 1, 14)
    for x in (p, p + 1):
        l.mine_floor(x, 14)
pits_top = (10, 18, 25)
for p in pits_top:
    l.clear(p, 6, p + 1, 6)
    for x in (p, p + 1):
        l.mine_floor(x, 6)
for x in range(4, 30, 3):
    l.mine_ceiling(x, 17)
    l.mine_ceiling(x + 1, 10)
    l.mine_ceiling(x + 2, 2)
for y in (12, 10, 8, 6):
    for x in (1, 2):
        l.oneway(x, y)
for y in (19, 17, 15, 13):
    for x in (30, 31):
        l.oneway(x, y)
l.put(2, 20, "S"); l.put(28, 20, "K"); l.put(30, 5, "X")
for x in (9, 15, 21):
    l.gold(x, 19)
for x in (12, 19, 26):
    l.gold(x, 12)
for x in (14, 22):
    l.gold(x, 4)
l.waypoints([(9, 19), (15, 19), (21, 19), (28, 20)], [(30, 18), (31, 12), (26, 13), (19, 12), (12, 12), (4, 13), (2, 11), (1, 5), (14, 4), (22, 4), (30, 5)])
l.write("levels/04-3.txt")

# 4:4 obstacle course: down the slope, over the pit, under the mines, up the chimney and back
l = Level("obstacle course")
l.fill(1, 5, 4, 21)                    # the start
for i in range(7):                     # slope from (5, 5) down to (11, 11)
    l.put(5 + i, 5 + i, "2")
    l.fill(5 + i, 6 + i, 5 + i, 21)
l.fill(12, 12, 14, 21)
for x in range(15, 20):
    l.mine_floor(x, 21)
l.fill(20, 12, 27, 21)                 # under the low ceiling
l.fill(20, 6, 27, 7)                   # the low ceiling
l.clear(23, 12, 24, 12)
for x in (23, 24):
    l.mine_floor(x, 12)
for x in (20, 22, 25, 27):
    l.mine_ceiling(x, 8)
l.fill(28, 3, 28, 10)                  # the chimney's left wall
l.fill(28, 12, 31, 21)                 # its floor
l.fill(12, 6, 14, 6)                   # a stepping stone on the way back
l.put(2, 4, "S"); l.put(28, 2, "K"); l.put(4, 4, "X")
for x, y in [(9, 7), (17, 9), (30, 7), (17, 3), (8, 2)]:
    l.gold(x, y)
l.waypoints([(9, 7), (13, 11), (17, 9), (21, 11), (26, 11), (30, 11), (30, 7), (30, 3), (28, 2)], [(24, 5), (17, 3), (13, 5), (8, 2), (4, 4)])
l.write("levels/04-4.txt")
