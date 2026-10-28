"""Episode 12, homing rockets: slower than you, but they turn; lead them into walls."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 12:0 incoming: a launcher over a long floor; posts to put between you and the rocket
l = Level("incoming")
l.fill(14, 3, 18, 3)
l.launcher(16, 4)
for x in (5, 10, 22, 27):
    l.fill(x, 15, x, 17)
l.fill(1, 13, 2, 13)
for y in (19, 17, 15):
    l.oneway(3, y)
l.put(16, 21, "S"); l.put(30, 21, "K"); l.put(1, 12, "X")
for x in (8, 13, 19, 24):
    l.gold(x, 21)
l.gold(3, 18)
l.waypoints([(19, 21), (24, 21), (30, 21)], [(24, 21), (19, 21), (13, 21), (8, 21), (3, 20), (3, 18), (3, 14), (1, 12)])
l.write("levels/12-0.txt")

# 12:1 dodgeball: a launcher in each top corner over a room of platforms
l = Level("dodgeball")
l.launcher(2, 2)
l.launcher(30, 2)
l.fill(1, 3, 3, 3)
l.fill(29, 3, 31, 3)
for x0, y in ((6, 18), (13, 16), (20, 18), (26, 15), (19, 12), (11, 12), (5, 9), (13, 7), (20, 7)):
    l.fill(x0, y, x0 + 3, y)
l.put(2, 21, "S"); l.put(21, 6, "K"); l.put(30, 21, "X")
for x, y in ((7, 17), (14, 15), (21, 17), (27, 14), (20, 11), (12, 11), (6, 8), (14, 6)):
    l.gold(x, y)
l.waypoints([(7, 17), (14, 15), (21, 17), (27, 14), (20, 11), (12, 11), (6, 8), (14, 6), (21, 6)], [(26, 14), (30, 21)])
l.write("levels/12-1.txt")

# 12:2 switchback: four lanes joined at alternate ends; a launcher behind you in each lane
l = Level("switchback")
for y, gap in ((6, (2, 3)), (11, (29, 30)), (16, (2, 3))):
    l.fill(1, y, 31, y)
    l.clear(gap[0], y, gap[1], y)
l.launcher(30, 2)
l.launcher(2, 8)
l.launcher(30, 13)
l.launcher(2, 18)
l.put(29, 5, "S"); l.put(30, 21, "K"); l.put(16, 21, "X")
for x in (20, 10):
    l.gold(x, 5)
    l.gold(x, 15)
for x in (12, 22):
    l.gold(x, 10)
    l.gold(x, 21)
l.waypoints([(20, 5), (10, 5), (2, 9), (12, 10), (22, 10), (30, 13), (20, 15), (10, 15), (2, 20), (12, 21), (22, 21), (30, 21)], [(22, 21), (16, 21)])
l.write("levels/12-2.txt")

# 12:3 crossroads: launchers at the ends of a cross; the corners are solid, the middle a climb
l = Level("crossroads")
l.fill(1, 1, 11, 7)
l.fill(21, 1, 31, 7)
l.fill(1, 15, 11, 21)
l.fill(21, 15, 31, 21)
steps = [(13, 18), (17, 16), (13, 14), (17, 12), (13, 10), (17, 8), (13, 6), (17, 4)]
for x0, y in steps:
    l.fill(x0, y, x0 + 1, y)
l.launcher(2, 11)
l.launcher(30, 11)
l.launcher(12, 1)
l.put(15, 21, "S"); l.put(18, 3, "K"); l.put(2, 14, "X")
for x0, y in steps[:-1]:
    l.gold(x0 + (1 if x0 == 13 else 0), y - 1)
l.gold(8, 14); l.gold(24, 14)
l.waypoints([(14, 17), (17, 15), (24, 14), (17, 15), (14, 13), (17, 11), (14, 9), (17, 7), (14, 5), (18, 3)], [(14, 9), (13, 13), (8, 14), (2, 14)])
l.write("levels/12-3.txt")

# 12:4 barrage: a rocket chasing you over the mines to the key, a ladder, a second launcher on
# top, blocks to hide behind, and the switch behind the locked door
l = Level("barrage")
l.fill(1, 12, 27, 12)
for x in (8, 9, 15, 16, 21, 22):
    l.mine_floor(x, 21)
l.launcher(2, 14)
for y in (19, 17, 15, 13):
    for x in range(28, 32):
        l.oneway(x, y)
l.launcher(30, 2)
for x0 in (12, 20):
    l.fill(x0, 8, x0 + 1, 9)
for x in (16, 17):
    l.mine_floor(x, 11)
l.fill(5, 1, 5, 10)
l.door(26, 21, 4, 11, locked=1)
l.put(4, 21, "S"); l.put(2, 11, "K"); l.put(8, 11, "X")
for x, y in ((12, 21), (19, 21), (24, 21), (29, 16), (24, 11), (19, 11), (14, 11), (9, 11)):
    l.gold(x, y)
l.waypoints([(12, 21), (19, 21), (24, 21), (26, 21), (29, 18), (29, 16), (29, 12), (24, 11), (19, 11), (14, 11), (9, 11), (2, 11)], [(8, 11)])
l.write("levels/12-4.txt")
