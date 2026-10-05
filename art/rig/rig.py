"""Shapes for Lantern Studio, worked out in Python.

The scripts here do not draw anything themselves. Each prints a list of
steps for Lantern Studio's `batch` tool (as JSON), and Claude hands that
list to Studio, which draws it. Everything is at twice game size: one
tile is 400 px here and 200 px in the game.

Run with the Python already on the machine; nothing needs installing.
"""
import json, math

OUT = "#14100e"          # the outline every shape has

# A sheet of someone on foot: three rows (down, up, side) of six poses.
ROWS = ["down", "up", "side"]
COLS = ["idle", "step_a", "pass", "step_b", "work_a", "work_b"]
CELL = (720, 960)        # one frame's box, at twice game size
ANCHOR = (360, 880)      # where the feet stand in it


def cell_origin(row, col):
    """Where a frame's feet are on the sheet."""
    return (COLS.index(col) * CELL[0] + ANCHOR[0], ROWS.index(row) * CELL[1] + ANCHOR[1])


def R(v):
    return int(round(v))


class Batch:
    """Collects steps for one call of Studio's `batch`."""

    def __init__(self):
        self.steps = []
        self.n = 0

    def add(self, tool, args, alias=None):
        step = {"tool": tool, "args": args}
        if alias:
            step["as"] = alias
        self.steps.append(step)

    def alias(self, stem="a"):
        self.n += 1
        return "%s%d" % (stem, self.n)

    def dump(self):
        print(json.dumps(self.steps, separators=(",", ":")))


class Frame:
    """One frame being drawn: its origin on the sheet, and the group it
    has in each sheet (made the first time something is drawn there)."""

    def __init__(self, batch, row, col, sheets):
        self.b, self.row, self.col, self.sheets = batch, row, col, sheets
        self.ox, self.oy = cell_origin(row, col)
        self.groups = {}
        self.dt = None

    def P(self, pts, d=(0, 0)):
        return [[R(self.ox + p[0] + d[0]), R(self.oy + p[1] + d[1])] + list(p[2:]) for p in pts]

    def group(self, sheet):
        if sheet not in self.groups:
            alias = self.b.alias("g")
            self.b.add("layer_add", {"kind": "group", "name": "%s %s" % (self.row, self.col), "into": self.sheets[sheet]}, alias)
            self.groups[sheet] = "@" + alias
        return self.groups[sheet]

    def path(self, sheet, pts, fill="none", stroke=OUT, sw=4, smooth=True, closed=True, d=(0, 0)):
        args = {"into": self.group(sheet), "points": self.P(pts, d), "closed": closed, "fill": fill, "stroke": stroke, "stroke_width": sw if stroke != "none" else 0}
        if smooth:
            args["smooth"] = True
        self.b.add("path_add", args)

    def line(self, sheet, pts, stroke=OUT, sw=2.5, d=(0, 0), smooth=False):
        self.path(sheet, pts, "none", stroke, sw, smooth, False, d)

    def ellipse(self, sheet, cx, cy, rx, ry, fill, outline="none", sw=0):
        self.b.add("shape_add", {"kind": "ellipse", "into": self.group(sheet), "x": R(self.ox + cx - rx), "y": R(self.oy + cy - ry), "width": R(2 * rx), "height": R(2 * ry), "fill": fill, "outline": outline, "stroke_width": sw})

    def stamp(self, layer_id, home, d=(0, 0), sheet="body"):
        """A copy of something drawn once for the whole row (the torso,
        the head), moved from where it was drawn to this frame."""
        alias = self.b.alias("s")
        self.b.add("layer_duplicate", {"layer_id": layer_id}, alias)
        self.b.add("layer_move", {"layer_id": "@" + alias, "into": self.group(sheet)})
        move = [R(self.ox - home[0] + d[0]), R(self.oy - home[1] + d[1])]
        if move != [0, 0]:
            self.b.add("layer_transform", {"layer_ids": ["@" + alias], "move": move})

    def details(self, sheet="body"):
        """A layer for the fine lines of this frame, the size of its box."""
        alias = self.b.alias("d")
        self.b.add("layer_add", {"name": "details", "into": self.group(sheet), "x": self.ox - ANCHOR[0], "y": self.oy - ANCHOR[1], "width": CELL[0], "height": CELL[1]}, alias)
        self.dt = "@" + alias

    def paint(self, strokes, size, color, d=(0, 0), **kw):
        if not strokes:
            return
        args = {"layer_id": self.dt, "size": size, "color": color, "strokes": [self.P(s, d) for s in strokes]}
        args.update(kw)
        self.b.add("paint_stroke", args)


# ---- shapes -------------------------------------------------------------

def mirror(pts):
    return [(-p[0],) + tuple(p[1:]) for p in reversed(pts)]


def mirror_strokes(strokes):
    return [[(-p[0],) + tuple(p[1:]) for p in s] for s in strokes]


def shift(pts, dx, dy):
    return [(p[0] + dx, p[1] + dy) + tuple(p[2:]) for p in pts]


def shift_strokes(strokes, dx, dy):
    return [shift(s, dx, dy) for s in strokes]


def stretch(pts, dy, y0, y1):
    """Move the lower part of a limb by `dy`, a little near `y0` and all
    of it from `y1` down: how a leg or an arm reaches or draws up."""
    out = []
    for p in pts:
        t = min(1.0, max(0.0, (p[1] - y0) / float(y1 - y0)))
        out.append((p[0], p[1] + dy * t) + tuple(p[2:]))
    return out


def stretch_strokes(strokes, dy, y0, y1):
    return [stretch(s, dy, y0, y1) for s in strokes]


def clamp_top(pts, ymin):
    return [(p[0], max(p[1], ymin)) for p in pts]


def clamp_bottom(pts, ymax):
    return [(p[0], min(p[1], ymax)) for p in pts]


def unit(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    n = math.hypot(dx, dy) or 1.0
    return (dx / n, dy / n)


def tube(joints, widths):
    """A limb as one outline: the joints down its middle, and half its
    width at each."""
    left, right = [], []
    for i, p in enumerate(joints):
        a = joints[max(i - 1, 0)]
        b = joints[min(i + 1, len(joints) - 1)]
        ux, uy = unit(a, b)
        nx, ny = -uy, ux
        w = widths[i]
        left.append((p[0] + nx * w, p[1] + ny * w))
        right.append((p[0] - nx * w, p[1] - ny * w))
    return left + right[::-1]


def along(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def frame_of(c, d):
    """Points measured along a direction `d` from `c` and across it."""
    n = (-d[1], d[0])
    return lambda s, t=0: (c[0] + s * d[0] + t * n[0], c[1] + s * d[1] + t * n[1])
