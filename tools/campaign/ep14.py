"""Episode 14, route puzzles: one-way tiles as gates and valves, doors in order."""
import sys
sys.path.insert(0, __file__.rsplit("/", 2)[0])
from mklevel import Level, px

# 14:0 one way street: two lanes of one-way gates, right along the bottom, left along the top
l = Level("one way street")
l.fill(4, 17, 28, 17)
for x in (10, 18):
    for y in range(18, 22):
        l.oneway(x, y, 0)
for x in (12, 20):
    for y in range(13, 17):
        l.oneway(x, y, 2)
for y in (19, 17, 15):
    for x in (29, 30, 31):
        l.oneway(x, y)
l.fill(1, 12, 27, 12)
l.put(2, 21, "S"); l.put(26, 21, "K"); l.put(2, 16, "X")
for x in (7, 14, 22, 24, 16, 8):
    l.gold(x, 21 if x in (7, 14, 22) else 16)
l.waypoints([(7, 21), (14, 21), (22, 21), (26, 21)], [(29, 20), (30, 16), (24, 16), (16, 16), (8, 16), (2, 16)])
l.write("levels/14-0.txt")

# 14:1 valves: four floors, each with a way up and a way down that only work one way
def up_valve(l, x, r):
    """A two-wide up-facing gap in the floor at row r, over a little staircase."""
    for xx in (x, x + 1):
        l.put(xx, r, " ")
        l.oneway(xx, r, 3)
    l.fill(x, r + 2, x + 1, r + 2)
    l.fill(x - 1, r + 3, x - 1, r + 3)

def down_valve(l, x, r):
    for xx in (x, x + 1):
        l.put(xx, r, " ")
        l.oneway(xx, r, 1)

l = Level("valves")
for r in (5, 9, 13, 17):
    l.fill(1, r, 31, r)
up_valve(l, 4, 17); down_valve(l, 28, 17)
up_valve(l, 27, 13); down_valve(l, 15, 13)
up_valve(l, 5, 9); down_valve(l, 22, 9)
up_valve(l, 15, 5)
l.put(2, 21, "S"); l.put(30, 8, "K"); l.put(30, 21, "X")
for x, y in ((16, 16), (16, 12), (14, 8), (8, 4), (24, 4), (20, 12), (20, 21)):
    l.gold(x, y)
l.waypoints([(3, 21), (4, 19), (5, 16), (16, 16), (26, 16), (27, 15), (28, 12), (20, 12), (16, 12), (6, 12), (5, 11), (6, 8), (14, 8), (15, 7), (16, 4), (8, 4), (24, 4), (16, 4), (16, 8), (30, 8)],
            [(23, 8), (22, 12), (16, 12), (16, 16), (28, 16), (28, 21), (20, 21), (30, 21)])
l.write("levels/14-1.txt")

# 14:2 chain of keys: four rooms in a row; each locked door's key is high in the room before it
l = Level("chain of keys")
keys = [(4, 8), (12, 6), (20, 10)]
for w, key in zip((8, 16, 24), keys):
    l.fill(w, 1, w, 20)
    l.door(*key, w, 21, locked=1)
for x0, y in ((2, 19), (5, 17), (2, 15), (5, 13), (2, 11), (4, 9)):
    l.fill(x0, y, x0 + 1, y)
for x0, y in ((10, 19), (13, 17), (10, 15), (13, 13), (10, 11), (13, 9), (11, 7)):
    l.fill(x0, y, x0 + 1, y)
for x0, y in ((18, 19), (21, 17), (18, 15), (21, 13), (19, 11)):
    l.fill(x0, y, x0 + 1, y)
for x0, y in ((26, 19), (29, 17), (26, 15), (29, 13), (26, 11), (29, 9), (27, 7), (28, 6)):
    l.fill(x0, y, x0 + 1, y)
l.put(2, 21, "S"); l.put(28, 5, "K"); l.put(5, 21, "X")
for x, y in ((6, 16), (14, 12), (22, 16), (30, 12), (11, 18), (27, 14)):
    l.gold(x, y)
l.waypoints([(3, 18), (6, 16), (3, 14), (6, 12), (3, 10), (4, 8), (7, 20), (11, 18), (14, 16), (11, 14), (14, 12), (11, 10), (14, 8), (12, 6), (15, 20), (19, 18), (22, 16), (19, 14), (22, 12), (20, 10), (23, 20), (27, 18), (30, 16), (27, 14), (30, 12), (27, 10), (30, 8), (28, 6), (28, 5)],
            [(26, 20), (18, 21), (10, 21), (5, 21)])
l.write("levels/14-2.txt")

# 14:3 wrong order: the way to the locked switch room passes the trigger of a trap door that
# shuts the room with the key; fetch the key first
l = Level("wrong order")
l.fill(1, 12, 27, 12)
l.fill(8, 13, 8, 20)
l.door(0, 0, 7, 21, trap=1, at=(px(21), 21 * 24 + 12))
l.fill(24, 13, 24, 20)
l.door(3, 18, 24, 21, locked=1)
for x0, y in ((2, 19), (4, 17)):
    l.fill(x0, y, x0 + 1, y)
for y in (19, 17, 15, 13):
    for x in (29, 30, 31):
        l.oneway(x, y)
l.put(16, 21, "S"); l.put(27, 21, "K"); l.put(16, 11, "X")
for x, y in ((11, 21), (5, 16), (19, 21), (29, 16), (23, 11), (9, 11)):
    l.gold(x, y)
l.waypoints([(11, 21), (6, 21), (3, 18), (5, 16), (3, 18), (6, 21), (11, 21), (19, 21), (25, 21), (27, 21)], [(29, 18), (29, 16), (29, 12), (23, 11), (16, 11), (9, 11), (16, 11)])
l.write("levels/14-3.txt")

# 14:4 switchyard: gates right along the bottom to the key, a ladder, gates left along the
# middle, a ladder, and the switch behind the locked door on top
l = Level("switchyard")
l.fill(1, 14, 27, 14)
l.fill(5, 7, 31, 7)
for x in (9, 19):
    for y in range(15, 22):
        l.oneway(x, y, 0)
for x in (20, 10):
    for y in range(8, 14):
        l.oneway(x, y, 2)
for y in (19, 17, 15):
    for x in (29, 30, 31):
        l.oneway(x, y)
for y in (12, 10, 8):
    for x in (2, 3, 4):
        l.oneway(x, y)
l.fill(27, 1, 27, 5)
l.door(25, 21, 26, 6, locked=1)
l.put(2, 21, "S"); l.put(30, 6, "K"); l.put(8, 6, "X")
for x, y in ((6, 21), (14, 21), (22, 21), (30, 16), (24, 13), (15, 13), (6, 13), (3, 9), (14, 6), (22, 6)):
    l.gold(x, y)
l.waypoints([(6, 21), (14, 21), (22, 21), (25, 21), (29, 20), (30, 16), (27, 13), (24, 13), (15, 13), (6, 13), (3, 12), (3, 9), (5, 6), (14, 6), (22, 6), (30, 6)], [(22, 6), (14, 6), (8, 6)])
l.write("levels/14-4.txt")
