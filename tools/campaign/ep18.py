"""Episode 18, mixes IV: the hardest combinations before the finale."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

def ladder(l, xs, ys):
    for y in ys:
        for x in xs:
            l.oneway(x, y)

# 18:0 minefield run: islands between mines under a ceiling of drones, a turret over the roof
l = Level("minefield run")
l.fill(1, 15, 27, 15)
for x in range(5, 27):
    if x % 4 in (1, 2):
        l.mine_floor(x, 21)
l.drone(8, 16, movement=2, d=0)
l.drone(20, 16, movement=2, d=2)
ladder(l, (28, 29, 30, 31), (19, 17))
l.turret(16, 2)
l.fill(15, 1, 17, 1)
for x0 in (8, 19):
    l.fill(x0, 10, x0 + 1, 11)
l.put(2, 21, "S"); l.put(30, 21, "K"); l.put(2, 14, "X")
for x in (7, 11, 15, 19, 23):
    l.gold(x, 20)
for x in (24, 14, 6):
    l.gold(x, 14)
l.waypoints([(3, 21), (7, 20), (8, 21), (11, 20), (12, 21), (15, 20), (16, 21), (19, 20), (20, 21), (23, 20), (24, 21), (30, 21)], [(29, 18), (29, 16), (26, 14), (24, 14), (14, 14), (6, 14), (2, 14)])
l.write("levels/18-0.txt")

# 18:1 freefall: down a mined shaft past a laser's window to the switch, then two pads back up
l = Level("freefall")
l.fill(1, 2, 31, 2)
for x0 in (8, 22):
    l.clear(x0, 2, x0 + 2, 2)
for w in (7, 11, 21, 25):
    l.fill(w, 3, w, 17)
l.clear(7, 11, 7, 12)
l.clear(25, 6, 25, 7)
for x, y in ((8, 6), (10, 9), (8, 13), (10, 16)):
    l.mine(x, y)
l.fill(2, 8, 4, 10)
l.drone(1, 7, movement=0, weapon=1)
l.fill(28, 9, 30, 11)
l.drone(27, 8, movement=1, weapon=1)
l.pad(22, 21)
l.fill(23, 12, 24, 12)
l.pad(24, 11)
l.put(4, 1, "S"); l.put(16, 21, "K"); l.put(29, 1, "X")
for x, y in ((9, 4), (9, 11), (9, 18), (13, 21), (19, 21), (22, 15), (24, 6), (27, 1)):
    l.gold(x, y)
l.waypoints([(9, 1), (9, 4), (9, 11), (9, 18), (13, 21), (16, 21)], [(19, 21), (22, 21), (22, 15), (23, 11), (24, 11), (24, 6), (25, 1), (27, 1), (29, 1)])
l.write("levels/18-1.txt")

# 18:2 clockwork heads: across the thwomps' heads while a drone runs the row above them
l = Level("clockwork heads")
l.fill(1, 11, 4, 21)
l.fill(27, 11, 31, 21)
for x in range(5, 27):
    l.mine_floor(x, 21)
for x in (8, 13, 18, 23):
    l.thwomp(x, 10, 1)
l.drone(6, 6, movement=2, d=0)
l.put(2, 10, "S"); l.put(29, 10, "K"); l.put(4, 10, "X")
for x in (8, 13, 18, 23):
    l.gold(x, 8)
l.gold(29, 5)
l.waypoints([(8, 9), (8, 8), (13, 9), (13, 8), (18, 9), (18, 8), (23, 9), (23, 8), (29, 10)], [(29, 5), (23, 9), (18, 9), (13, 9), (8, 9), (4, 10)])
l.write("levels/18-2.txt")

# 18:3 lockdown: three lanes; a trap door shuts behind you in each and a rocket chases you on
l = Level("lockdown")
for y, gap in ((8, (29, 30)), (15, (2, 3))):
    l.fill(1, y, 31, y)
    l.clear(gap[0], y, gap[1], y)
for lane_floor, wall, trig in ((21, 12, 9), (14, 20, 23), (7, 12, 9)):
    l.fill(wall, lane_floor - 5, wall, lane_floor - 1)
    l.door(0, 0, wall - 1, lane_floor, trap=1, at=(px(trig), lane_floor * 24 + 12))
l.launcher(29, 18)
l.launcher(4, 11)
l.launcher(29, 4)
ladder(l, (2, 3), (20, 18, 16))
ladder(l, (29, 30), (13, 11, 9))
for x in (24, 8):
    l.mine_floor(x, 21)
for x in (24, 9):
    l.mine_floor(x, 7) if x != 9 else None
l.mine_floor(18, 7)
l.put(30, 21, "S"); l.put(5, 7, "K"); l.put(2, 7, "X")
for x, y in ((20, 21), (6, 21), (8, 14), (24, 14), (22, 7), (14, 7)):
    l.gold(x, y)
l.waypoints([(20, 21), (6, 21), (3, 20), (3, 16), (8, 14), (24, 14), (29, 13), (29, 9), (22, 7), (14, 7), (5, 7)], [(2, 7)])
l.write("levels/18-3.txt")

# 18:4 everything: islands, a chaser and a thwomp to the key; a ladder; bounce blocks over mines
# under a laser and a launcher; the switch behind the locked door
l = Level("everything")
l.fill(1, 14, 27, 14)
for x in range(5, 25):
    if x % 5 in (1, 2):
        l.mine_floor(x, 21)
l.drone(9, 16, movement=2, chaser=1)
l.thwomp(21, 15, 1)
ladder(l, (28, 29, 30, 31), (19, 17, 15))
for x in range(11, 18):
    l.mine_floor(x, 13)
for x in (12, 16):
    l.bounce(x, 11)
l.fill(20, 6, 22, 7)
l.drone(19, 5, movement=0, weapon=1)
l.launcher(30, 2)
l.fill(5, 1, 5, 12)
l.door(26, 21, 4, 13, locked=1)
l.put(2, 21, "S"); l.put(2, 13, "K"); l.put(8, 13, "X")
for x, y in ((4, 21), (9, 21), (14, 21), (19, 21), (24, 21), (29, 16), (24, 13), (14, 9), (9, 13)):
    l.gold(x, y)
l.waypoints([(4, 21), (9, 21), (14, 21), (19, 21), (24, 21), (26, 21), (29, 20), (29, 16), (26, 13), (24, 13), (19, 13), (16, 10), (14, 9), (12, 10), (9, 13), (2, 13)], [(8, 13)])
l.write("levels/18-4.txt")
