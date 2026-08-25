#!/usr/bin/env python3
"""从本目录下的 SVG 图标源生成 WinUI 应用所需的全部图标资源。

用法:
    python build_icons.py            # 写入 src/JekyllPostTool.App/Assets/
    python build_icons.py --preview  # 仅输出预览拼图到临时目录，不写入项目

依赖: pillow、numpy（pip install pillow numpy）

产物清单:
    AppIcon.ico                                        多尺寸 ICO（16/24/32 用任务栏简化版）
    Square44x44Logo.scale-200.png                      88x88 桌面版
    Square150x150Logo.scale-200.png                    300x300 桌面版
    Square44x44Logo.targetsize-24_altform-unplated.png 24x24 任务栏彩色版
    Square44x44Logo.targetsize-48_altform-lightunplated.png 48x48 任务栏单色版
    LockScreenLogo.scale-200.png                       48x48 桌面版
    StoreLogo.png                                      50x50 桌面版
    SplashScreen.scale-200.png                         1240x600 品牌渐变底 + 居中图标
    Wide310x150Logo.scale-200.png                      620x300 品牌渐变底 + 居中图标
"""

import os
import struct
import sys
import tempfile

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.abspath(__file__))
APP_ASSETS = os.path.normpath(os.path.join(ROOT, "..", "src", "JekyllPostTool.App", "Assets"))

S = 4  # 桌面版超采样倍数（256 -> 1024 渲染后缩回）
ST = 16  # 32x32 任务栏版超采样倍数


def hexa(h, a=255):
    """'#RRGGBB' + 可选 alpha -> (r,g,b,a)。"""
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5)) + (a,)


def gradient_image(size, stops, p1, p2):
    """线性渐变（SVG linearGradient 语义）：stops=[(offset,(r,g,b,a)),...]，
    p1/p2 为渐变轴两端点（像素坐标）。返回 RGBA Image。"""
    w, h = size
    xs, ys = np.meshgrid(np.arange(w, dtype=np.float64),
                         np.arange(h, dtype=np.float64))
    ax, ay = p2[0] - p1[0], p2[1] - p1[1]
    t = ((xs - p1[0]) * ax + (ys - p1[1]) * ay) / (ax * ax + ay * ay)
    t = np.clip(t, 0.0, 1.0)
    offs = np.array([o for o, _ in stops])
    cols = np.array([c for _, c in stops], dtype=np.float64)
    out = np.zeros((h, w, 4), dtype=np.uint8)
    for ch in range(4):
        out[..., ch] = np.interp(t, offs, cols[:, ch]).round().astype(np.uint8)
    return Image.fromarray(out, "RGBA")


def rounded_rect_mask(size, box, radius):
    m = Image.new("L", size, 0)
    ImageDraw.Draw(m).rounded_rectangle(box, radius=radius, fill=255)
    return m


def apply_mask(layer, mask):
    layer.putalpha(ImageChops.multiply(layer.getchannel("A"), mask))
    return layer


def over(base, layer):
    return Image.alpha_composite(base, layer)


def round_line(draw, p1, p2, width, fill):
    """带圆形端帽的线段（对应 SVG line + stroke-linecap:round）。"""
    draw.line([p1, p2], fill=fill, width=int(width))
    r = width / 2
    for cx, cy in (p1, p2):
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=fill)


def tinted_shadow(alpha_L, sigma, color, opacity, offset):
    """由图层 alpha 生成 feDropShadow 风格投影。"""
    blur = alpha_L.filter(ImageFilter.GaussianBlur(sigma))
    a = blur.point(lambda v: int(v * opacity))
    img = Image.new("RGBA", alpha_L.size, color[:3] + (0,))
    img.putalpha(a)
    return img.transform(img.size, Image.AFFINE,
                         (1, 0, -offset[0], 0, 1, -offset[1]),
                         resample=Image.BICUBIC)


# ---------------------------------------------------------------- 桌面版 256
BASE_STOPS = [(0.0, hexa("#6366F1")), (0.5, hexa("#8B5CF6")), (1.0, hexa("#A855F7"))]
HI_STOPS = [(0.0, (255, 255, 255, 46)), (0.5, (255, 255, 255, 0)), (1.0, (255, 255, 255, 0))]
PEN_BODY_STOPS = [(0.0, hexa("#475569")), (0.5, hexa("#1E293B")), (1.0, hexa("#334155"))]
PEN_NIB_STOPS = [(0.0, hexa("#CBD5E1")), (0.6, hexa("#94A3B8")), (1.0, hexa("#64748B"))]


def _pen_layer_desktop(w):
    """桌面版钢笔，局部坐标绘制（原点=笔尖触纸点）。"""
    pad = 6 * S
    x0, y0, x1, y1 = -pad, -pad, 126 * S + pad, pad  # 覆盖 x[-3.5,123] y[-9,9]
    lay = Image.new("RGBA", (int(x1 - x0), int(y1 - y0)), (0, 0, 0, 0))
    d = ImageDraw.Draw(lay)

    def L(x, y):  # 局部坐标 -> 图层像素
        return ((x - x0 / S) * S, (y - y0 / S) * S)

    # 笔尖三角：水平渐变填充
    tri_box = (L(0, -8)[0], L(0, -8)[1], L(16, 8)[0], L(16, 8)[1])
    tri_grad = gradient_image((int(tri_box[2] - tri_box[0]), int(tri_box[3] - tri_box[1])),
                              PEN_NIB_STOPS, (0, 0), (tri_box[2] - tri_box[0], 0))
    tri_mask = Image.new("L", tri_grad.size, 0)
    ImageDraw.Draw(tri_mask).polygon(
        [L(0, 0), L(16, -8), L(16, 8)], fill=255)
    lay.paste(tri_grad, (int(tri_box[0]), int(tri_box[1])), tri_mask)

    # 笔尖通气缝
    round_line(d, L(0, 0), L(14, 0), 0.8 * S, hexa("#475569"))
    # 笔尖环
    d.rounded_rectangle([L(14, -9), L(24, 9)], radius=2 * S, fill=hexa("#334155"))
    # 笔杆：垂直渐变
    body_box = (L(22, -6)[0], L(22, -6)[1], L(112, 6)[0], L(112, 6)[1])
    body_grad = gradient_image((int(body_box[2] - body_box[0]), int(body_box[3] - body_box[1])),
                               PEN_BODY_STOPS, (0, 0), (0, body_box[3] - body_box[1]))
    body_mask = rounded_rect_mask(body_grad.size, (0, 0, body_grad.size[0] - 1, body_grad.size[1] - 1), 3 * S)
    lay.paste(body_grad, (int(body_box[0]), int(body_box[1])), body_mask)
    # 笔杆高光
    hi = Image.new("RGBA", lay.size, (0, 0, 0, 0))
    ImageDraw.Draw(hi).rounded_rectangle([L(24, -5), L(110, -2)], radius=1.5 * S,
                                         fill=(255, 255, 255, 31))
    lay = over(lay, hi)
    # 笔帽 + 笔尾装饰 + 墨水点
    d.rounded_rectangle([L(108, -7), L(120, 7)], radius=4 * S, fill=hexa("#1E293B"))
    r = 3 * S
    d.ellipse([L(120, 0)[0] - r, L(120, 0)[1] - r, L(120, 0)[0] + r, L(120, 0)[1] + r],
              fill=hexa("#6366F1"))
    r = 3.5 * S
    d.ellipse([L(0, 0)[0] - r, L(0, 0)[1] - r, L(0, 0)[0] + r, L(0, 0)[1] + r],
              fill=hexa("#1E1B4B"))
    return lay, (x0, y0)


def render_desktop():
    """按 icon-win-desktop.svg 规格，1024px 超采样渲染。"""
    w = 256 * S
    img = Image.new("RGBA", (w, w), (0, 0, 0, 0))

    # 圆角方形底板（渐变）+ 顶部高光叠加
    plate_mask = rounded_rect_mask((w, w), (8 * S, 8 * S, 248 * S, 248 * S), 56 * S)
    img = over(img, apply_mask(gradient_image((w, w), BASE_STOPS, (8 * S, 8 * S), (248 * S, 248 * S)),
                               plate_mask))
    img = over(img, apply_mask(gradient_image((w, w), HI_STOPS, (8 * S, 0), (8 * S, w)),
                               plate_mask))

    # 稿纸组
    paper = Image.new("RGBA", (w, w), (0, 0, 0, 0))
    d = ImageDraw.Draw(paper)
    d.rounded_rectangle([50 * S, 50 * S, 190 * S, 210 * S], radius=12 * S, fill=hexa("#FFF8F0"))
    d.rounded_rectangle([64 * S, 66 * S, 136 * S, 75 * S], radius=4.5 * S, fill=hexa("#64748B"))
    for ya, xb in ((92, 176), (110, 176), (128, 176), (146, 160), (164, 176), (182, 140)):
        round_line(d, (64 * S, ya * S), (xb * S, ya * S), 3 * S, hexa("#E2E8F0"))

    # 稿纸投影（feDropShadow dy=4 std=5 #312E81 20%）
    img = over(img, tinted_shadow(paper.getchannel("A"), 5 * S, hexa("#312E81"), 0.2, (0, 4 * S)))
    img = over(img, paper)

    # 钢笔组：局部层旋转 32°（SVG 顺时针）贴到 (96,90)，再叠其投影
    pen_local, (lx0, ly0) = _pen_layer_desktop(S)
    big = Image.new("RGBA", (w, w), (0, 0, 0, 0))
    big.paste(pen_local, (int(96 * S + lx0), int(90 * S + ly0)))
    pen = big.rotate(-32, center=(96 * S, 90 * S), resample=Image.BICUBIC)
    img = over(img, tinted_shadow(pen.getchannel("A"), 3 * S, hexa("#1E1B4B"), 0.3, (2 * S, 3 * S)))
    img = over(img, pen)

    return img.resize((256, 256), Image.LANCZOS)


# ------------------------------------------------------- 任务栏彩色/单色 32
def render_taskbar_color():
    """icon-win-taskbar.svg：纸稿 + 彩色笔尖，透明背景。"""
    w = 32 * ST
    img = Image.new("RGBA", (w, w), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([6 * ST, 4 * ST, 24 * ST, 28 * ST], radius=2.5 * ST,
                        fill=hexa("#FFF8F0"), outline=hexa("#CBD5E1"), width=max(1, round(0.75 * ST)))
    d.rounded_rectangle([9 * ST, 7 * ST, 18 * ST, 9 * ST], radius=1 * ST, fill=hexa("#64748B"))
    for ya, xb in ((12, 21), (16, 21), (20, 17)):
        round_line(d, (9 * ST, ya * ST), (xb * ST, ya * ST), 1.2 * ST, hexa("#E2E8F0"))
    # 钢笔尖：局部层旋转 35° 贴到 (19,21)
    pad = 2 * ST
    lay = Image.new("RGBA", (16 * ST + 2 * pad, 12 * ST + 2 * pad), (0, 0, 0, 0))
    pd = ImageDraw.Draw(lay)

    def L(x, y):
        return (x * ST + pad, y * ST + pad)

    pd.polygon([L(0, 0), L(8, -4), L(8, 4)], fill=hexa("#64748B"))
    pd.rounded_rectangle([L(7, -4.5), L(12, 4.5)], radius=1 * ST, fill=hexa("#1E293B"))
    big = Image.new("RGBA", (w, w), (0, 0, 0, 0))
    big.paste(lay, (19 * ST - pad, 21 * ST - pad))
    img = over(img, big.rotate(-35, center=(19 * ST, 21 * ST), resample=Image.BICUBIC))
    return img.resize((32, 32), Image.LANCZOS)


def render_taskbar_mono():
    """icon-win-taskbar-mono.svg：深色描边线稿，适配浅色背景。"""
    ink = hexa("#1E293B")
    w = 32 * ST
    img = Image.new("RGBA", (w, w), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([6 * ST, 4 * ST, 24 * ST, 28 * ST], radius=2.5 * ST,
                        outline=ink, width=max(1, round(1.75 * ST)))
    round_line(d, (9 * ST, 9 * ST), (17 * ST, 9 * ST), 2 * ST, ink)
    for ya, xb in ((14, 21), (18, 21), (22, 16)):
        round_line(d, (9 * ST, ya * ST), (xb * ST, ya * ST), 1.75 * ST, ink)
    pad = 2 * ST
    lay = Image.new("RGBA", (14 * ST + 2 * pad, 10 * ST + 2 * pad), (0, 0, 0, 0))
    pd = ImageDraw.Draw(lay)

    def L(x, y):
        return (x * ST + pad, y * ST + pad)

    pd.polygon([L(0, 0), L(7, -3.5), L(7, 3.5)], fill=ink)
    pd.rounded_rectangle([L(6, -4), L(10.5, 4)], radius=1 * ST, fill=ink)
    big = Image.new("RGBA", (w, w), (0, 0, 0, 0))
    big.paste(lay, (19 * ST - pad, 21 * ST - pad))
    img = over(img, big.rotate(-35, center=(19 * ST, 21 * ST), resample=Image.BICUBIC))
    return img.resize((32, 32), Image.LANCZOS)


# ---------------------------------------------------------------- 输出封装
def save_ico(path, entries):
    """手工写多尺寸 ICO（每项内嵌 PNG，Vista+ 支持）。entries=[(size, RGBA Image)]"""
    blobs = []
    for sz, im in entries:
        if im.size != (sz, sz):
            im = im.resize((sz, sz), Image.LANCZOS)
        buf = __import__("io").BytesIO()
        im.save(buf, "PNG")
        blobs.append((sz, buf.getvalue()))
    out = struct.pack("<HHH", 0, 1, len(blobs))
    offset = 6 + 16 * len(blobs)
    for sz, data in blobs:
        b = sz % 256  # 256 编码为 0
        out += struct.pack("<BBBBHHII", b, b, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    with open(path, "wb") as f:
        f.write(out + b"".join(d for _, d in blobs))


def brand_background(w, h):
    """启动画面/宽徽标的品牌渐变底（与底板同色系）。"""
    return gradient_image((w, h), BASE_STOPS, (0, 0), (w, h))


def centered(bg, icon, target):
    ic = icon.resize((target, target), Image.LANCZOS)
    # 轻微投影提升可读性
    sh = tinted_shadow(ic.getchannel("A"), max(4, target // 40), hexa("#312E81"), 0.35,
                       (0, target // 30))
    bg.alpha_composite(sh, ((bg.width - target) // 2, (bg.height - target) // 2))
    bg.alpha_composite(ic, ((bg.width - target) // 2, (bg.height - target) // 2))
    return bg


def main():
    write = "--preview" not in sys.argv
    desktop = render_desktop()
    color = render_taskbar_color()
    mono = render_taskbar_mono()

    products = {
        "Square44x44Logo.scale-200.png": desktop.resize((88, 88), Image.LANCZOS),
        "Square150x150Logo.scale-200.png": desktop.resize((300, 300), Image.LANCZOS),
        "LockScreenLogo.scale-200.png": desktop.resize((48, 48), Image.LANCZOS),
        "StoreLogo.png": desktop.resize((50, 50), Image.LANCZOS),
        "Square44x44Logo.targetsize-24_altform-unplated.png": color.resize((24, 24), Image.LANCZOS),
        "Square44x44Logo.targetsize-48_altform-lightunplated.png": mono.resize((48, 48), Image.LANCZOS),
        "SplashScreen.scale-200.png": centered(brand_background(1240, 600), desktop, 300),
        "Wide310x150Logo.scale-200.png": centered(brand_background(620, 300), desktop, 180),
    }

    if write:
        os.makedirs(APP_ASSETS, exist_ok=True)
        save_ico(os.path.join(APP_ASSETS, "AppIcon.ico"),
                 [(16, color), (24, color), (32, color),
                  (48, desktop), (64, desktop), (128, desktop), (256, desktop)])
        for name, im in products.items():
            im.save(os.path.join(APP_ASSETS, name))
        print(f"已写入 {APP_ASSETS}")
    else:
        # 预览拼图：浅灰底便于观察透明边缘
        names = list(products)
        cell = 320
        sheet = Image.new("RGB", (cell * 3, cell * 3), (235, 237, 240))
        items = [("desktop 256", desktop), ("taskbar-color 32", color), ("taskbar-mono 32", mono)]
        for i, name in enumerate(names):
            im = products[name].copy()
            im.thumbnail((cell - 20, cell - 20))
            items.append((name, im))
        d = ImageDraw.Draw(sheet)
        for i, (label, im) in enumerate(items[:9]):
            x, y = (i % 3) * cell, (i // 3) * cell
            sheet.paste(im, (x + (cell - im.width) // 2, y + (cell - im.height) // 2),
                        im if im.mode == "RGBA" else None)
            d.text((x + 8, y + 4), label, fill=(40, 40, 40))
        prev = os.path.join(tempfile.gettempdir(), "jpt_icons_preview.png")
        sheet.save(prev)
        print(f"预览已生成 {prev}")

    print("完成。")


if __name__ == "__main__":
    main()
