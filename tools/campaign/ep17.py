"""Episode 17, mixes III: slopes, mazes and long flights with the hazards around them."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

def ladder(l, xs, ys):
    for y in ys:
        for x in xs:
            l.oneway(x, y)

def valley(l, x0, top=14):
    """A V of 45 degree slopes, x0 to x0 + 11, from row `top` down six rows and back up."""
    for i in range(6):
        l.put(x0 + i, top + i, "2")
        l.fill(x0 + i, top + i + 1, x0 + i, 21)
        l.put(x0 + 11 - i, top + i, "3")
        l.fill(x0 + 11 - i, top + i + 1, x0 + 11 - i, 21)

# 17:0 valleys: two deep Vs with mines in their bottoms, a laser circling overhead
l = Level("valleys")
l.fill(1, 14, 2, 21)
valley(l, 3)
l.fill(15, 14, 16, 21)
valley(l, 17)
l.fill(29, 14, 31, 21)
for x0 in (3, 17):
    l.mine(x0 + 5, 18)
    l.mine(x0 + 6, 18)
l.fill(14, 4, 17, 5)
l.drone(13, 3, movement=0, weapon=1)
l.put(1, 13, "S"); l.put(30, 13, "K"); l.put(2, 13, "X")
for x in (6, 11, 20, 25):
    l.gold(x, 12)
l.gold(16, 11)
l.waypoints([(6, 12), (11, 12), (16, 12), (20, 12), (25, 12), (30, 13)], [(25, 12), (20, 12), (16, 11), (11, 12), (6, 12), (2, 13)])
l.write("levels/17-0.txt")

# 17:1 rat run: three lanes cut into rooms by trek doors, a chaser in each room
l = Level("rat run")
for y, gap in ((8, (29, 30)), (15, (2, 3))):
    l.fill(1, y, 31, y)
    l.clear(gap[0], y, gap[1], y)
for lane_floor in (21, 14, 7):
    top = lane_floor - 5
    for w in (11, 21):
        l.fill(w, top, w, lane_floor - 1)
        l.door(0, 0, w - 1, lane_floor)
l.drone(16, 21, movement=0, chaser=1)
l.drone(6, 14, movement=1, chaser=1)
l.drone(26, 14, movement=0, chaser=1)
l.drone(16, 7, movement=1, chaser=1)
ladder(l, (2, 3), (20, 18, 16))
ladder(l, (29, 30), (13, 11, 9))
l.put(25, 21, "S"); l.put(2, 7, "K"); l.put(28, 7, "X")
for x, y in ((16, 21), (6, 21), (8, 14), (16, 14), (24, 14), (24, 7), (16, 7), (8, 7)):
    l.gold(x, y)
l.waypoints([(16, 21), (6, 21), (3, 20), (3, 16), (8, 14), (16, 14), (24, 14), (29, 13), (29, 9), (24, 7), (16, 7), (8, 7), (2, 7)], [(8, 7), (16, 7), (24, 7), (28, 7)])
l.write("levels/17-1.txt")

# 17:2 sky bridge: a cannon and bounce blocks over a floor of mines under a turret, a lift back
l = Level("sky bridge")
for x in range(4, 27):
    l.mine_floor(x, 21)
l.fill(1, 16, 3, 16)
l.obj(f"launchpad {px(3)} {16 * 24} 0.7071 -0.7071")
for x, y in ((11, 15), (15, 14), (19, 15), (23, 14)):
    l.bounce(x, y)
l.fill(27, 16, 31, 16)
l.pad(28, 15)
for x0, x1, y in ((20, 25, 7), (12, 16, 6), (4, 8, 7)):
    l.fill(x0, y, x1, y)
l.turret(16, 1)
l.put(1, 15, "S"); l.put(30, 15, "K"); l.put(5, 6, "X")
for x, y in ((7, 12), (13, 12), (17, 12), (21, 12), (25, 13), (22, 6), (14, 5), (10, 4)):
    l.gold(x, y)
l.waypoints([(3, 15), (7, 12), (11, 14), (13, 12), (15, 13), (17, 12), (19, 14), (21, 12), (23, 13), (25, 13), (30, 15)], [(28, 15), (27, 9), (22, 6), (18, 5), (14, 5), (10, 4), (5, 6)])
l.write("levels/17-2.txt")

# 17:3 tower: wall jump up a chimney between windows, a launcher outside each wall
l = Level("tower")
l.fill(14, 4, 14, 21)
l.fill(17, 4, 17, 21)
for y in (8, 14):
    l.clear(14, y, 14, y + 1)
    l.clear(17, y + 3, 17, y + 4)
l.launcher(3, 11)
l.launcher(29, 13)
l.fill(1, 12, 4, 12)
l.fill(28, 14, 31, 14)
l.fill(12, 3, 19, 3)
l.clear(15, 3, 16, 3)
l.fill(11, 10, 13, 10)
l.fill(11, 16, 13, 16)
l.put(15, 21, "S"); l.put(19, 2, "K"); l.put(2, 21, "X")
for y in (18, 13, 8):
    l.gold(15, y)
    l.gold(16, y - 2)
l.gold(12, 2); l.gold(8, 21)
l.waypoints([(15, 18), (16, 16), (15, 13), (16, 11), (15, 8), (16, 6), (16, 2), (19, 2)], [(12, 2), (12, 9), (12, 15), (8, 21), (2, 21)])
l.write("levels/17-3.txt")

# 17:4 stronghold: two keys, two locked doors, a thwomp, a laser and a launcher
l = Level("stronghold")
l.fill(1, 14, 27, 14)
l.fill(6, 20, 8, 20)
for x in (5, 9):
    l.mine_floor(x, 21)
l.fill(16, 15, 16, 20)
l.door(7, 19, 15, 21, locked=1)
l.thwomp(21, 15, 1)
ladder(l, (28, 29, 30, 31), (19, 17, 15))
l.fill(14, 6, 16, 7)
l.drone(13, 5, movement=0, weapon=1)
l.launcher(30, 2)
l.fill(5, 1, 5, 12)
l.door(26, 21, 4, 13, locked=1)
l.put(2, 21, "S"); l.put(2, 13, "K"); l.put(9, 13, "X")
for x, y in ((7, 17), (12, 21), (19, 21), (24, 21), (29, 16), (24, 13), (18, 13), (12, 13)):
    l.gold(x, y)
l.waypoints([(4, 21), (7, 19), (7, 17), (12, 21), (19, 21), (24, 21), (26, 21), (29, 20), (29, 16), (26, 13), (24, 13), (18, 13), (12, 13), (9, 13), (2, 13)], [(9, 13)])
l.write("levels/17-4.txt")
