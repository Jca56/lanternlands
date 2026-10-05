"""The player on foot, and what they wear, as steps for Lantern Studio.

    python3 player.py setup            the empty sheets
    python3 player.py stamps down      what a row draws once (torso, head)
    python3 player.py frame down idle  one frame, on every sheet
    python3 player.py grade            the colour grade, last of all

`ids.json` beside this file says which Studio layers the sheets and the
stamps are; Claude fills it in from what Studio answers.
"""
import json, os, sys
from rig import *

HERE = os.path.dirname(os.path.abspath(__file__))

# Colours. FAR is the side of a body turned away from us.
SKIN, FACE, TUNIC, LEATHER, DARKL = "#b78a65", "#ad8261", "#465669", "#4a3626", "#33241a"
TROUSER, BOOT, CUFF, MANTLE, HAIR = "#3a2e27", "#2b201a", "#463325", "#4b3b2d", "#2a1f1a"
FAR_TROUSER, FAR_BOOT, FAR_CUFF, FAR_TUNIC, FAR_LEATHER, FAR_SKIN = "#2f2520", "#231a15", "#392a1f", "#3a4859", "#3d2c1f", "#9a7355"
# Gear is drawn in greys and coloured by its metal in the game.
G1, G2, G3, GD = "#cfcfcf", "#a9a9a9", "#ececec", "#6f6f6f"
SHADOW = "#00000046"

K = 0.88                 # the front and back are this much narrower than first drawn
FX = 138 - 138 * K       # how far that brought the sword hand in


def Bx(pts):
    return [(p[0] * K,) + tuple(p[1:]) for p in pts]


def Bs(strokes):
    return [Bx(s) for s in strokes]


def Rg(pts, cx):
    """Moved in with the body, but not squeezed: hands, a pouch."""
    return shift(pts, cx * K - cx, 0)


def Rs(strokes, cx):
    return [Rg(s, cx) for s in strokes]


# ---- the sword and the hand that holds it -------------------------------

def sword(F, sheet, c, d):
    at = frame_of(c, d)
    F.path(sheet, [at(30, 12), at(265, 10), at(300), at(265, -10), at(30, -12)], "#d6d6d6", smooth=False, sw=3)
    F.line(sheet, [at(40), at(220)], "#9a9a9a", 6)
    F.path(sheet, [at(23, 36), at(37, 36), at(37, -36), at(23, -36)], "#8a8a8a", smooth=False, sw=3)
    F.path(sheet, [at(23, 7), at(-30, 7), at(-30, -7), at(23, -7)], "#5a5a5a", smooth=False, sw=3)
    F.path(sheet, [at(-40, -12), at(-28, 0), at(-40, 12), at(-52, 0)], "#8a8a8a", sw=3)


FIST = [(23, -12), (23, 12), (14, 20), (-14, 20), (-23, 12), (-23, -12), (-14, -20), (14, -20)]


def fist(F, sheet, c, d, fill=SKIN, thumb=True, knuckles=False):
    at = frame_of(c, d)
    F.path(sheet, [at(s, t) for s, t in FIST], fill, sw=3)
    if knuckles:
        for s in (11.5, 0, -11.5):
            F.line(sheet, [at(s, -18), at(s, 18)])
    if thumb:
        F.line(sheet, [at(15, 15), at(8, -7)], OUT, 15)
        F.line(sheet, [at(15, 15), at(8, -7)], fill, 9.5)


def knuckle_strokes(c, d):
    at = frame_of(c, d)
    return [[at(s, -18), at(s, 18)] for s in (11.5, 0, -11.5)]


def bent_arm(S, E, W):
    """An arm from its joints: sleeve, bracer and wrist outlines, and
    where the straps cross the bracer."""
    sleeve = tube([S, E, along(E, W, 0.3)], [26, 24, 22])
    bracer = tube([along(E, W, 0.3), along(E, W, 0.96)], [24, 21])
    u = unit(E, W)
    n = (-u[1], u[0])
    straps = []
    for t, w in ((0.5, 23), (0.8, 21)):
        m = along(E, W, t)
        straps.append([(m[0] + n[0] * w, m[1] + n[1] * w), (m[0] - n[0] * w, m[1] - n[1] * w)])
    return sleeve, bracer, straps


def shield(F, sheet, cx, cy):
    body = [(-62, -92), (62, -92), (66, -10), (40, 60), (0, 96), (-40, 60), (-66, -10)]
    F.path(sheet, shift(body, cx, cy), G1, smooth=False)
    F.path(sheet, shift([(p[0] * 0.7, p[1] * 0.7 - 6) for p in body], cx, cy), G2, smooth=False, sw=2)
    F.ellipse(sheet, cx, cy - 14, 17, 17, G3, OUT, 3)


# ---- front and back -----------------------------------------------------

LEG = [(-6, -310), (-88, -310), (-94, -240), (-86, -170), (-84, -110), (-78, -60), (-30, -60), (-28, -110), (-26, -170), (-10, -255)]
BOOT_FRONT = [(-88, -135), (-24, -135), (-26, -90), (-30, -52), (-22, -22), (-22, 2), (-44, 18), (-76, 16), (-94, 0), (-92, -26), (-82, -52), (-86, -90)]
BOOT_BACK = [(-88, -135), (-24, -135), (-26, -90), (-30, -52), (-26, -22), (-28, 0), (-44, 8), (-70, 8), (-86, 0), (-88, -26), (-82, -52), (-86, -90)]
CUFF_FB = [(-93, -142), (-56, -138), (-19, -142), (-21, -112), (-56, -108), (-91, -112)]
TUNIC_FB = [(-44, -548), (-102, -528), (-116, -480), (-104, -415), (-92, -350), (-104, -262), (-70, -246), (-36, -256), (0, -242), (36, -256), (70, -246), (104, -262), (92, -350), (104, -415), (116, -480), (102, -528), (44, -548), (0, -538)]
MANTLE_FB = [(-120, -516), (-84, -546), (-40, -556), (0, -548), (40, -556), (84, -546), (120, -516), (132, -474), (104, -452), (70, -462), (40, -446), (0, -458), (-40, -446), (-70, -462), (-104, -452), (-132, -474)]
GORGET_FB = [(-120, -516), (-84, -546), (-62, -546), (-32, -530), (0, -526), (32, -530), (62, -546), (84, -546), (120, -516), (132, -474), (104, -452), (70, -462), (40, -446), (0, -458), (-40, -446), (-70, -462), (-104, -452), (-132, -474)]
# The arm that holds the sword and the arm that does not, as the front shows them.
ARM_S = [(-92, -520), (-126, -512), (-150, -450), (-166, -390), (-162, -330), (-156, -286), (-120, -284), (-116, -330), (-116, -390), (-100, -450)]
ARM_F = [(92, -520), (126, -512), (146, -450), (154, -390), (152, -330), (148, -286), (112, -284), (108, -330), (108, -390), (100, -450)]
BRACER_S = [(-168, -366), (-114, -366), (-118, -292), (-158, -292)]
BRACER_F = [(106, -366), (156, -366), (150, -292), (110, -292)]
HAND_F = [(110, -290), (148, -290), (152, -262), (146, -236), (128, -228), (114, -240)]
THUMB_F = [(110, -284), (102, -268), (106, -250), (116, -258)]
WRIST_S = [(-154, -293), (-118, -293), (-114, -274), (-148, -272)]
HELM_FRONT = [(-70, -618), (-74, -652), (-58, -694), (-26, -716), (0, -720), (26, -716), (58, -694), (74, -652), (70, -618), (30, -614), (0, -612), (-30, -614)]
HELM_BACK = [(-72, -600), (-74, -650), (-58, -694), (-26, -716), (0, -720), (26, -716), (58, -694), (74, -650), (72, -600), (62, -564), (0, -554), (-62, -564)]

A0, A1 = -450, -290       # an arm reaches or draws up between these heights
L0, L1 = -250, -60        # and a leg between these


def stamps_front(F, up):
    if not up:
        F.path("A", Bx(TUNIC_FB), TUNIC)
        F.path("A", Bx([(-92, -512), (-68, -524), (98, -362), (84, -342)]), "#3a2a1e", smooth=False, sw=3)
        F.path("A", Bx([(-96, -362), (96, -350), (98, -322), (-97, -334)]), DARKL, smooth=False, sw=3)
        F.path("A", Rg([(14, -340), (30, -338), (34, -290), (22, -286)], 24), DARKL, smooth=False, sw=3)
        F.path("A", [(-18, -360), (14, -358), (14, -324), (-18, -326)], "#2a1d15", "#8a7540", 6, False)
        F.path("A", Rg([(48, -338), (92, -334), (96, -290), (72, -278), (48, -290)], 72), LEATHER, sw=3)
        F.details("A")
        F.paint(Bs([[(-70, -428), (-72, -366)], [(-38, -428), (-39, -364)], [(-2, -406), (-2, -362)], [(40, -428), (40, -412)], [(40, -374), (40, -360)], [(72, -428), (72, -386)], [(-70, -316), (-76, -254)], [(-38, -314), (-40, -252)], [(-2, -310), (0, -246)]]), 3, "#2b3544", opacity=0.7)
        F.paint(Rs([[(50, -316), (72, -308), (95, -314)]], 72), 3, "#1d1512", smooth=True)
        F.paint(Rs([[(72, -299)]], 72), 7, "#8a7540")
        px, py = -70 * K, -284
        F.paint([[(px, py)]], 24, "#52627a", tip="square", rotation=8, opacity=0.5)
        F.paint([[(px + a, py + b), (px + c, py + e)] for a, b, c, e in [(-6, -14, -6, -9), (6, -13, 6, -8), (-7, 9, -7, 14), (5, 10, 5, 15), (-14, -6, -10, -5), (-15, 5, -11, 6), (10, -4, 14, -3), (9, 7, 13, 8)]], 2, "#232b36", opacity=0.8)
        F.paint(Bs([[(-100, -260), (-40, -250), (40, -250), (100, -262)]]), 20, "#4c4536", preset="spray", opacity=0.4)
        F.path("B", Bx(MANTLE_FB), MANTLE)
        F.path("B", [(-26, -566), (26, -566), (30, -538), (-30, -538)], "#8e6549", smooth=False, sw=3)
        F.path("B", [(-48, -548), (-20, -536), (0, -532), (20, -536), (48, -548), (40, -524), (0, -512), (-40, -524)], "#3d2f24", sw=3)
        ear = [(-54, -618), (-66, -612), (-67, -592), (-54, -580)]
        F.path("B", ear, FACE, sw=3)
        F.path("B", mirror(ear), FACE, sw=3)
        F.path("B", [(-52, -640), (-56, -602), (-50, -572), (-34, -548), (0, -538), (34, -548), (50, -572), (56, -602), (52, -640), (34, -672), (0, -682), (-34, -672)], FACE)
        eye = [(-34, -598), (-22, -602), (-10, -598), (-22, -594)]
        F.path("B", eye, "#b9ad98", "#1c1512", 2.5)
        F.path("B", mirror(eye), "#b9ad98", "#1c1512", 2.5)
        iris = [(-22, -602), (-18, -598), (-22, -594), (-26, -598)]
        F.path("B", iris, "#33251a", "none")
        F.path("B", mirror(iris), "#33251a", "none")
        F.path("B", [(-58, -604), (-63, -636), (-57, -668), (-36, -692), (-6, -702), (26, -699), (50, -683), (63, -654), (60, -606), (54, -618), (50, -638), (38, -648), (20, -652), (2, -646), (-16, -650), (-32, -644), (-46, -634), (-52, -618)], HAIR, sw=3)
        F.line("B", [(-12, -571), (-4, -566), (5, -566), (13, -572)], "#6d4631", 4, smooth=True)
        F.line("B", [(-18, -552), (-6, -554), (7, -554), (19, -551)], "#4a2c24", 4, smooth=True)
        F.line("B", [(-37, -630), (-31, -606), (-29, -584)], "#d3a78f", 3, smooth=True)
        F.details("B")
        F.paint(Bs([[(-124, -482), (-102, -468), (-70, -477), (-40, -462), (0, -474), (40, -462), (70, -477), (102, -468), (124, -482)]]), 3.5, "#7a6248", spacing=2, smooth=True)
        F.paint([[(0, -518)]], 13, "#77726a")
        F.paint([[(0, -518)]], 6, "#1a1512")
        F.paint([[(-4, -522)]], 3, "#d8d4cc")
        F.paint(Bs([[(-100, -500), (-60, -520)], [(40, -510), (90, -492)]]), 18, "#6f5a44", preset="spray", opacity=0.3)
        F.paint([[(-40, -668, .2), (-20, -688, 1), (6, -694, .3)], [(-48, -644, .2), (-34, -670, 1), (-10, -684, .3)], [(0, -690, .2), (26, -688, 1), (46, -672, .3)], [(-20, -660, .3), (0, -668, 1), (24, -662, .3)], [(30, -670, .3), (48, -656, 1), (56, -630, .3)]], 3, "#4f3d30", smooth=True)
        F.paint([[(-30, -656, .3), (-10, -674, 1), (16, -678, .3)], [(-54, -630, .3), (-48, -656, 1), (-34, -678, .3)], [(12, -656, .3), (36, -662, 1), (52, -648, .3)]], 3, "#150f0d", smooth=True)
        F.paint([[(-35, -599), (-22, -604), (-9, -599)], [(9, -599), (22, -604), (35, -599)]], 3, "#17110f", smooth=True)
        F.paint([[(-24, -600)], [(20, -600)]], 2.5, "#e8e0d0")
        F.paint([[(-32, -590), (-22, -588), (-12, -591)], [(12, -591), (22, -588), (32, -590)], [(-2, -616), (-1, -608)], [(3, -616), (2, -608)]], 2, "#8b6248", smooth=True)
        F.paint([[(-6, -565)], [(6, -565)], [(-20, -551), (-17, -548)], [(18, -550), (21, -547)]], 3.5, "#4a2c24")
        F.paint([[(-43, -609, .5), (-26, -613, 1), (-8, -606, .45)], [(8, -606, .45), (26, -613, 1), (43, -609, .5)]], 6, "#231915", smooth=True)
        F.paint([[(-34, -690, 1), (-42, -704, .2)], [(-10, -700, 1), (-12, -714, .2)], [(20, -699, 1), (28, -712, .2)], [(46, -684, 1), (58, -694, .2)], [(-60, -638, 1), (-69, -646, .2)], [(61, -636, 1), (69, -642, .2)]], 5, HAIR)
    else:
        F.path("A", Bx(TUNIC_FB), TUNIC)
        F.path("A", Bx(mirror([(-92, -512), (-68, -524), (98, -362), (84, -342)])), "#3a2a1e", smooth=False, sw=3)
        F.path("A", Bx(mirror([(-96, -362), (96, -350), (98, -322), (-97, -334)])), DARKL, smooth=False, sw=3)
        F.details("A")
        F.paint(Bs([[(-70, -428), (-72, -366)], [(-38, -410), (-39, -364)], [(0, -440), (0, -362)], [(40, -428), (40, -364)], [(72, -428), (72, -400)], [(-70, -316), (-76, -254)], [(-38, -314), (-40, -252)], [(0, -312), (0, -246)], [(40, -316), (42, -252)], [(72, -318), (78, -256)]]), 3, "#2b3544", opacity=0.7)
        F.paint(Bs([[(-100, -260), (-40, -250), (40, -250), (100, -262)]]), 20, "#4c4536", preset="spray", opacity=0.4)
        F.path("B", Bx(MANTLE_FB), MANTLE)
        F.path("B", [(-26, -566), (26, -566), (30, -538), (-30, -538)], "#8e6549", smooth=False, sw=3)
        F.path("B", [(-46, -550), (0, -562), (46, -550), (42, -506), (0, -480), (-42, -506)], "#3d2f24", sw=3)
        ear = [(-54, -618), (-66, -612), (-67, -592), (-54, -580)]
        F.path("B", ear, FACE, sw=3)
        F.path("B", mirror(ear), FACE, sw=3)
        F.path("B", [(-58, -640), (-62, -600), (-56, -572), (-40, -552), (0, -546), (40, -552), (56, -572), (62, -600), (58, -640), (40, -684), (0, -702), (-40, -684)], HAIR)
        F.details("B")
        F.paint(Bs([[(-124, -482), (-102, -468), (-70, -477), (-40, -462), (0, -474), (40, -462), (70, -477), (102, -468), (124, -482)]]), 3.5, "#7a6248", spacing=2, smooth=True)
        F.paint(Bs([[(-100, -500), (-64, -514)], [(60, -512), (96, -494)]]), 18, "#6f5a44", preset="spray", opacity=0.3)
        F.paint([[(-32, -524), (0, -508), (32, -524)], [(-22, -510, .3), (0, -496, 1), (22, -510, .3)]], 3, "#241a14", smooth=True)
        F.paint([[(-6, -690, .2), (-30, -668, 1), (-50, -620, .3)], [(6, -690, .2), (32, -666, 1), (52, -616, .3)], [(-2, -676, .2), (-16, -640, 1), (-22, -590, .3)], [(4, -674, .2), (18, -636, 1), (22, -584, .3)], [(-40, -640, .3), (-48, -606, 1), (-44, -572, .3)]], 3, "#4f3d30", smooth=True)
        F.paint([[(0, -690, .3), (-22, -660, 1), (-36, -612, .3)], [(2, -688, .3), (26, -656, 1), (38, -606, .3)], [(0, -660, .3), (2, -620, 1), (0, -570, .3)], [(44, -640, .3), (50, -604, 1), (44, -568, .3)]], 3, "#150f0d", smooth=True)
        F.paint([[(-34, -690, 1), (-42, -704, .2)], [(-8, -700, 1), (-10, -714, .2)], [(22, -698, 1), (30, -711, .2)], [(-60, -636, 1), (-69, -644, .2)], [(61, -630, 1), (69, -636, .2)], [(-30, -552, 1), (-36, -540, .2)], [(-8, -548, 1), (-10, -536, .2)], [(14, -548, 1), (16, -536, .2)], [(34, -552, 1), (40, -542, .2)]], 5, HAIR)


# What each front or back pose does: how far each leg and arm reaches
# (down the screen is toward us), how much the body drops, and for a
# swing, the joints of the sword arm and which way the sword points.
# Feet together ("pass") is the standing pose again: on the sheet those
# two frames were made by copying the standing ones.
FRONT = {
    "idle": dict(legs=(0, 0), arms=(0, 0), bob=0),
    "step_a": dict(legs=(14, -18), arms=(-10, 10), bob=6),
    "pass": dict(legs=(0, 0), arms=(0, 0), bob=0),
    "step_b": dict(legs=(-18, 14), arms=(10, -10), bob=6),
    "work_a": dict(legs=(0, 0), arms=(0, 0), bob=0, swing=((-88, -500), (-152, -500), (-150, -574), (-152, -600), (-0.42, -0.908))),
    "work_b": dict(legs=(6, -4), arms=(0, 0), bob=4, swing=((-88, -500), (-124, -418), (-74, -350), (-56, -330), (0.5, 0.866))),
}
BACK = {
    "idle": dict(legs=(0, 0), arms=(0, 0), bob=0),
    "step_a": dict(legs=(14, -18), arms=(10, -10), bob=6),
    "pass": dict(legs=(0, 0), arms=(0, 0), bob=0),
    "step_b": dict(legs=(-18, 14), arms=(-10, 10), bob=6),
    "work_a": dict(legs=(0, 0), arms=(0, 0), bob=0, swing=((88, -500), (152, -500), (150, -574), (152, -600), (0.42, -0.908))),
    "work_b": dict(legs=(-4, 6), arms=(0, 0), bob=4, swing=((88, -500), (128, -420), (156, -344), (160, -316), (0.26, 0.966))),
}


def front(F, col, up, stamps):
    pose = (BACK if up else FRONT)[col]
    (dyl, dyr), (dys, dyf), bob = pose["legs"], pose["arms"], pose["bob"]
    swing = pose.get("swing")
    home = cell_origin(F.row, "idle")
    sx = -1 if up else 1                      # the sword arm is on our left from the front, our right from behind

    def side(pts, flip):
        return mirror(pts) if flip else pts

    F.ellipse("body", 0, 6, 125, 30, SHADOW)
    # Legs.
    boot = BOOT_BACK if up else BOOT_FRONT
    for flip, dy in ((False, dyl), (True, dyr)):
        leg = stretch(Bx(side(LEG, flip)), dy, L0, L1)
        b = stretch(Bx(side(boot, flip)), dy, L0, L1)
        c = stretch(Bx(side(CUFF_FB, flip)), dy, L0, L1)
        F.path("body", leg, TROUSER)
        F.path("body", b, BOOT)
        F.path("body", c, CUFF, smooth=False)
        F.path("legs", clamp_top(leg, -246 + bob), G1)
        knee = stretch(Bx(side([(-84, -168), (-26, -168)], flip)), dy, L0, L1)
        F.line("legs", knee)
        F.path("feet", b, G1)
        F.path("feet", c, G2, smooth=False)
    F.stamp(stamps["A"], home, (0, bob))
    ub = (0, bob)
    belt = [(-96, -362), (96, -350), (98, -322), (-97, -334)]
    F.path("chest", Bx(TUNIC_FB), G1, d=ub)
    F.path("chest", Bx(mirror(belt) if up else belt), G2, smooth=False, sw=3, d=ub)
    F.line("chest", [(0, -520), (0, -366)], d=ub)
    # Arms: the sword arm, then the free one.
    if swing:
        S, E, W, Fc, d = swing
        sleeve, bracer, straps = bent_arm(S, E, W)
        F.path("body", sleeve, TUNIC, d=ub)
        F.path("body", bracer, LEATHER, smooth=False, sw=3, d=ub)
        F.path("chest", sleeve, G1, d=ub)
        F.path("hands", bracer, G2, smooth=False, sw=3, d=ub)
        fc = (Fc[0], Fc[1] + bob)
    else:
        sleeve = stretch(Bx(side(ARM_S, up)), dys, A0, A1)
        bracer = stretch(Bx(side(BRACER_S, up)), dys, A0, A1)
        wrist = stretch(shift(side(WRIST_S, up), sx * FX, 0), dys, A0, A1)
        F.path("body", sleeve, TUNIC, d=ub)
        F.path("body", bracer, LEATHER, smooth=False, sw=3, d=ub)
        F.path("body", wrist, SKIN, smooth=False, sw=3, d=ub)
        F.path("chest", clamp_bottom(sleeve, -366 + dys * 0.5), G1, d=ub)
        F.path("hands", bracer, G2, smooth=False, sw=3, d=ub)
        straps = stretch_strokes(Bs(mirror_strokes(S_STRAPS) if up else S_STRAPS), dys, A0, A1)
        fc = (sx * (-135 + FX), -262 + dys + bob)
        d = (sx * -0.469, -0.883)
    fist(F, "body", fc, d, thumb=not up)
    fist(F, "hands", fc, d, G1, thumb=False)
    fsleeve = stretch(Bx(side(ARM_F, up)), dyf, A0, A1)
    fbracer = stretch(Bx(side(BRACER_F, up)), dyf, A0, A1)
    fhand = stretch(Rg(side(HAND_F, up), sx * 130), dyf, A0, A1)
    F.path("body", fsleeve, TUNIC, d=ub)
    F.path("body", fbracer, LEATHER, smooth=False, sw=3, d=ub)
    F.path("body", fhand, SKIN, sw=3, d=ub)
    if not up:
        F.path("body", stretch(Rg(THUMB_F, 130), dyf, A0, A1), SKIN, sw=3, d=ub)
    F.path("chest", clamp_bottom(fsleeve, -366 + dyf * 0.5), G1, d=ub)
    F.path("hands", fbracer, G2, smooth=False, sw=3, d=ub)
    F.path("hands", fhand, G1, sw=3, d=ub)
    F.stamp(stamps["B"], home, ub)
    # The fine lines, over everything.
    F.details()
    for flip, dy in ((False, dyl), (True, dyr)):
        folds, strap, mud, knees = LEG_LINES_BACK if up else LEG_LINES_FRONT
        def leg_lines(s):
            return stretch_strokes(Bs(mirror_strokes(s) if flip else s), dy, L0, L1)
        F.paint(leg_lines(folds), 4, "#1d1512", smooth=True)
        F.paint(leg_lines(strap), 9, "#1a1310")
        F.paint(shift_strokes(leg_lines(strap), 0, -5), 2, "#6b523a")
        if not up:
            F.paint(leg_lines([[(-54, -36)]]), 8, "#8a7540", tip="square")
        F.paint(leg_lines(mud), 22, "#6a5840", preset="spray", opacity=0.45)
        F.paint(leg_lines(knees), 16, "#6a5840", preset="spray", opacity=0.4)
    F.paint(straps, 5, "#1d1512", d=ub)
    if not up:
        F.paint(knuckle_strokes(fc, d), 2.5, OUT)
    if not swing:
        folds = stretch_strokes(Bs(mirror_strokes(S_FOLDS) if up else S_FOLDS), dys, A0, A1)
        F.paint(folds, 3, "#1c2530", smooth=True, d=ub)
        if not up:
            F.paint([[((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)] for a, b in straps], 5, "#8a7540", tip="square", d=ub)
    F.paint(stretch_strokes(Bs(mirror_strokes(F_STRAPS) if up else F_STRAPS), dyf, A0, A1), 5, "#1d1512", d=ub)
    F.paint(stretch_strokes(Bs(mirror_strokes(F_FOLDS) if up else F_FOLDS), dyf, A0, A1), 3, "#1c2530", smooth=True, d=ub)
    fingers = [[(122, -250), (124, -232)], [(133, -248), (134, -230)]] + ([] if up else [[(142, -252), (142, -238)]])
    F.paint(stretch_strokes(Rs(mirror_strokes(fingers) if up else fingers, sx * 130), dyf, A0, A1), 2.5, "#6b4530", d=ub)
    if not up:
        F.paint(stretch_strokes(Bs([[(130, -344)], [(130, -312)]]), dyf, A0, A1), 5, "#8a7540", tip="square", d=ub)
    # What is worn over the body.
    F.path("chest", Bx(GORGET_FB), G3, d=ub)
    if up:
        F.path("head", HELM_BACK, G1, d=ub)
        F.path("head", [(-68, -584), (68, -584), (62, -564), (0, -554), (-62, -564)], G2, smooth=False, sw=3, d=ub)
    else:
        F.path("head", HELM_FRONT, G1, d=ub)
        F.path("head", [(-76, -630), (76, -630), (76, -612), (-76, -612)], G2, smooth=False, sw=3, d=ub)
        F.path("head", [(-9, -614), (9, -614), (8, -572), (-8, -572)], G2, smooth=False, sw=3, d=ub)
    F.path("head", [(-44, -690), (-34, -694), (-34, -664), (-44, -660)], G3, "none", d=ub)
    # The shield is on the free arm: over the body from the front, behind it from the back.
    shield(F, "shield_under" if up else "shield", sx * 122, -330 + dyf + bob)
    sword(F, "weapon_under" if up else "weapon", fc, d)
    if not up:
        fist(F, "grip", fc, d, "#ffffff", knuckles=True)


S_STRAPS = [[(-167, -344), (-114, -344)], [(-163, -312), (-116, -312)]]
S_FOLDS = [[(-160, -400, .2), (-144, -390, 1), (-124, -398, .2)], [(-158, -380, .2), (-142, -374, .8), (-122, -380, .2)]]
F_STRAPS = [[(107, -344), (156, -344)], [(109, -312), (153, -312)]]
F_FOLDS = [[(114, -400, .2), (132, -390, 1), (150, -398, .2)], [(112, -380, .2), (130, -374, .8), (148, -380, .2)]]
# For the leg on our left: folds at the knee, the strap at the ankle, mud on the boot, mud on the knee.
LEG_LINES_FRONT = ([[(-80, -178, .2), (-60, -168, 1), (-38, -176, .2)], [(-74, -156, .2), (-56, -150, .8), (-40, -156, .2)]], [[(-83, -38), (-27, -34)]], [[(-86, 0), (-60, 10), (-26, 4)]], [[(-70, -160), (-55, -155)]])
LEG_LINES_BACK = ([[(-80, -176, .2), (-60, -184, 1), (-38, -178, .2)], [(-74, -156, .2), (-56, -162, .8), (-40, -158, .2)]], [[(-83, -38), (-29, -36)]], [[(-82, 2), (-56, 8), (-30, 2)]], [[(-66, -120), (-52, -114)]])


# ---- the side, facing right ---------------------------------------------

LEG_S = [(-34, -310), (40, -310), (40, -240), (34, -170), (30, -110), (26, -60), (-22, -60), (-24, -110), (-26, -170), (-34, -240)]
BOOT_S = [(-28, -135), (34, -135), (32, -80), (32, -46), (52, -30), (76, -16), (84, -2), (78, 8), (-24, 8), (-32, -2), (-30, -40), (-27, -90)]
CUFF_S = [(-33, -142), (2, -139), (38, -142), (36, -112), (2, -109), (-31, -112)]
# A stride: the leg that reaches and the leg that pushes off.
LEG_REACH = [(-32, -285.8), (32, -314.2), (85.5, -179.3), (106.7, -71.2), (62.1, -60), (34.3, -156.5)]
BOOT_REACH = [(36.5, -129.4), (96.7, -144.4), (104, -106), (109.6, -71.9), (138, -58), (164, -46), (170, -32), (160, -22), (104, -6), (68, 0), (54, -10), (59.2, -59.3), (46, -96)]
CUFF_REACH = [(32.1, -134.5), (98.1, -150.9), (104.4, -121.6), (40.4, -105.6)]
LEG_PUSH = [(-34.2, -307.3), (34.2, -292.7), (-1.7, -152.2), (-83.3, -64.8), (-117.5, -95.6), (-56.5, -163.8)]
BOOT_PUSH = [(-74.2, -155.5), (-28.2, -114.1), (-52, -90), (-81.1, -62.8), (-62, -34), (-46, -12), (-50, 4), (-76, 8), (-106, -28), (-136, -64), (-134, -90), (-119.7, -97.6), (-98, -126)]
CUFF_PUSH = [(-72.5, -162), (-21.9, -116.6), (-42.8, -94.9), (-91.8, -139.1)]
TUNIC_S = [(-18, -548), (-48, -520), (-58, -470), (-54, -410), (-44, -350), (-54, -262), (-24, -248), (6, -256), (36, -246), (58, -258), (50, -350), (58, -410), (62, -470), (48, -524), (16, -546)]
TUNIC_STRIDE = [(-18, -548), (-48, -520), (-58, -470), (-54, -410), (-44, -350), (-64, -268), (-32, -250), (4, -260), (42, -248), (72, -262), (52, -350), (58, -410), (62, -470), (48, -524), (16, -546)]
MANTLE_S = [(-38, -548), (-62, -520), (-72, -478), (-58, -456), (-38, -466), (-14, -448), (10, -462), (36, -446), (56, -458), (70, -478), (58, -524), (30, -548), (0, -556)]
GORGET_S = [(-38, -546), (-62, -520), (-72, -478), (-58, -456), (-38, -466), (-14, -448), (10, -462), (36, -446), (56, -458), (70, -478), (58, -518), (30, -528), (0, -534)]
ARM_NEAR = [(-24, -520), (26, -520), (30, -450), (30, -400), (46, -340), (54, -296), (18, -286), (8, -330), (-12, -380), (-22, -450)]
BRACER_NEAR = [(-7.6, -359.3), (38.4, -372.3), (55, -302.4), (14.6, -291)]
WRIST_NEAR = [(16, -296), (54, -304), (58, -282), (24, -272)]
ARM_NEAR_FWD = [(-22.5, -500), (-10, -522), (18, -524), (26.5, -510), (47.5, -402.8), (81.9, -326.9), (46.1, -309.1), (0.5, -393.2)]
BRACER_NEAR_FWD = [(8.8, -374.8), (51.8, -396.2), (78.6, -335.9), (41, -317.1)]
WRIST_NEAR_FWD = [(42, -320), (78, -338), (88, -312), (54, -298)]
HELM_S = [(52, -640), (48, -680), (24, -706), (-6, -714), (-38, -706), (-64, -682), (-72, -640), (-68, -596), (-52, -566), (-28, -576), (-18, -610), (-4, -634), (22, -638)]

SIDE = {
    "idle": dict(u=(0, 0), near="stand", far="stand", arm="hang"),
    "step_a": dict(u=(6, 10), near="push", far="reach", arm="fwd", far_arm=((-4, -500), (-40, -400), (-70, -320), (-80, -296))),
    "pass": dict(u=(0, -4), near="stand", far="lift", arm="hang"),
    "step_b": dict(u=(6, 10), near="reach", far="push", arm=((2, -505), (-28, -402), (-56, -324), (-64, -298), (0.2, -0.98)), far_arm=((-4, -500), (34, -400), (66, -322), (76, -296))),
    "work_a": dict(u=(-4, 0), near="stand", far="stand", over=True, arm=((2, -505), (-52, -560), (-80, -636), (-92, -662), (-0.75, -0.661))),
    "work_b": dict(u=(8, 6), near="stand", far="stand", arm=((2, -505), (50, -458), (90, -428), (112, -418), (0.743, 0.669))),
}
SIDE_LEGS = {
    "stand": (LEG_S, BOOT_S, CUFF_S, [[(-18, -178, .2), (4, -168, 1), (28, -176, .2)], [(-14, -156, .2), (6, -150, .8), (24, -156, .2)]], [(-29, -40), (33, -38)], [[(-24, -2), (20, 2), (70, -2)]]),
    "reach": (LEG_REACH, BOOT_REACH, CUFF_REACH, [[(40, -170, .2), (60, -160, 1), (82, -170, .2)]], [(58.2, -59.1), (110.6, -72.1)], [[(70, -8), (110, -12), (160, -30)]]),
    "push": (LEG_PUSH, BOOT_PUSH, CUFF_PUSH, [[(-52, -168, .2), (-30, -156, 1), (-8, -160, .2)], [(-50, -148, .2), (-32, -138, .8), (-14, -142, .2)]], [(-120.4, -98.3), (-80.4, -62.1)], [[(-110, -40), (-80, 0), (-52, -4)]]),
}
SIDE_LEGS["lift"] = tuple(shift(p, 28, -8) if i < 3 else p for i, p in enumerate(SIDE_LEGS["stand"]))


def stamp_side(F):
    F.path("B", MANTLE_S, MANTLE)
    F.path("B", [(-20, -570), (18, -566), (22, -538), (-22, -540)], "#8e6549", smooth=False, sw=3)
    F.path("B", [(-30, -552), (0, -542), (30, -548), (34, -528), (0, -520), (-32, -530)], "#3d2f24", sw=3)
    F.path("B", [(0, -682), (34, -674), (48, -650), (53, -624), (51, -610), (67, -584), (56, -574), (57, -564), (52, -558), (55, -552), (50, -541), (36, -533), (12, -540), (-10, -552), (-40, -560), (-56, -600), (-58, -640), (-40, -672)], FACE)
    F.path("B", [(46, -650), (40, -676), (20, -696), (-6, -703), (-34, -696), (-56, -676), (-65, -640), (-60, -600), (-46, -572), (-30, -580), (-24, -606), (-14, -626), (2, -628), (8, -606), (16, -612), (22, -640)], HAIR, sw=3)
    F.path("B", [(-14, -618), (0, -621), (3, -603), (-7, -588), (-16, -600)], FACE, sw=3)
    F.path("B", [(20, -600), (30, -605), (40, -600), (30, -596)], "#b9ad98", "#1c1512", 2.5)
    F.path("B", [(33, -604), (37, -600), (33, -596), (29, -600)], "#33251a", "none")
    F.details("B")
    F.paint([[(-64, -486), (-52, -470), (-38, -480), (-14, -462), (10, -476), (36, -460), (54, -472), (64, -488)]], 3.5, "#7a6248", spacing=2, smooth=True)
    F.paint([[(-30, -520), (0, -510)], [(20, -500), (44, -496)]], 18, "#6f5a44", preset="spray", opacity=0.3)
    F.paint([[(30, -534)]], 11, "#77726a")
    F.paint([[(30, -534)]], 5, "#1a1512")
    F.paint([[(28, -537)]], 2.5, "#d8d4cc")
    F.paint([[(16, -612, .5), (30, -617, 1), (46, -612, .45)]], 6, "#231915", smooth=True)
    F.paint([[(19, -601), (30, -606), (41, -601)]], 3, "#17110f", smooth=True)
    F.paint([[(32, -602)]], 2.5, "#e8e0d0")
    F.paint([[(22, -592), (32, -590), (40, -593)]], 2, "#8b6248", smooth=True)
    F.paint([[(50, -584), (48, -577), (55, -573)]], 3, "#6d4631", smooth=True)
    F.paint([[(57, -575)]], 3, "#4a2c24")
    F.paint([[(55, -558), (45, -557), (38, -553)]], 3.5, "#4a2c24", smooth=True)
    F.paint([[(30, -634), (35, -610), (35, -590)]], 3, "#d3a78f", smooth=True)
    F.paint([[(-10, -612), (-4, -606), (-8, -596)]], 2.5, "#8b6248", smooth=True)
    F.paint([[(30, -672, .2), (4, -690, 1), (-30, -684, .3)], [(36, -656, .2), (8, -676, 1), (-24, -672, .3)], [(-40, -660, .3), (-54, -630, 1), (-52, -596, .3)], [(-10, -660, .3), (-34, -644, 1), (-44, -612, .3)]], 3, "#4f3d30", smooth=True)
    F.paint([[(20, -664, .3), (-6, -680, 1), (-36, -668, .3)], [(-22, -650, .3), (-40, -626, 1), (-42, -592, .3)]], 3, "#150f0d", smooth=True)
    F.paint([[(-10, -700, 1), (-16, -716, .2)], [(14, -698, 1), (18, -712, .2)], [(-36, -692, 1), (-46, -704, .2)], [(-62, -640, 1), (-72, -646, .2)], [(-56, -590, 1), (-64, -580, .2)]], 5, HAIR)


def side(F, col, stamps):
    pose = SIDE[col]
    u = pose["u"]
    home = cell_origin("side", "idle")
    stride = pose["near"] in ("push", "reach")
    F.ellipse("body", 14 if stride else 8, 6, 170 if stride else 112, 30, SHADOW)
    lines = []
    # The far arm shows only when it swings clear of the body, and the shield is on it.
    if "far_arm" in pose:
        S, E, W, H = pose["far_arm"]
        sleeve, bracer, _ = bent_arm(S, E, W)
        F.path("body", sleeve, FAR_TUNIC, d=u)
        F.path("body", bracer, FAR_LEATHER, smooth=False, sw=3, d=u)
        F.ellipse("body", H[0] + u[0], H[1] + u[1], 21, 25, FAR_SKIN, OUT, 3)
        F.path("chest", sleeve, G2, d=u)
        F.path("hands", bracer, GD, smooth=False, sw=3, d=u)
        F.ellipse("hands", H[0] + u[0], H[1] + u[1], 21, 25, G2, OUT, 3)
        shield_at = (W[0] + u[0], W[1] + u[1] - 20)
    else:
        shield_at = (26 + u[0], -330 + u[1])
    shield(F, "shield_under", *shield_at)
    # Legs: the far one, then the near one.
    for which, far in ((pose["far"], True), (pose["near"], False)):
        leg, boot, cuff, folds, strap, mud = SIDE_LEGS[which]
        d = (-16, -6) if far and which in ("stand", "lift") else ((-8, -4) if far else (0, 0))
        F.path("body", leg, FAR_TROUSER if far else TROUSER, d=d)
        F.path("body", boot, FAR_BOOT if far else BOOT, d=d)
        F.path("body", cuff, FAR_CUFF if far else CUFF, smooth=False, d=d)
        F.path("legs", clamp_top(shift(leg, *d), -250 + u[1]), G2 if far else G1)
        F.path("feet", boot, G2 if far else G1, d=d)
        F.path("feet", cuff, GD if far else G2, smooth=False, d=d)
        if not far:
            lines = (folds, strap, mud)
    # Torso.
    tunic = TUNIC_STRIDE if stride else TUNIC_S
    F.path("body", tunic, TUNIC, d=u)
    F.path("body", [(12, -462), (30, -466), (62, -396), (52, -382)], "#3a2a1e", smooth=False, sw=3, d=u)
    F.path("body", [(-46, -364), (52, -354), (53, -326), (-47, -336)], DARKL, smooth=False, sw=3, d=u)
    F.path("body", [(44, -340), (56, -338), (58, -292), (48, -290)], DARKL, smooth=False, sw=3, d=u)
    F.path("body", [(44, -359), (56, -358), (56, -325), (44, -326)], "#2a1d15", "#8a7540", 6, False, d=u)
    F.path("chest", tunic, G1, d=u)
    F.path("chest", [(-46, -364), (52, -354), (53, -326), (-47, -336)], G2, smooth=False, sw=3, d=u)
    # The near arm, which holds the sword.
    arm = pose["arm"]
    if arm == "hang":
        sleeve, bracer, wrist = ARM_NEAR, BRACER_NEAR, WRIST_NEAR
        fc, d = (40, -274), (0.349, -0.937)
        straps = [[(-1.3, -340.3), (42.9, -352.7)], [(8.8, -310.2), (50, -321.8)]]
        folds = [[(-14, -400, .2), (6, -392, 1), (24, -402, .2)], [(-10, -420, .2), (8, -414, .8), (26, -422, .2)]]
        gsleeve = clamp_bottom(sleeve, -360)
    elif arm == "fwd":
        sleeve, bracer, wrist = ARM_NEAR_FWD, BRACER_NEAR_FWD, WRIST_NEAR_FWD
        fc, d = (72, -296), (0.30, -0.954)
        straps = [[(18, -358.7), (59.2, -379.3)], [(32.3, -333.4), (70.7, -352.6)]]
        folds = [[(2, -404, .2), (22, -396, 1), (44, -408, .2)], [(6, -424, .2), (24, -418, .8), (42, -428, .2)]]
        gsleeve = tube([(2, -505), (24, -398), (36, -374)], [26, 24, 22])
    else:
        S, E, W, fc, d = arm
        sleeve, bracer, straps = bent_arm(S, E, W)
        wrist, folds, gsleeve = None, [], sleeve
    fc = (fc[0] + u[0], fc[1] + u[1])

    def near_arm():
        F.path("body", sleeve, TUNIC, d=u)
        F.path("body", bracer, LEATHER, smooth=False, sw=3, d=u)
        if wrist:
            F.path("body", wrist, SKIN, smooth=False, sw=3, d=u)
        fist(F, "body", fc, d)

    # Drawn back to strike, the arm is in front of the head and the mantle; otherwise under them.
    over = pose.get("over", False)
    if not over:
        near_arm()
        F.path("chest", gsleeve, G1, d=u)
    F.stamp(stamps["B"], home, u)
    F.path("chest", GORGET_S, G3, d=u)
    if over:
        near_arm()
        F.path("chest", gsleeve, G1, d=u)
    F.path("hands", bracer, G2, smooth=False, sw=3, d=u)
    fist(F, "hands", fc, d, G1, thumb=False)
    F.path("head", HELM_S, G1, d=u)
    F.path("head", [(20, -642), (56, -636), (60, -618), (24, -622)], G2, smooth=False, sw=3, d=u)
    F.path("head", [(-30, -690), (-20, -694), (-22, -664), (-32, -660)], G3, "none", d=u)
    sword(F, "weapon", fc, d)
    fist(F, "grip", fc, d, "#ffffff", knuckles=True)
    # The fine lines.
    F.details()
    folds_l, strap, mud = lines
    F.paint(folds_l, 4, "#1d1512", smooth=True)
    F.paint([strap], 9, "#1a1310")
    F.paint([shift(strap, -1, -5)], 2, "#6b523a")
    F.paint([[((strap[0][0] + strap[1][0]) / 2, (strap[0][1] + strap[1][1]) / 2)]], 8, "#8a7540", tip="square")
    F.paint(mud, 22, "#6a5840", preset="spray", opacity=0.45)
    F.paint([[(-52, -328), (-56, -268)], [(-24, -328), (-28, -264)]], 3, "#2b3544", opacity=0.7, d=u)
    if arm in ("hang",):
        F.paint([[(-2, -292)]], 24, "#52627a", tip="square", rotation=8, opacity=0.5, d=u)
    F.paint(folds, 3, "#1c2530", smooth=True, d=u)
    F.paint(straps, 5, "#1d1512", d=u)
    F.paint([[((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)] for a, b in straps], 5, "#8a7540", tip="square", d=u)
    F.paint(knuckle_strokes(fc, d), 2.5, OUT)


# ---- what to print ------------------------------------------------------

SHEETS = ["shield_under", "weapon_under", "body", "legs", "feet", "chest", "hands", "head", "shield", "weapon", "grip"]


def ids():
    with open(os.path.join(HERE, "ids.json")) as f:
        return json.load(f)


def main(argv):
    b = Batch()
    what = argv[1]
    if what == "setup":
        for sheet in SHEETS + ["stamps"]:
            b.add("layer_add", {"kind": "group", "name": sheet}, sheet)
            if sheet in ("body", "grip"):
                b.add("layer_set", {"layer_id": "@" + sheet, "blend_mode": "normal"})
                b.add("layer_add", {"kind": "group", "name": "frames", "into": "@" + sheet}, sheet + "_frames")
    elif what == "stamps":
        row = argv[2]
        F = Frame(b, row, "idle", {"A": ids()["stamps_group"], "B": ids()["stamps_group"]})
        if row == "side":
            stamp_side(F)
        else:
            stamps_front(F, row == "up")
    elif what == "frame":
        row, col = argv[2], argv[3]
        known = ids()
        F = Frame(b, row, col, known["sheets"])
        if row == "side":
            side(F, col, known["stamps"]["side"])
        else:
            front(F, col, row == "up", known["stamps"][row])
    elif what == "grade":
        top = ids()["graded"]["body"]
        b.add("adjust_layer_add", {"type": "hue_saturation", "into": top, "name": "grade", "saturation": -14})
        b.add("adjust_layer_add", {"type": "brightness_contrast", "into": top, "name": "contrast", "contrast": 8})
    b.dump()


if __name__ == "__main__":
    main(sys.argv)
