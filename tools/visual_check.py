#!/usr/bin/env python3
"""Compare deterministic Rust framebuffers with checked-in golden images.

The goldens detect regressions in this port; they are not captures from a NES.
Use --record only after reviewing the generated PNGs.
"""
from pathlib import Path
import argparse, gzip, struct, subprocess, zlib

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target' / 'screenshots'
GOLDEN = ROOT / 'tests' / 'visual'

def png(ppm: bytes) -> bytes:
    header = b'P6\n256 240\n255\n'
    if not ppm.startswith(header) or len(ppm) != len(header) + 256 * 240 * 3:
        raise ValueError('unexpected PPM format')
    pixels = ppm[len(header):]
    raw = b''.join(b'\x00' + pixels[y * 768:(y + 1) * 768] for y in range(240))
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 256, 240, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--record', action='store_true')
    args = parser.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    GOLDEN.mkdir(parents=True, exist_ok=True)
    subprocess.run(['cargo', 'run', '--quiet', '--locked', '-p', 'tetris_nes_core', '--example', 'render_screens', '--', str(OUT)], cwd=ROOT, check=True)
    failed = False
    for frame in sorted(OUT.glob('*.ppm')):
        actual = frame.read_bytes()
        (OUT / (frame.stem + '.png')).write_bytes(png(actual))
        expected = GOLDEN / (frame.stem + '.ppm.gz')
        if args.record:
            expected.write_bytes(gzip.compress(actual, mtime=0))
        elif not expected.exists() or gzip.decompress(expected.read_bytes()) != actual:
            print(f'FAIL visual: {frame.stem}')
            failed = True
        else:
            print(f'PASS visual: {frame.stem}')
    if failed: raise SystemExit(1)

if __name__ == '__main__': main()
