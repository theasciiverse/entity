"""Episode 5, doors: trek doors, locked doors and their keys, trap doors."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 5:0 knock knock: rooms joined by trek doors, each holding a drone that the closed doors keep in
l = Level("knock knock")
for w in (8, 16, 24):
    l.fill(w, 1, w, 20)
    l.door(0, 0, w - 1, 21)            # trek door on the gap at the foot of the wall
    l.fill(w, 21, w, 21, " ")
l.fill(1, 11, 5, 11)                   # the door's ledge in the first room
l.fill(1, 16, 2, 16)
for x0 in (9, 17, 25):                 # a platform in each room
    l.fill(x0 + 2, 15, x0 + 4, 15)
l.drone(12, 18, movement=2)
l.drone(20, 12, movement=3)
l.drone(28, 18, movement=2)
l.put(3, 21, "S"); l.put(30, 21, "K"); l.put(4, 10, "X")
for x, y in ((12, 13), (20, 13), (28, 13), (10, 20), (18, 20), (26, 20), (6, 19), (2, 14)):
    l.gold(x, y)
l.waypoints([(10, 20), (12, 13), (18, 20), (20, 13), (26, 20), (28, 13), (30, 21)], [(23, 21), (15, 21), (6, 19), (2, 14), (4, 10)])
l.write("levels/05-0.txt")

# 5:1 keyring: the switch behind three locked doors, each key somewhere else in the level
l = Level("keyring")
l.fill(4, 5, 4, 21)                    # the climb at the start
l.fill(19, 13, 31, 13)                 # roof of the vault
for w, key in ((19, (4, 4)), (23, (13, 6)), (27, (29, 12))):
    l.fill(w, 14, w, 20)
    l.door(*key, w, 21, locked=1)
for x0, y in ((21, 11), (16, 9), (12, 7)):
    l.fill(x0, y, x0 + 2, y)
l.fill(16, 17, 17, 17)                 # a step down from the roof
l.put(2, 21, "S"); l.put(30, 21, "K"); l.put(10, 21, "X")
for x, y in ((2, 18), (2, 15), (2, 12), (2, 9), (8, 5), (17, 4), (22, 10), (25, 10), (17, 16), (21, 21), (25, 21), (14, 21)):
    l.gold(x, y)
l.waypoints([(2, 18), (2, 15), (2, 12), (2, 9), (4, 4), (8, 5), (13, 6), (17, 4), (17, 8), (22, 10), (25, 10), (29, 12), (17, 16), (18, 21), (21, 21), (25, 21), (30, 21)], [(14, 21), (10, 21)])
l.write("levels/05-1.txt")

# 5:2 trapped: the short way to the switch crawls past the trigger of the exit's trap door
l = Level("trapped")
l.fill(1, 17, 5, 17)                   # roof of the exit's room
l.fill(5, 18, 5, 20)
l.door(0, 0, 4, 21, trap=1, at=(px(18), 21 * 24 + 12))
l.fill(9, 12, 27, 20)                  # the block, with a crawlspace under it
l.fill(6, 19, 7, 19)
l.fill(1, 15, 3, 15)
l.fill(5, 13, 7, 13)
l.fill(30, 17, 31, 17)
l.put(7, 21, "S"); l.put(29, 21, "K"); l.put(2, 21, "X")
for x in (12, 16, 20, 24):
    l.gold(x, 10)
for x, y in ((30, 14), (2, 13), (6, 11), (29, 9), (30, 19), (8, 20)):
    l.gold(x, y)
l.waypoints([(6, 18), (3, 16), (2, 13), (6, 11), (12, 10), (16, 10), (20, 10), (24, 10), (29, 9), (30, 14), (30, 16), (30, 19), (29, 21)], [(30, 16), (29, 11), (20, 11), (8, 11), (6, 12), (2, 14), (3, 16), (6, 18), (8, 20), (2, 21)])
l.write("levels/05-2.txt")

# 5:3 airlock: three floors, each with a locked hatch whose key is on the floor above it
l = Level("airlock")
for y in (5, 10, 15):
    l.fill(1, y, 31, y)
for hx, hy, key in ((29, 5, (16, 3)), (2, 10, (24, 7)), (29, 15, (16, 13))):
    l.clear(hx, hy, hx, hy)
    l.door(*key, hx, hy - 1, vertical=1, locked=1)
l.fill(15, 4, 17, 4)                   # the first key's block
for x in (12, 13, 19, 20):
    l.mine_floor(x, 4)
l.fill(23, 8, 25, 8)                   # the second key's ledge
for x in (14, 18):
    l.mine_floor(x, 9)
    l.mine_ceiling(x, 6)
l.fill(15, 14, 17, 14)
for x in (8, 9, 23, 24):
    l.mine_floor(x, 14)
l.fill(27, 18, 31, 18)                 # a ledge to break the last drop
for x in range(10, 15):
    l.mine_floor(x, 21)
l.put(3, 4, "S"); l.put(16, 9, "K"); l.put(5, 21, "X")
for x, y in ((8, 3), (24, 3), (8, 8), (12, 13), (20, 12), (20, 19), (29, 7), (2, 7), (2, 13), (29, 13), (25, 19), (9, 19)):
    l.gold(x, y)
l.waypoints([(8, 3), (16, 3), (24, 3), (29, 4), (29, 7), (24, 7), (16, 9)], [(8, 8), (2, 7), (2, 9), (2, 13), (12, 13), (16, 13), (20, 12), (29, 13), (29, 17), (25, 19), (20, 19), (9, 19), (5, 21)])
l.write("levels/05-3.txt")

# 5:4 vault: the key up a climb beyond a drone's room, the switch behind its locked door
l = Level("vault")
l.fill(9, 14, 18, 14)                  # the drone's room
for w in (9, 18):
    l.fill(w, 15, w, 20)
l.door(0, 0, 8, 21)
l.door(0, 0, 18, 21)
l.drone(13, 18, movement=2)
l.fill(23, 14, 31, 14)                 # the vault
l.fill(23, 15, 23, 20)
l.door(6, 8, 23, 21, locked=1)
for i, y in enumerate((19, 17, 15, 13, 11, 9)):   # the climb to the key
    x0 = 1 if i % 2 == 0 else 5
    l.fill(x0, y, x0 + 2, y)
l.fill(27, 10, 27, 13)                 # a wall on the vault's roof
l.put(20, 21, "S"); l.put(30, 21, "K"); l.put(30, 13, "X")
for x, y in ((11, 20), (16, 20), (2, 18), (6, 16), (2, 14), (6, 12), (2, 10), (21, 17), (21, 12), (25, 13), (26, 21), (28, 21)):
    l.gold(x, y)
l.waypoints([(16, 20), (11, 20), (6, 20), (2, 18), (6, 16), (2, 14), (6, 12), (2, 10), (6, 8), (2, 18), (8, 21), (13, 21), (19, 21), (26, 21), (28, 21), (30, 21)], [(21, 20), (21, 17), (21, 12), (25, 13), (27, 9), (30, 13)])
l.write("levels/05-4.txt")
