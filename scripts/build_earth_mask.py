"""Build the orbital illustration's 1024x512 land mask from Natural Earth GeoJSON.
Usage: python3 scripts/build_earth_mask.py path/to/ne_110m_land.geojson
The source is public-domain cartography; this mask is not simulation terrain.
"""
import json
import math
import pathlib
import sys

WIDTH, HEIGHT = 1024, 512
mask = bytearray(WIDTH * HEIGHT)
data = json.loads(pathlib.Path(sys.argv[1]).read_text())
for feature in data['features']:
    geom = feature['geometry']
    polygons = geom['coordinates'] if geom['type'] == 'MultiPolygon' else [geom['coordinates']]
    for polygon in polygons:
        for index, ring in enumerate(polygon):
            points = [((lon + 180) / 360 * WIDTH, (90 - lat) / 180 * HEIGHT) for lon, lat in ring]
            low = max(0, math.floor(min(p[1] for p in points)))
            high = min(HEIGHT, math.ceil(max(p[1] for p in points)))
            for row in range(low, high):
                y = row + 0.5
                intersections = []
                for a, b in zip(points, points[1:] + points[:1]):
                    if (a[1] > y) != (b[1] > y):
                        intersections.append(a[0] + (y-a[1]) * (b[0]-a[0]) / (b[1]-a[1]))
                intersections.sort()
                for left, right in zip(intersections[::2], intersections[1::2]):
                    start = max(0, math.ceil(left - 0.5))
                    end = min(WIDTH, math.ceil(right - 0.5))
                    for col in range(start, end):
                        mask[row * WIDTH + col] = 1 if index == 0 else 0
# Pack eight samples per byte; the renderer reads the embedded mask directly.
packed = bytearray(len(mask)//8)
for i, value in enumerate(mask):
    packed[i//8] |= value << (i % 8)
pathlib.Path('assets/maps/earth-land.bin').write_bytes(packed)
