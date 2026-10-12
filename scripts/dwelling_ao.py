"""Offline short-range diffuse visibility for the box-authored fixture. CC0.
Conservative box occluders approximate bevels; direct illumination is not baked.
"""
import math
from dwelling_mesh import cross, dot, unit

PAD = 3
REACH = .65
SAMPLES = 24


def visibility(origin, rays, boxes):
    blocked = 0.
    for ray in rays:
        nearest = REACH
        for lo, hi in boxes:
            enter, leave = 0., nearest
            for axis in range(3):
                d = ray[axis]
                if abs(d) < 1e-8:
                    if origin[axis] < lo[axis] or origin[axis] > hi[axis]:
                        leave = -1.; break
                else:
                    a, b = (lo[axis]-origin[axis])/d, (hi[axis]-origin[axis])/d
                    enter, leave = max(enter, min(a,b)), min(leave, max(a,b))
                    if enter > leave: break
            if enter <= leave: nearest = min(nearest, enter)
        blocked += (1-nearest/REACH)**2
    return 1-blocked/len(rays)


def bake(meshes, faces, boxes, size):
    # White corner reserved for bevel triangles and genuinely exposed faces.
    atlas = bytearray([255])*(size*size*4)
    coords = [[[.5/size,.5/size] for _ in m[0]] for m in meshes]
    x, y, row = 8, 8, 0
    baked = 0
    for material, start, count, owner in faces:
        if count != 4: continue
        positions, normals = meshes[material][:2]
        points = positions[start:start+4]
        n = normals[start]
        lo = [min(p[i] for p in points) for i in range(3)]
        hi = [max(p[i] for p in points) for i in range(3)]
        # Cull boxes outside the reach or fully behind this face, including self.
        near=[]
        for box_index, (lower, upper) in enumerate(boxes):
            if box_index == owner: continue
            gap=sum(max(0,lower[i]-hi[i],lo[i]-upper[i])**2 for i in range(3))
            front=sum((upper[i] if n[i]>=0 else lower[i])*n[i] for i in range(3))-dot(points[0],n)
            if gap < REACH**2 and front > .002: near.append((lower,upper))
        if not near: continue
        u=[points[1][i]-points[0][i] for i in range(3)]
        v=[points[3][i]-points[0][i] for i in range(3)]
        w=max(2,math.ceil(math.sqrt(dot(u,u))/.10)+1)
        h=max(2,math.ceil(math.sqrt(dot(v,v))/.10)+1)
        if x+w+2*PAD>size: x=8;y+=row;row=0
        if y+h+2*PAD>size: raise ValueError('AO atlas overflow')
        row=max(row,h+2*PAD)
        tangent=unit(u);bitangent=cross(n,tangent)
        rays=[]
        for i in range(SAMPLES):
            radius=math.sqrt((i+.5)/SAMPLES);angle=i*2.39996323
            rays.append(tuple(tangent[a]*radius*math.cos(angle)+bitangent[a]*radius*math.sin(angle)+n[a]*math.sqrt(1-radius*radius) for a in range(3)))
        pixels=[]
        for py in range(h):
            for px in range(w):
                # Inset from UV seams avoids falsely starting inside an adjacent box.
                fu=.002+.996*px/(w-1);fv=.002+.996*py/(h-1)
                p=tuple(points[0][a]+u[a]*fu+v[a]*fv+n[a]*.003 for a in range(3))
                # Leave bounded bounce fill even in a narrow architectural crevice.
                pixels.append(round(255*(.3+.7*visibility(p,rays,near))))
        for py in range(-PAD,h+PAD):
            for px in range(-PAD,w+PAD):
                value=pixels[max(0,min(h-1,py))*w+max(0,min(w-1,px))]
                index=((y+PAD+py)*size+x+PAD+px)*4
                atlas[index:index+4]=bytes([value,value,value,255])
        for corner,(cx,cy) in enumerate([(0,0),(w-1,0),(w-1,h-1),(0,h-1)]):
            coords[material][start+corner]=[(x+PAD+cx+.5)/size,(y+PAD+cy+.5)/size]
        x+=w+2*PAD;baked+=1
    print(f'  AO: {baked} sheltered face charts, {size}x{size}, {SAMPLES} rays/sample',flush=True)
    return coords,atlas
