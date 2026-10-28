"""Episode 7, launch pads and bounce blocks."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 7:0 liftoff: pads lift you past ledges you steer onto, then steps back down to the door
l = Level("liftoff")
l.pad(6, 21)
l.fill(8, 12, 11, 12)
l.pad(10, 11)
l.fill(13, 4, 16, 4)
for x0, y in ((20, 7), (25, 11), (29, 16)):
    l.fill(x0, y, min(x0 + 3, 31), y)
for x in range(13, 29, 2):
    l.mine_floor(x, 21)
l.put(2, 21, "S"); l.put(15, 3, "K"); l.put(30, 15, "X")
for x, y in ((6, 15), (7, 12), (10, 6), (12, 2), (21, 5), (26, 9), (29, 13)):
    l.gold(x, y)
l.waypoints([(6, 15), (7, 12), (9, 11), (10, 6), (12, 2), (15, 3)], [(21, 5), (21, 6), (26, 9), (26, 10), (29, 13), (30, 15)])
l.write("levels/07-0.txt")

# 7:1 trampoline: bounce blocks over a floor of mines, then back over the top
l = Level("trampoline")
for x in range(4, 26):
    l.mine_floor(x, 21)
for x in (6, 10, 14, 18, 22):
    l.bounce(x, 18)
l.fill(26, 19, 31, 21)
l.pad(29, 18)
l.fill(20, 12, 27, 12)
for x in (16, 12, 8):
    l.bounce(x, 12)
l.fill(1, 12, 4, 12)
l.put(2, 21, "S"); l.put(30, 18, "K"); l.put(2, 11, "X")
for x in (8, 12, 16, 20, 24):
    l.gold(x, 16)
for x in (14, 10, 6):
    l.gold(x, 10)
l.waypoints([(6, 17), (8, 16), (10, 17), (12, 16), (14, 17), (16, 16), (18, 17), (20, 16), (22, 17), (24, 16), (30, 18)], [(29, 17), (28, 10), (24, 11), (16, 11), (14, 10), (12, 11), (10, 10), (8, 11), (6, 10), (2, 11)])
l.write("levels/07-1.txt")

# 7:2 cannon: diagonal pads on islands, each throwing you to the next across the mines
l = Level("cannon")
islands = (3, 11, 19)
for x in range(5, 27):
    if all(abs(x - i) > 1 for i in islands):
        l.mine_floor(x, 21)
for i in islands:
    l.obj(f"launchpad {px(i)} 528 0.7071 -0.7071")
l.fill(27, 18, 31, 21)
l.pad(30, 17)
l.fill(18, 7, 26, 7)
l.fill(1, 9, 12, 9)
l.put(1, 21, "S"); l.put(28, 17, "K"); l.put(2, 8, "X")
for x, y in ((7, 17), (15, 17), (23, 17), (29, 9), (22, 5), (15, 7), (8, 7)):
    l.gold(x, y)
l.waypoints([(3, 21), (7, 17), (11, 21), (15, 17), (19, 21), (23, 17), (28, 17)], [(30, 17), (29, 9), (25, 6), (22, 5), (18, 6), (15, 7), (11, 8), (8, 7), (2, 8)])
l.write("levels/07-2.txt")

# 7:3 updraft: up pads from ledge to ledge, mines overhead where you must steer clear
l = Level("updraft")
ledges = [(6, 17, 9), (12, 13, 16), (20, 9, 23), (26, 5, 29)]
l.pad(3, 21)
for x0, y, x1 in ledges:
    l.fill(x0, y, x1, y)
    if x1 < 29:
        l.pad(x1 - 1, y - 1)
for x, y in ((3, 8), (8, 4), (15, 1), (23, 1)):
    l.mine(x, y)
for x in range(8, 31, 2):
    l.mine_floor(x, 21)
l.put(1, 21, "S"); l.put(28, 4, "K"); l.put(1, 12, "X")
l.fill(1, 13, 3, 13)
for x, y in ((3, 14), (7, 12), (12, 8), (18, 5), (24, 2), (8, 16), (15, 12), (22, 8)):
    l.gold(x, y)
l.waypoints([(3, 21), (3, 14), (7, 16), (8, 16), (7, 12), (12, 12), (15, 12), (12, 8), (18, 8), (22, 8), (18, 5), (24, 4), (24, 2), (28, 4)], [(20, 8), (14, 12), (8, 16), (2, 12)])
l.write("levels/07-3.txt")

# 7:4 fairground: bounce blocks to a cannon, the key, a lift, floating blocks to the switch room
l = Level("fairground")
for x in range(4, 20):
    if x not in (14, 15):
        l.mine_floor(x, 21)
for x in (6, 10):
    l.bounce(x, 18)
l.obj(f"launchpad {px(14)} 528 0.7071 -0.7071")
l.pad(28, 21)
l.fill(21, 12, 26, 12)
for x, y in ((18, 12), (14, 11), (10, 10)):
    l.bounce(x, y)
l.fill(7, 8, 9, 8)
l.fill(1, 6, 5, 6)
l.fill(6, 1, 6, 4)
l.door(24, 21, 5, 5, locked=1)
l.put(2, 21, "S"); l.put(2, 5, "K"); l.put(25, 11, "X")
for x, y in ((8, 16), (12, 16), (18, 17), (28, 14), (22, 10), (16, 9), (12, 8), (8, 6)):
    l.gold(x, y)
l.waypoints([(6, 17), (8, 16), (10, 17), (12, 16), (14, 21), (18, 17), (24, 21), (28, 21), (28, 14), (24, 11), (22, 10), (18, 11), (16, 9), (14, 10), (12, 8), (10, 9), (8, 7), (8, 6), (2, 5)], [(8, 7), (10, 9), (14, 10), (18, 11), (25, 11)])
l.write("levels/07-4.txt")
