#!/usr/bin/env python3
"""Reproducible CC0 modelling source for the Genesis static GLB fixture.
All geometry is in metres, Y up, front +Z. No third-party art or baked light.
Run from any directory; exports opaque indexed geometry and embedded PNG maps.
"""
import json
import math
from pathlib import Path
import struct
import zlib
from dwelling_mesh import box as mesh_box
from dwelling_ao import bake

ROOT = Path(__file__).resolve().parents[1] / 'assets/models/dwelling'
N = 128

def png(pixels, size=N):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    raw = b''.join(b'\0' + bytes(pixels[y*size*4:(y+1)*size*4]) for y in range(size))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>2I5B', size, size, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')

def noise(u,v,frequency):
    x,y=u*frequency,v*frequency
    ix,iy=math.floor(x),math.floor(y)
    fx,fy=x-ix,y-iy;fx=fx*fx*(3-2*fx);fy=fy*fy*(3-2*fy)
    def sample(a,b):
        h=((a%frequency)*374761393+(b%frequency)*668265263+12345)&0xffffffff
        h=((h^(h>>13))*1274126177)&0xffffffff
        return ((h^(h>>16))&65535)/32767.5-1
    a=sample(ix,iy)*(1-fx)+sample(ix+1,iy)*fx
    b=sample(ix,iy+1)*(1-fx)+sample(ix+1,iy+1)*fx
    return a*(1-fy)+b*fy

def height(kind, u, v):
    grain = sum(noise(u,v,f)*amp for f,amp in [(4,.45),(12,.26),(32,.16),(64,.08)])
    if kind == 2:
        warp=.7*noise(u,v,4)
        return .18*math.sin(u*math.tau*29+warp) + grain*.45
    return grain

def maps(kind):
    base, normal, surface, ao = [], [], [], []
    colors = [(194,176,143),(187,172,144),(110,77,43),(143,118,79)]
    for y in range(N):
        for x in range(N):
            u,v=x/N,y/N
            h=height(kind,u,v)
            variation=1+h*(.12 if kind!=2 else .3)
            base.extend([max(0,min(255,round(c*variation))) for c in colors[kind]]+[255])
            dx=(height(kind,u+1/N,v)-height(kind,u-1/N,v))*.18
            dy=(height(kind,u,v+1/N)-height(kind,u,v-1/N))*.18
            norm=math.sqrt(dx*dx+dy*dy+1)
            normal.extend([round(127.5*(1-dx/norm)),round(127.5*(1-dy/norm)),round(127.5*(1+1/norm)),255])
            surface.extend([255,round(224-h*12),0,255])
            occ=round(255*(.98+.02*max(0,h))) if kind==1 else 255
            ao.extend([occ,occ,occ,255])
    return [png(p) for p in [base,normal,surface,ao]]

MAPS=[maps(i) for i in range(4)]

def build(lod):
    meshes=[([],[],[],[],[]) for _ in range(4)]
    faces=[];boxes=[]
    def box(center,size,mat,bevel=0):
        charts=[]
        boxes.append(([center[i]-size[i]/2 for i in range(3)],[center[i]+size[i]/2 for i in range(3)]))
        mesh_box(meshes[mat],center,size,bevel if lod==0 else 0,
                 max(range(3),key=lambda i:size[i]) if mat==2 else None, charts)
        faces.extend((mat,start,count,len(boxes)-1) for start,count in charts)
    def opening(x,bottom,w,h):
        boards=6 if lod==0 else 1
        for i in range(boards):
            box((x-w/2+(i+.5)*w/boards,bottom+h/2,2.205),(w/boards-(.007 if boards>1 else 0),h-.02,.055),2,.005)
        if lod==0:
            for f in [.24,.76]: box((x,bottom+h*f,2.25),(w*.90,.065,.035),2,.005)
            if bottom==0: box((x+w*.3,1.05,2.285),(.1,.055,.055),2,.007)
        box((x,bottom+h+.095,2.435),(w+.30,.18,.18),2,.012)
        if bottom>0: box((x,bottom-.035,2.425),(w+.16,.07,.17),1,.01)
    if lod==2:
        box((0,1.4,0),(6.5,2.8,4.8),0)
        for x,b,w,h in [(-1.8,1.45,.5,.65),(0,0,.85,1.95),(1.8,1.45,.5,.65)]:
            box((x,b+h/2,2.402),(w,h,.012),2)
    else:
        for x in [-3.075,3.075]:box((x,1.4,0),(.35,2.8,4.8),0)
        box((0,1.4,-2.225),(5.8,2.8,.35),0)
        left=-2.9
        for x,b,w,h in [(-1.8,1.45,.5,.65),(0,0,.85,1.95),(1.8,1.45,.5,.65)]:
            edge=x-w/2
            box(((left+edge)/2,1.4,2.225),(edge-left,2.8,.35),0)
            if b:box((x,b/2,2.225),(w,b,.35),0)
            top=b+h;box((x,(top+2.8)/2,2.225),(w,2.8-top,.35),0)
            opening(x,b,w,h);left=x+w/2
        box(((left+2.9)/2,1.4,2.225),(2.9-left,2.8,.35),0)
        box((0,.035,0),(5.8,.07,4.1),3)
    box((0,2.875,0),(6.5,.15,4.8),3)
    for x in [-3.15,3.15]:box((x,3.125,0),(.2,.35,4.8),0,.018)
    for z in [-2.3,2.3]:box((0,3.125,z),(6.1,.35,.2),0,.018)
    if lod<2:
        for i in range(11 if lod==0 else 5):
            x=-2.85+i*5.7/(10 if lod==0 else 4)
            box((x,2.71,2.435),(.13,.15,.20),2,.012)
    # A continuous masonry plinth at every LOD preserves the same broad palette.
    # Individual staggered courses only refine its silhouette at close range.
    for side in range(4):
        front=side==0
        length=6.5 if side<2 else 4.8
        spans=[(-length/2,-.48),(.48,length/2)] if front else [(-length/2,length/2)]
        for left,right in spans:
            if side<2:
                center=((left+right)/2,.27,2.398 if front else -2.398)
                size=(right-left,.54,.035)
            else:
                center=(3.248 if side==2 else -3.248,.27,(left+right)/2)
                size=(.035,.54,right-left)
            box(center,size,1)
            if lod==0:
                for row in range(2):
                    cursor=left
                    i=0
                    while cursor<right-.01:
                        span=min(.40+((i*7+row*3+side*5)%7)*.041,right-cursor)
                        mid=cursor+span/2
                        if side<2:
                            center=(mid,.135+row*.27,2.410 if front else -2.410)
                            size=(max(span-.012,.003),.256,.042)
                        else:
                            center=(3.260 if side==2 else -3.260,.135+row*.27,mid)
                            size=(.042,.256,max(span-.012,.003))
                        box(center,size,1,.008)
                        cursor+=span;i+=1
    ao_size=[1024,512,512][lod]
    ao_uv,ao_pixels=bake(meshes,faces,boxes,ao_size)
    ao_png=png(ao_pixels,ao_size)
    blob=bytearray();views=[];accessors=[];images=[];textures=[];materials=[]
    def view(data):
        while len(blob)%4:blob.append(0)
        index=len(views);views.append({'buffer':0,'byteOffset':len(blob),'byteLength':len(data)});blob.extend(data);return index
    def accessor(values,components,typ,component=5126):
        flat=[v for row in values for v in row] if components>1 else values
        data=struct.pack('<'+('f' if component==5126 else 'I')*len(flat),*flat)
        a={'bufferView':view(data),'componentType':component,'count':len(values),'type':typ}
        if typ=='VEC3':a.update(min=[min(v[i] for v in values) for i in range(3)],max=[max(v[i] for v in values) for i in range(3)])
        accessors.append(a);return len(accessors)-1
    for kind,name in enumerate(['lime plaster','limestone masonry','weathered timber','earthen roof']):
        ids=[]
        for map_index,data in enumerate(MAPS[kind]):
            if map_index==3: data=ao_png
            images.append({'bufferView':view(data),'mimeType':'image/png'});ids.append(len(textures));textures.append({'source':len(images)-1,'sampler':0})
        materials.append({'name':name,'pbrMetallicRoughness':{'baseColorTexture':{'index':ids[0]},'metallicRoughnessTexture':{'index':ids[2]},'metallicFactor':0,'roughnessFactor':1},'normalTexture':{'index':ids[1]},'occlusionTexture':{'index':ids[3],'texCoord':1}})
    primitives=[]
    for mat,(p,n,t,uv,idx) in enumerate(meshes):
        if not p:continue
        primitives.append({'attributes':{'POSITION':accessor(p,3,'VEC3'),'NORMAL':accessor(n,3,'VEC3'),'TANGENT':accessor(t,4,'VEC4'),'TEXCOORD_0':accessor(uv,2,'VEC2'),'TEXCOORD_1':accessor(ao_uv[mat],2,'VEC2')},'indices':accessor(idx,1,'SCALAR',5125),'material':mat})
    doc={'asset':{'version':'2.0','generator':'Genesis CC0 build_dwelling.py'},'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'mesh':0,'name':'dwelling metres'}],'meshes':[{'primitives':primitives}],'buffers':[{'byteLength':len(blob)}],'bufferViews':views,'accessors':accessors,'images':images,'textures':textures,'samplers':[{'magFilter':9729,'minFilter':9987,'wrapS':10497,'wrapT':10497}],'materials':materials}
    data=json.dumps(doc,separators=(',',':')).encode();data+=b' '*(-len(data)%4);blob+=b'\0'*(-len(blob)%4)
    glb=struct.pack('<III',0x46546c67,2,28+len(data)+len(blob))+struct.pack('<I4s',len(data),b'JSON')+data+struct.pack('<I4s',len(blob),b'BIN\0')+blob
    path=ROOT/f'lod{lod}.glb';path.write_bytes(glb)
    print(f'{path.name}: {sum(len(m[4])//3 for m in meshes)} triangles, {len(glb)} bytes')

if __name__=='__main__':
    ROOT.mkdir(parents=True,exist_ok=True)
    for lod in range(3):build(lod)
