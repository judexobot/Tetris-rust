#!/usr/bin/env python3
"""Reproduce the encoded, raw NES graphical assets from TetrisNESDisasm-master.zip.
Only needed if replacing assets. Requires Pillow (`pip install pillow`).
Usage: python tools/extract_assets.py /path/to/TetrisNESDisasm-master.zip
The resulting data are user-supplied Nintendo tiles and nametables, not libre graphics.
"""
from pathlib import Path
from zipfile import ZipFile
from io import BytesIO
from PIL import Image
import sys

OUTPUT = Path(__file__).resolve().parents[1] / 'core' / 'assets'

def parse_asm(buf:bytes):
    result=[]
    stack=[]
    active=True
    for source in buf.decode('utf8').splitlines():
        line=source.split(';')[0].strip()
        if line.startswith('.if ') or line.startswith('.ifdef '):
            enabled=False if ('NWC' in line and '= 1' in line) else True
            stack.append((active,enabled))
            active=active and enabled
            continue
        if line.startswith('.else'):
            if stack:
                outer,enabled=stack[-1]
                active=outer and not enabled
            continue
        if line.startswith('.endif'):
            if stack:
                active=stack.pop()[0]
            continue
        if active and line.startswith('.byte'):
            for field in line[5:].split(','):
                item=field.strip()
                if item.startswith('$'):
                    result.append(int(item[1:],16))
                elif item.isdecimal():
                    result.append(int(item))
                else:
                    raise ValueError('Unsupported asm .byte: '+item)
    return bytes(result)

def map_from_packets(raw:bytes):
    mem=bytearray([0xff]*1024)
    pos=0
    while pos<len(raw):
        if raw[pos]>=0x80:break
        hi,lo,countinfo=raw[pos:pos+3]
        pos+=3
        count=(countinfo&63) or 64
        addr=(hi<<8)|lo
        inc=32 if countinfo&128 else 1
        if countinfo&64:
            data=[raw[pos]]*count
            pos+=1
        else:
            data=raw[pos:pos+count]
            pos+=count
        for value in data:
            mem[(addr-0x2000)%1024]=value
            addr+=inc
    return bytes(mem)


def main(archive:Path):
    OUTPUT.mkdir(parents=True,exist_ok=True)
    with ZipFile(archive) as z:
        base=next(n for n in z.namelist() if n.endswith('/main.asm')).removesuffix('main.asm')
        for name in ('game','title_menu','typeA_ending','typeB_ending'):
            im=Image.open(BytesIO(z.read(base+f'gfx/{name}_tileset.png'))).convert('L')
            if im.size!=(128,128):raise ValueError('Unexpected atlas size')
            px=list(im.getdata())
            tilepixels=[]
            for tile in range(256):
                x0=tile%16*8;y0=tile//16*8
                for y in range(8):
                    for x in range(8):
                        tilepixels.append(min(3,round(px[(y0+y)*128+x0+x]/85)))
            (OUTPUT/f'{name}.raw').write_bytes(bytes(tilepixels))
        for name in ('game','game_type_menu','level_menu','type_b_ending',
                     'type_b_lvl9_ending','type_a_ending','high_scores','enter_high_score'):
            raw=z.read(base+f'gfx/nametables/{name}_nametable.bin')
            (OUTPUT/f'{name}.map').write_bytes(map_from_packets(raw))
        raw=parse_asm(z.read(base+'gfx/nametables/title_screen_nametable.asm'))
        (OUTPUT/'title_screen.map').write_bytes(map_from_packets(raw))
    print(f'Assets recreated in {OUTPUT}')

if __name__=='__main__':
    if len(sys.argv)!=2:
        raise SystemExit('Usage: python tools/extract_assets.py TetrisNESDisasm-master.zip')
    main(Path(sys.argv[1]))
