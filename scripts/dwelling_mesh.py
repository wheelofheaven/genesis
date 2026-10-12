"""Small indexed box/bevel authoring helper; all coordinates are metres. CC0."""
import math


def dot(a, b):
    return sum(x*y for x, y in zip(a, b))


def cross(a, b):
    return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])


def unit(a):
    length = math.sqrt(dot(a, a))
    return tuple(x/length for x in a)


def box(mesh, center, size, bevel=0, grain_axis=None, faces=None):
    """Closed outward mesh, with a planar chamfer bounded by the original box."""
    positions, normals, tangents, uvs, indices = mesh
    half = [v/2 for v in size]
    radius = min(bevel, min(half)*.4)
    core = [v-radius for v in half]

    def face(points, normal):
        normal = unit(normal)
        a, b, c = points[:3]
        if dot(cross([b[i]-a[i] for i in range(3)], [c[i]-a[i] for i in range(3)]), normal) < 0:
            points = list(reversed(points))
        # A vertical texture axis on walls; stable horizontal coordinates on roofs.
        up = (0, 1, 0) if abs(normal[1]) < .95 else (0, 0, -1)
        if grain_axis is not None:
            grain = tuple(1 if i == grain_axis else 0 for i in range(3))
            if abs(dot(grain, normal)) < .95:
                up = grain
        tangent = unit(cross(up, normal))
        vertical = cross(normal, tangent)
        start = len(positions)
        for local in points:
            p = [center[i]+local[i] for i in range(3)]
            positions.append(p); normals.append(normal); tangents.append([*tangent, -1])
            uvs.append([dot(p, tangent), -dot(p, vertical)])
        if faces is not None: faces.append((start,len(points)))
        for i in range(1, len(points)-1):
            indices.extend([start, start+i, start+i+1])

    for axis in range(3):
        other = [i for i in range(3) if i != axis]
        for sign in [-1, 1]:
            points = []
            for a, b in [(-1, -1), (1, -1), (1, 1), (-1, 1)]:
                p = [0., 0., 0.]; p[axis] = sign*half[axis]
                p[other[0]] = a*core[other[0]]; p[other[1]] = b*core[other[1]]
                points.append(p)
            n = [0., 0., 0.]; n[axis] = sign
            face(points, n)
    if radius == 0:
        return
    for axis in range(3):
        a, b = [i for i in range(3) if i != axis]
        for sa in [-1, 1]:
            for sb in [-1, 1]:
                points = []
                for end, side in [(-1, 0), (1, 0), (1, 1), (-1, 1)]:
                    p = [0., 0., 0.]; p[axis] = end*core[axis]
                    p[a] = sa*(half[a] if side == 0 else core[a])
                    p[b] = sb*(core[b] if side == 0 else half[b])
                    points.append(p)
                n = [0., 0., 0.]; n[a] = sa; n[b] = sb
                face(points, n)
    for sx in [-1, 1]:
        for sy in [-1, 1]:
            for sz in [-1, 1]:
                signs = [sx, sy, sz]
                points = [[signs[i]*(half[i] if i == axis else core[i]) for i in range(3)] for axis in range(3)]
                face(points, signs)
