#!/usr/bin/env python3
"""Evidência administrativa CPU dos meshes reais egui; não executa backend gráfico.

Requer numpy e Pillow apenas no ambiente de inspeção, fora do produto. Usa as
posições, recortes, cores e atlas originais exportados por viewport_gallery.
A amostragem bilinear é uma aproximação de rasterização, não uma captura da GPU.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform

import numpy as np
from PIL import Image, __version__ as pillow_version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    source = args.directory / "egui-meshes.json"
    record = json.loads(source.read_text(encoding="utf-8"))
    width, height = record["width"], record["height"]
    scale = record["pixels_per_point"]
    textures = {}
    for texture in record["textures"]:
        tw, th = texture["size"]
        textures[texture["id"]] = np.frombuffer(
            (args.directory / texture["file"]).read_bytes(), dtype=np.uint8
        ).reshape(th, tw, 4).astype(np.float32) / 255.0
    canvas = np.zeros((height, width, 3), dtype=np.float32)
    canvas[:] = [16 / 255, 23 / 255, 28 / 255]
    triangles = 0
    for mesh in record["meshes"]:
        texture = textures[mesh["texture"]]
        th, tw = texture.shape[:2]
        vertices = np.array([[*v[:4], *v[4]] for v in mesh["vertices"]], dtype=np.float64)
        vertices[:, :2] *= scale
        vertices[:, 4:] /= 255.0
        clip = np.array(mesh["clip"]) * scale
        for indexes in np.array(mesh["indices"]).reshape(-1, 3):
            triangle = vertices[indexes]
            a, b, c = triangle[:, :2]
            left = max(0, int(np.floor(max(clip[0], triangle[:, 0].min()))))
            right = min(width, int(np.ceil(min(clip[2], triangle[:, 0].max()))))
            top = max(0, int(np.floor(max(clip[1], triangle[:, 1].min()))))
            bottom = min(height, int(np.ceil(min(clip[3], triangle[:, 1].max()))))
            denominator = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
            if right <= left or bottom <= top or abs(denominator) < 1e-10:
                continue
            yy, xx = np.mgrid[top:bottom, left:right]
            xx, yy = xx + .5, yy + .5
            wa = ((b[1] - c[1]) * (xx - c[0]) + (c[0] - b[0]) * (yy - c[1])) / denominator
            wb = ((c[1] - a[1]) * (xx - c[0]) + (a[0] - c[0]) * (yy - c[1])) / denominator
            wc = 1 - wa - wb
            mask = (wa >= -1e-9) & (wb >= -1e-9) & (wc >= -1e-9)
            if not mask.any():
                continue
            weights = np.stack([wa[mask], wb[mask], wc[mask]], axis=1)
            uv = weights @ triangle[:, 2:4]
            color = weights @ triangle[:, 4:]
            tx = np.clip(uv[:, 0] * tw - .5, 0, tw - 1)
            ty = np.clip(uv[:, 1] * th - .5, 0, th - 1)
            x0, y0 = tx.astype(np.int64), ty.astype(np.int64)
            x1, y1 = np.minimum(x0 + 1, tw - 1), np.minimum(y0 + 1, th - 1)
            fx, fy = (tx - x0)[:, None], (ty - y0)[:, None]
            sampled = ((texture[y0, x0] * (1 - fx) + texture[y0, x1] * fx) * (1 - fy)
                       + (texture[y1, x0] * (1 - fx) + texture[y1, x1] * fx) * fy)
            rgba = sampled * color
            region = canvas[top:bottom, left:right]
            region[mask] = rgba[:, :3] + region[mask] * (1 - rgba[:, 3:4])
            triangles += 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(np.clip(np.round(canvas * 255), 0, 255).astype(np.uint8)).save(args.output)
    metadata = {
        "evidence": record["evidence"], "rasterization": "CPU bilinear approximation of exported meshes",
        "native_backend_executed": False, "web_browser_executed": False,
        "mesh_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "image_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(),
        "width": width, "height": height, "triangles": triangles,
        "scale": record["scale"], "high_contrast": record["high_contrast"],
        "snapshot_sha256": record["view_report"]["snapshot_sha256"],
        "tools": {"python": platform.python_version(), "numpy": np.__version__, "pillow": pillow_version},
        "renderer_script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    args.output.with_suffix(".json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(metadata))


if __name__ == "__main__":
    main()
