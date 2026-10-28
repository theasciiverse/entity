"""Episode 13, chaingun drones: a burst of bullets in a spread after each charge."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

def gun(l, x, y, movement=0, d=0):
    l.drone(x, y, movement=movement, weapon=2, d=d)

# 13:0 suppressing fire: a chaingun circling a block high in the room; posts to duck behind
l = Level("suppressing fire")
l.fill(14, 5, 18, 7)
gun(l, 13, 4)
for x0 in (6, 11, 20, 25):
    l.fill(x0, 15, x0 + 1, 16)
l.fill(1, 10, 2, 10)
l.fill(30, 10, 31, 10)
for x in (3, 29):
    for y in (19, 17, 15, 13, 11):
        l.oneway(x, y)
l.put(16, 21, "S"); l.put(30, 9, "K"); l.put(1, 9, "X")
for x, y in ((21, 21), (26, 21), (7, 14), (12, 14), (21, 14), (26, 14)):
    l.gold(x, y)
l.waypoints([(21, 21), (26, 21), (29, 20), (29, 14), (29, 10), (30, 9)], [(29, 14), (26, 14), (21, 14), (12, 14), (7, 14), (3, 14), (3, 10), (1, 9)])
l.write("levels/13-0.txt")

# 13:1 gun deck: a chaingun runs the ceiling over a floor of crates and mines
l = Level("gun deck")
for x0 in (5, 11, 17, 23):
    l.fill(x0, 20, x0 + 2, 21)
for x in (8, 9, 14, 15, 20, 21, 26, 27):
    l.mine_floor(x, 21)
gun(l, 16, 1, movement=2)
l.fill(1, 14, 3, 14)
for y in (19, 17, 15):
    l.oneway(4, y)
l.put(2, 21, "S"); l.put(30, 21, "K"); l.put(2, 13, "X")
for x0 in (5, 11, 17, 23):
    l.gold(x0 + 1, 19)
l.gold(4, 16)
l.waypoints([(6, 19), (12, 19), (18, 19), (24, 19), (30, 21)], [(24, 19), (18, 19), (12, 19), (6, 19), (4, 18), (4, 16), (4, 14), (2, 13)])
l.write("levels/13-1.txt")

# 13:2 pillbox: a chaingun inside a box with slits; go round it, never long in front of a slit
l = Level("pillbox")
l.fill(12, 8, 20, 14)
l.clear(13, 9, 19, 13)
for x, y in ((12, 11), (20, 11), (16, 8), (16, 14)):
    l.clear(x, y, x, y)
gun(l, 14, 12, movement=2)
for x0, y in ((3, 18), (7, 15), (3, 12), (7, 9), (11, 6), (17, 4), (23, 6), (26, 9), (29, 12), (26, 15), (29, 18)):
    l.fill(x0, y, x0 + 2, y)
l.put(2, 21, "S"); l.put(18, 3, "K"); l.put(30, 21, "X")
for x0, y in ((7, 15), (7, 9), (23, 6), (29, 12), (26, 15)):
    l.gold(x0 + 1, y - 1)
l.waypoints([(4, 17), (8, 14), (4, 11), (8, 8), (12, 5), (18, 3)], [(24, 5), (27, 8), (30, 11), (27, 14), (30, 17), (30, 21)])
l.write("levels/13-2.txt")

# 13:3 firing range: three lanes joined at alternate ends, a chaingun at the far end of each,
# crates to duck behind
l = Level("firing range")
for y, gap in ((8, (29, 30)), (15, (2, 3))):
    l.fill(1, y, 31, y)
    l.clear(gap[0], y, gap[1], y)
for x in (8, 15, 22):
    l.fill(x, 20, x, 21)
    l.fill(x + 2, 13, x + 2, 14)
    l.fill(x, 6, x, 7)
gun(l, 30, 19, movement=2)
gun(l, 2, 12, movement=2)
gun(l, 30, 5, movement=2)
for y in (13, 11, 9):
    for x in (29, 30):
        l.oneway(x, y)
for y in (20, 18, 16):
    for x in (2, 3):
        l.oneway(x, y)
l.put(5, 21, "S"); l.put(16, 7, "K"); l.put(4, 7, "X")
for x in (12, 19, 26):
    l.gold(x, 21)
for x in (6, 13, 20):
    l.gold(x, 14)
l.waypoints([(12, 21), (19, 21), (26, 21), (19, 21), (12, 21), (3, 19), (3, 17), (3, 14), (6, 14), (13, 14), (20, 14), (29, 14), (29, 12), (29, 8), (24, 7), (16, 7)], [(10, 7), (4, 7)])
l.write("levels/13-3.txt")

# 13:4 armory: a chaingun over the crates to the key, a ladder, a laser and a zap drone on top,
# the switch behind the locked door
l = Level("armory")
l.fill(1, 12, 27, 12)
for x in (8, 14, 20):
    l.fill(x, 20, x + 1, 21)
gun(l, 16, 13, movement=2)
for y in (19, 17, 15, 13):
    for x in range(28, 32):
        l.oneway(x, y)
l.fill(14, 4, 16, 5)
l.drone(13, 3, movement=0, weapon=1)
l.fill(21, 8, 22, 9)
l.drone(20, 7, movement=1)
l.fill(5, 1, 5, 10)
l.door(26, 21, 4, 11, locked=1)
l.put(3, 21, "S"); l.put(2, 11, "K"); l.put(8, 11, "X")
for x, y in ((11, 21), (17, 21), (23, 21), (29, 16), (25, 11), (18, 11), (12, 11)):
    l.gold(x, y)
l.waypoints([(11, 21), (17, 21), (23, 21), (26, 21), (29, 18), (29, 16), (29, 12), (25, 11), (18, 11), (12, 11), (8, 11), (2, 11)], [(8, 11)])
l.write("levels/13-4.txt")
