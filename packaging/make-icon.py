#!/usr/bin/env python3
"""Generate IslandHop's .icns / .ico / source PNG with zero third-party deps.

Usage: python3 packaging/make-icon.py
Draws a rounded-square badge with a light "island" pill, a green status dot and
two text lines. Supersampling-free SDF edges give 1px anti-aliasing.
"""
import math
import os
import struct
import subprocess
import tempfile
import zlib

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
ASSETS = os.path.join(ROOT, "app", "assets")
os.makedirs(ASSETS, exist_ok=True)
SIZE = 1024


def rr(x, y, cx, cy, w, h, r):
    dx = abs(x - cx) - (w / 2 - r)
    dy = abs(y - cy) - (h / 2 - r)
    return math.hypot(max(dx, 0.0), max(dy, 0.0)) + min(max(dx, dy), 0.0) - r


def circle(x, y, cx, cy, rad):
    return math.hypot(x - cx, y - cy) - rad


def cover(d):
    return max(0.0, min(1.0, 0.5 - d))


def over(dst, color, a):
    """Straight-alpha source-over; channel values are 0..255, alpha is 0..1."""
    if a <= 0:
        return dst
    r, g, b, da = dst
    oa = a + da * (1 - a)
    if oa <= 0:
        return (0.0, 0.0, 0.0, 0.0)
    w = da * (1 - a)
    return (
        (color[0] * a + r * w) / oa,
        (color[1] * a + g * w) / oa,
        (color[2] * a + b * w) / oa,
        oa,
    )


def chunk(tag, data):
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)


def write_png(path, size, rows):
    raw = b"".join(b"\x00" + row for row in rows)
    blob = (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(blob)


def draw():
    rows = []
    for y in range(SIZE):
        row = bytearray()
        t = y / SIZE
        c1, c2 = (24, 32, 50), (52, 72, 104)
        base = tuple(c1[i] * (1 - t) + c2[i] * t for i in range(3))
        for x in range(SIZE):
            px = (0.0, 0.0, 0.0, 0.0)
            px = over(px, base, cover(rr(x, y, 512, 512, 1024, 1024, 224)))
            px = over(px, (238, 246, 255), cover(rr(x, y, 512, 500, 600, 236, 118)))
            px = over(px, (76, 209, 160), cover(circle(x, y, 322, 500, 52)))
            px = over(px, (110, 200, 245), cover(circle(x, y, 404, 500, 22)))
            px = over(px, (150, 175, 200), cover(rr(x, y, 690, 452, 200, 26, 13)))
            px = over(px, (150, 175, 200), cover(rr(x, y, 670, 548, 160, 26, 13)))
            row += bytes((
                int(max(0, min(255, px[0]))),
                int(max(0, min(255, px[1]))),
                int(max(0, min(255, px[2]))),
                int(max(0, min(255, px[3] * 255))),
            ))
        rows.append(bytes(row))
    return rows


def sips_resize(src, size, dst):
    subprocess.run(["sips", "-z", str(size), str(size), src, "--out", dst],
                   check=True, capture_output=True)


def make_ico(png, path):
    sizes = [16, 32, 48, 64, 128, 256]
    blobs = []
    with tempfile.TemporaryDirectory() as tmp:
        for s in sizes:
            out = os.path.join(tmp, f"{s}.png")
            sips_resize(png, s, out)
            with open(out, "rb") as f:
                blobs.append(f.read())
    header = struct.pack("<HHH", 0, 1, len(sizes))
    offset = 6 + 16 * len(sizes)
    entries, body = b"", b""
    for s, blob in zip(sizes, blobs):
        b = s if s < 256 else 0
        entries += struct.pack("<BBBBHHII", b, b, 0, 0, 1, 32, len(blob), offset)
        body += blob
        offset += len(blob)
    with open(path, "wb") as f:
        f.write(header + entries + body)


def make_icns(png, path):
    with tempfile.TemporaryDirectory() as tmp:
        iconset = os.path.join(tmp, "AppIcon.iconset")
        os.makedirs(iconset)
        names = {
            "icon_16x16.png": 16, "icon_16x16@2x.png": 32,
            "icon_32x32.png": 32, "icon_32x32@2x.png": 64,
            "icon_128x128.png": 128, "icon_128x128@2x.png": 256,
            "icon_256x256.png": 256, "icon_256x256@2x.png": 512,
            "icon_512x512.png": 512, "icon_512x512@2x.png": 1024,
        }
        for name, size in names.items():
            sips_resize(png, size, os.path.join(iconset, name))
        subprocess.run(["iconutil", "-c", "icns", iconset, "-o", path], check=True, capture_output=True)


def main():
    png = os.path.join(ASSETS, "AppIcon.png")
    print("drawing 1024px master ...")
    write_png(png, SIZE, draw())
    make_ico(png, os.path.join(ASSETS, "AppIcon.ico"))
    make_icns(png, os.path.join(ASSETS, "AppIcon.icns"))
    for name in ["AppIcon.png", "AppIcon.ico", "AppIcon.icns"]:
        p = os.path.join(ASSETS, name)
        print(f"  {name}: {os.path.getsize(p)} bytes")


if __name__ == "__main__":
    main()
