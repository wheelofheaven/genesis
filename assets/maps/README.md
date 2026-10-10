# Orbital Earth illustration

`earth-land.bin` is a 1024 × 512 equirectangular land mask, eight samples per
byte (least significant bit first), west to east and north to south. It is
embedded in the application for a cached orbital illustration. It supplies
continental silhouettes only, not simulation terrain, historical coastlines,
vegetation, bathymetry, or a playable planetary mesh. Shading and atmospheric
patterns are decorative.

Made with [Natural Earth](https://www.naturalearthdata.com/), 1:110m land polygons.
Natural Earth data is [public domain](https://www.naturalearthdata.com/about/terms-of-use/).

Source retrieved October 10, 2026:
https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_land.geojson

Source SHA-256: `9e0729ee253ca7d7a5c4ae9395fb1902264c5377c52e224d13dd85010e2835d9`
Mask SHA-256: `73511620aef445be28972f6438f940bb38957e0d4eb11e18b7baec20fc59f2ad`

Rebuild from that GeoJSON using `python3 scripts/build_earth_mask.py <source>`.
