#!/usr/bin/env python3
"""从本目录下的 SVG 图标源生成 WinUI 应用所需的全部图标资源。

用法:
    python build_icons.py            # 写入 src/JekyllPostTool.App/Assets/
    python build_icons.py --preview  # 仅输出预览拼图到临时目录，不写入项目

依赖: resvg-py、pillow（pip install resvg-py pillow）。resvg 为自带二进制的
SVG 栅格化器，Windows 上无需额外原生库。

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

import io
import os
import struct
import sys
import tempfile

import resvg_py
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.abspath(__file__))
APP_ASSETS = os.path.normpath(os.path.join(ROOT, "..", "src", "JekyllPostTool.App", "Assets"))

DESKTOP_SVG = "icon-win-desktop.svg"
TASKBAR_SVG = "icon-win-taskbar.svg"
TASKBAR_MONO_SVG = "icon-win-taskbar-mono.svg"

# 启动画面/宽徽标的品牌渐变底（与桌面版底板同色系），直接以 SVG 表达
_BRAND_BG_SVG = """<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}">
  <defs><linearGradient id="g" gradientUnits="userSpaceOnUse"
      x1="0" y1="0" x2="{w}" y2="{h}">
    <stop offset="0" stop-color="#6366F1"/>
    <stop offset=".5" stop-color="#8B5CF6"/>
    <stop offset="1" stop-color="#A855F7"/>
  </linearGradient></defs>
  <rect width="{w}" height="{h}" fill="url(#g)"/>
</svg>"""


def render(svg_name=None, size=None, svg_text=None):
    """SVG → 指定尺寸的 RGBA Image。矢量源按目标尺寸栅格化，无需超采样。"""
    if svg_text is not None:
        png = resvg_py.svg_to_bytes(svg_string=svg_text, width=size, height=size)
    else:
        png = resvg_py.svg_to_bytes(svg_path=os.path.join(ROOT, svg_name),
                                    width=size, height=size)
    return Image.open(io.BytesIO(bytes(png))).convert("RGBA")


def render_brand_background(w, h):
    return render(svg_text=_BRAND_BG_SVG.format(w=w, h=h))


def centered(bg, icon, target):
    ic = icon.resize((target, target), Image.LANCZOS)
    bg.alpha_composite(ic, ((bg.width - target) // 2, (bg.height - target) // 2))
    return bg


def save_ico(path, entries):
    """手工写多尺寸 ICO（每项内嵌 PNG，Vista+ 支持）；各尺寸可用不同图形。
    entries=[(size, RGBA Image)]"""
    blobs = []
    for sz, im in entries:
        if im.size != (sz, sz):
            im = im.resize((sz, sz), Image.LANCZOS)
        buf = io.BytesIO()
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


def main():
    write = "--preview" not in sys.argv

    desktop256 = render(DESKTOP_SVG, 256)
    taskbar32 = render(TASKBAR_SVG, 32)
    mono48 = render(TASKBAR_MONO_SVG, 48)

    products = {
        "Square44x44Logo.scale-200.png": render(DESKTOP_SVG, 88),
        "Square150x150Logo.scale-200.png": render(DESKTOP_SVG, 300),
        "LockScreenLogo.scale-200.png": render(DESKTOP_SVG, 48),
        "StoreLogo.png": render(DESKTOP_SVG, 50),
        "Square44x44Logo.targetsize-24_altform-unplated.png": render(TASKBAR_SVG, 24),
        "Square44x44Logo.targetsize-48_altform-lightunplated.png": mono48,
        "SplashScreen.scale-200.png": centered(render_brand_background(1240, 600), desktop256, 300),
        "Wide310x150Logo.scale-200.png": centered(render_brand_background(620, 300), desktop256, 180),
    }

    if write:
        os.makedirs(APP_ASSETS, exist_ok=True)
        save_ico(os.path.join(APP_ASSETS, "AppIcon.ico"),
                 [(16, taskbar32), (24, taskbar32), (32, taskbar32),
                  (48, desktop256), (64, render(DESKTOP_SVG, 64)),
                  (128, render(DESKTOP_SVG, 128)), (256, desktop256)])
        for name, im in products.items():
            im.save(os.path.join(APP_ASSETS, name))
        print(f"已写入 {APP_ASSETS}")
    else:
        # 预览拼图：浅灰底便于观察透明边缘
        cell = 320
        sheet = Image.new("RGB", (cell * 3, cell * 3), (235, 237, 240))
        items = [("desktop 256", desktop256), ("taskbar-color 32", taskbar32),
                 ("taskbar-mono 48", mono48)]
        for name, im in products.items():
            im = im.copy()
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
