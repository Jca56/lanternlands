"""A rough look at a frame without Lantern Studio.

    python3 preview.py out.png side step_b            the body alone
    python3 preview.py out.png side step_b all        with everything worn

It turns the same steps Studio would get into an SVG and has
`rsvg-convert` (already on the machine) make a PNG of it. Curves and
brush strokes are only near what Studio draws: this is for checking
where things are, not how they look.
"""
import json, os, subprocess, sys
import player
from rig import Batch, Frame, CELL, ANCHOR, cell_origin

ORDER = ["shield_under", "weapon_under", "body", "legs", "feet", "chest", "hands", "head", "shield", "weapon", "grip"]


def smooth_path(pts, closed):
    """A Catmull-Rom curve through the points, as SVG."""
    n = len(pts)
    if n < 3:
        return "M" + " L".join("%g,%g" % (p[0], p[1]) for p in pts)
    def at(i):
        return pts[i % n] if closed else pts[max(0, min(n - 1, i))]
    d = "M%g,%g" % (pts[0][0], pts[0][1])
    for i in range(n if closed else n - 1):
        p0, p1, p2, p3 = at(i - 1), at(i), at(i + 1), at(i + 2)
        c1 = (p1[0] + (p2[0] - p0[0]) / 6.0, p1[1] + (p2[1] - p0[1]) / 6.0)
        c2 = (p2[0] - (p3[0] - p1[0]) / 6.0, p2[1] - (p3[1] - p1[1]) / 6.0)
        d += " C%g,%g %g,%g %g,%g" % (c1[0], c1[1], c2[0], c2[1], p2[0], p2[1])
    return d + (" Z" if closed else "")


def svg_of(steps, sheets, stamps, offset=(0, 0)):
    """The drawing each sheet's steps make, sheet by sheet."""
    out = dict((s, []) for s in ORDER)
    where = {}                                    # a group's alias -> its sheet
    by_id = dict((v, k) for k, v in sheets.items())
    layer_sheet = {}
    for step in steps:
        tool, a = step["tool"], step["args"]
        if tool == "layer_add" and a.get("kind") == "group":
            where["@" + step["as"]] = by_id.get(a.get("into"), "body")
        elif tool == "layer_add":
            layer_sheet["@" + step["as"]] = where.get(a.get("into"), "body")
        elif tool == "path_add":
            sheet = where.get(a["into"], "body")
            pts = [(p[0] + offset[0], p[1] + offset[1]) for p in a["points"]]
            closed = a.get("closed", False)
            d = smooth_path(pts, closed) if a.get("smooth") else "M" + " L".join("%g,%g" % p for p in pts) + (" Z" if closed else "")
            fill = a.get("fill", "none")
            stroke = a.get("stroke", "none")
            out[sheet].append('<path d="%s" fill="%s" stroke="%s" stroke-width="%g" stroke-linejoin="round" stroke-linecap="round"/>' % (d, fill if closed else "none", stroke, a.get("stroke_width", 0)))
        elif tool == "shape_add":
            sheet = where.get(a["into"], "body")
            fill = a["fill"]
            op = int(fill[7:9], 16) / 255.0 if len(fill) == 9 else 1
            out[sheet].append('<ellipse cx="%g" cy="%g" rx="%g" ry="%g" fill="%s" fill-opacity="%g" stroke="%s" stroke-width="%g"/>' % (a["x"] + a["width"] / 2.0 + offset[0], a["y"] + a["height"] / 2.0 + offset[1], a["width"] / 2.0, a["height"] / 2.0, fill[:7], op, a.get("outline", "none"), a.get("stroke_width", 0)))
        elif tool == "paint_stroke" and a.get("preset") != "spray":
            sheet = layer_sheet.get(a["layer_id"], "body")
            for s in a["strokes"]:
                pts = [(p[0] + offset[0], p[1] + offset[1]) for p in s]
                if len(pts) == 1:
                    pts = pts + [(pts[0][0] + 0.1, pts[0][1])]
                out[sheet].append('<path d="M%s" fill="none" stroke="%s" stroke-opacity="%g" stroke-width="%g" stroke-linecap="round"/>' % (" L".join("%g,%g" % p for p in pts), a["color"], a.get("opacity", 1), a["size"]))
        elif tool == "layer_duplicate" and a["layer_id"] in stamps:
            where["@" + step["as"]] = "stamp:" + a["layer_id"]
        elif tool == "layer_move" and where.get(a["layer_id"], "").startswith("stamp:"):
            out["body"].append(("stamp", where[a["layer_id"]][6:], a["layer_id"]))
        elif tool == "layer_transform":
            for i, item in enumerate(out["body"]):
                if isinstance(item, tuple) and item[2] in a["layer_ids"]:
                    out["body"][i] = ("stamp", item[1], item[2], a["move"])
    return out


def main(argv):
    target, row, col = argv[1], argv[2], argv[3]
    everything = len(argv) > 4
    known = player.ids()
    b = Batch()
    F = Frame(b, row, col, known["sheets"])
    if row == "side":
        player.side(F, col, known["stamps"]["side"])
    else:
        player.front(F, col, row == "up", known["stamps"][row])
    stamp_ids = known["stamps"][row]

    # What a stamp is made of, to stand in for its copies: the stamp
    # function draws its first group as A and its second as B.
    def stamp_items(which):
        one = Batch()
        OF = Frame(one, row, "idle", {"A": "sA", "B": "sB"})
        (player.stamp_side if row == "side" else lambda f: player.stamps_front(f, row == "up"))(OF)
        art = svg_of(one.steps, {"shield_under": "sA", "weapon_under": "sB"}, {})
        return art["shield_under"] if which == "A" else art["weapon_under"]
    art = svg_of(b.steps, known["sheets"], set(stamp_ids.values()))
    ox, oy = cell_origin(row, col)
    hx, hy = cell_origin(row, "idle")
    x0, y0 = ox - ANCHOR[0], oy - ANCHOR[1]
    body = []
    for item in art["body"]:
        if isinstance(item, tuple):
            which = [k for k, v in stamp_ids.items() if v == item[1]][0]
            move = item[3] if len(item) > 3 else (0, 0)
            body.append('<g transform="translate(%g,%g)">%s</g>' % (move[0], move[1], "".join(stamp_items(which))))
        else:
            body.append(item)
    art["body"] = body
    sheets = ORDER if everything else ["body"]
    svg = '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" viewBox="%d %d %d %d"><rect x="%d" y="%d" width="%d" height="%d" fill="#3a4a2c"/>%s</svg>' % (CELL[0], CELL[1], x0, y0, CELL[0], CELL[1], x0, y0, CELL[0], CELL[1], "".join("".join(art[s]) for s in sheets))
    tmp = target + ".svg"
    with open(tmp, "w") as f:
        f.write(svg)
    subprocess.check_call(["rsvg-convert", "-o", target, tmp])
    os.remove(tmp)


if __name__ == "__main__":
    main(sys.argv)
