#!/usr/bin/env python3
"""从 .NET 工程的 AppIcon.ico 生成 Rust 侧的窗口图标资源。

用法（在仓库根目录）：

    python src-rs/scripts/gen-icon-asset.py

为什么需要转换：`src/JekyllPostTool.App/Assets/AppIcon.ico` 的七个条目全是 PNG 压缩，
而 `CreateIconFromResourceEx` 在本机对 PNG 条目直接返回 NULL。Rust 侧改成自己摆像素
（`CreateDIBSection` + `CreateIconIndirect`），资源就得是免解码的 32bpp DIB。

两个必须留意的细节：
- 逐帧取原生尺寸，不要拿某一帧缩放。原件的 16/24/32 是无底色的独立手绘帧，
  48 以上才带紫色圆角底，靠缩放复现会让小图标多出一层底；
- 只保留 16/24/32/48/64 五档：256px 的 DIB 条目一个就要 270KB，caption 与任务栏用不到。
"""

import io
import struct
from pathlib import Path

from PIL import Image

REPO = Path(__file__).resolve().parents[2]
SOURCE = REPO / "src/JekyllPostTool.App/Assets/AppIcon.ico"
TARGET = REPO / "src-rs/crates/jp-app/assets/AppIcon.ico"
SIZES = [16, 24, 32, 48, 64]


def source_frames(raw: bytes) -> dict:
    """按尺寸取出原件的每一帧（PNG 条目交给 PIL 解，DIB 条目自己拼）。"""
    count = struct.unpack("<H", raw[4:6])[0]
    frames = {}
    for index in range(count):
        record = raw[6 + index * 16 : 22 + index * 16]
        px = record[0] or 256
        size, offset = struct.unpack("<II", record[8:16])
        blob = raw[offset : offset + size]
        if blob[:4] == b"\x89PNG":
            frames[px] = Image.open(io.BytesIO(blob)).convert("RGBA")
        else:
            header = struct.unpack("<IiiHHIIiiII", blob[:40])
            body = blob[40 : 40 + px * px * 4]
            image = Image.frombytes("RGBA", (px, px), body, "raw", "BGRA", 0, -1)
            if header[2] > 0:  # biHeight = 2 * 高：自底向上存储
                image = image.transpose(Image.FLIP_TOP_BOTTOM)
            frames[px] = image
    return frames


def image_entry(img: Image.Image, px: int) -> bytes:
    rgba = img.tobytes()
    stride = px * 4
    xor = bytearray()
    for row in range(px - 1, -1, -1):  # DIB 自底向上
        line = rgba[row * stride : (row + 1) * stride]
        for col in range(px):
            r, g, b, a = line[col * 4 : col * 4 + 4]
            xor += bytes((b, g, r, a))
    mask_stride = ((px + 31) // 32) * 4
    mask = bytes(mask_stride * px)  # 全零 AND 掩码：透明度交给 alpha 通道
    header = struct.pack("<IiiHHIIiiII", 40, px, px * 2, 1, 32, 0, len(xor), 0, 0, 0, 0)
    return header + bytes(xor) + mask


def main() -> None:
    frames = source_frames(SOURCE.read_bytes())
    entries = []
    for px in SIZES:
        img = frames.get(px)
        if img is None:
            raise SystemExit(f"原件缺少 {px}px 帧，请确认 {SOURCE}")
        if img.size != (px, px):
            img = img.resize((px, px), Image.LANCZOS)
        entries.append((px, image_entry(img, px)))

    offset = 6 + 16 * len(entries)
    directory = b""
    body = b""
    for px, data in entries:
        byte = 0 if px >= 256 else px
        directory += struct.pack("<BBBBHHII", byte, byte, 0, 0, 1, 32, len(data), offset)
        body += data
        offset += len(data)

    TARGET.parent.mkdir(parents=True, exist_ok=True)
    TARGET.write_bytes(struct.pack("<HHH", 0, 1, len(entries)) + directory + body)
    print(f"写出 {TARGET}（{len(entries)} 档，{TARGET.stat().st_size} 字节）")


if __name__ == "__main__":
    main()
