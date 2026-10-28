"""Episode 8, zap drones: followers, wanderers and chasers."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 8:0 patrol: a drone circling each block; take the gold off the tops between their laps
l = Level("patrol")
blocks = [(5, 16, 8, 18), (13, 12, 16, 14), (21, 8, 24, 10)]
for i, (x0, y0, x1, y1) in enumerate(blocks):
    l.fill(x0, y0, x1, y1)
    l.drone(x0 - 1, y1, movement=i % 2)
    l.gold((x0 + x1) // 2, y0 - 1)
l.fill(27, 5, 31, 5)
l.put(2, 21, "S"); l.put(30, 4, "K"); l.put(18, 21, "X")
l.gold(11, 20); l.gold(26, 6); l.gold(19, 9); l.gold(20, 15)
l.waypoints([(6, 15), (11, 20), (14, 11), (19, 9), (22, 7), (26, 6), (30, 4)], [(26, 7), (20, 15), (18, 21)])
l.write("levels/08-0.txt")

# 8:1 rush hour: three corridors, each with drones running end to end; jump them as they come
l = Level("rush hour")
l.fill(1, 14, 28, 14)
l.fill(3, 7, 28, 7)                    # the top corridor's floor, open at the far left
for y in (12, 10, 8):
    for x in (1, 2):
        l.oneway(x, y)
for y in (19, 16, 12, 9):
    for x in (30, 31):
        l.oneway(x, y)
l.drone(12, 21, movement=2, d=2)
l.drone(22, 21, movement=2, d=0)
l.drone(8, 13, movement=3, d=0)
l.drone(18, 13, movement=3, d=2)
l.drone(14, 6, movement=2, d=0)
l.put(2, 21, "S"); l.put(29, 21, "K"); l.put(27, 6, "X")
for x in (8, 16, 24):
    l.gold(x, 19)
    l.gold(x - 2, 11)
    l.gold(x + 1, 4)
l.waypoints([(8, 19), (16, 19), (24, 19), (29, 21)], [(30, 15), (29, 13), (22, 11), (14, 11), (6, 11), (3, 13), (2, 11), (1, 7), (4, 6), (9, 4), (17, 4), (25, 4), (27, 6)])
l.write("levels/08-1.txt")

# 8:2 hunted: chasers circle the pillars and charge down any clear line to you; use the cover
l = Level("hunted")
pillars = [(x, y) for x in (6, 14, 22) for y in (6, 13)]
for x, y in pillars:
    l.fill(x, y, x + 2, y + 2)
l.drone(5, 5, movement=0, chaser=1)
l.drone(25, 12, movement=1, chaser=1)
l.drone(13, 16, movement=0, chaser=1)
l.fill(28, 4, 31, 4)
l.put(15, 21, "S"); l.put(30, 3, "K"); l.put(2, 21, "X")
for x, y in pillars:
    l.gold(x + 1, y - 1)
l.waypoints([(19, 21), (23, 12), (23, 5), (27, 3), (30, 3)], [(23, 5), (15, 5), (15, 12), (7, 12), (7, 5), (3, 10), (2, 21)])
l.write("levels/08-2.txt")

# 8:3 carousel: two drones run round the walls of the room while you climb the tower in the middle
l = Level("carousel")
for i, y in enumerate((18, 15, 12, 9, 6)):
    x0 = 12 if i % 2 == 0 else 17
    l.fill(x0, y, x0 + 3, y)
l.drone(1, 21, movement=1, d=0)
l.drone(31, 1, movement=1, d=2)
l.put(16, 21, "S"); l.put(14, 5, "K"); l.put(2, 21, "X")
for i, y in enumerate((17, 14, 11, 8)):
    l.gold(14 if i % 2 == 0 else 19, y)
l.gold(9, 21); l.gold(24, 21); l.gold(10, 17)
l.waypoints([(24, 21), (14, 17), (19, 14), (14, 11), (19, 8), (14, 5)], [(10, 17), (9, 21), (2, 21)])
l.write("levels/08-3.txt")

# 8:4 swarm: wanderers below, chasers round two pillars, followers round the blocks on top,
# and the switch behind a locked door at the far end
l = Level("swarm")
l.fill(1, 17, 27, 17)                  # roof of the bottom corridor
for y in (19, 15):
    for x in (29, 30, 31):
        l.oneway(x, y)
l.drone(10, 21, movement=2, d=0)
l.drone(20, 21, movement=2, d=2)
for x0 in (8, 20):                     # the middle: two pillars, a chaser round each
    l.fill(x0, 12, x0 + 2, 13)
    l.drone(x0 - 1, 11, movement=0, chaser=1)
l.fill(4, 8, 31, 8)                    # floor of the top
for y in (15, 13, 11, 9):
    for x in (1, 2, 3):
        l.oneway(x, y)
for x0 in (11, 19):
    l.fill(x0, 4, x0 + 3, 5)
    l.drone(x0 - 1, 6, movement=1)
l.fill(28, 1, 28, 6)
l.door(15, 16, 27, 7, locked=1)
l.put(2, 21, "S"); l.put(30, 7, "K"); l.put(6, 7, "X")
for x, y in ((6, 20), (15, 20), (24, 20), (30, 13), (24, 16), (6, 16), (13, 3), (21, 3), (25, 7)):
    l.gold(x, y)
l.waypoints([(6, 20), (15, 20), (24, 20), (29, 18), (30, 13), (24, 16), (15, 16), (6, 16), (2, 14), (2, 10), (4, 7), (13, 3), (17, 7), (21, 3), (25, 7), (30, 7)], [(25, 7), (17, 7), (6, 7)])
l.write("levels/08-4.txt")
