"""Run with python3 -m unittest discover -s scripts -p 'test_*.py'."""
import math
import unittest
from dwelling_ao import visibility

class AmbientVisibility(unittest.TestCase):
    def test_near_wall_occludes_but_distant_geometry_leaves_open_sky(self):
        rays=[]
        for i in range(32):
            r=math.sqrt((i+.5)/32);angle=i*2.39996323
            rays.append((r*math.cos(angle),math.sqrt(1-r*r),r*math.sin(angle)))
        point=(0.,.003,0.)
        wall=([-.4,0.,-1.],[-.2,1.,1.])
        distant=([-2.,0.,-1.],[-1.8,1.,1.])
        self.assertEqual(visibility(point,rays,[]),1.)
        self.assertEqual(visibility(point,rays,[distant]),1.)
        shaded=visibility(point,rays,[wall])
        self.assertGreater(shaded,0.)
        self.assertLess(shaded,.97)
        self.assertEqual(shaded,visibility(point,rays,[wall,wall]))

    def test_parallel_rays_handle_misses_and_origins_inside_a_box(self):
        box=([0.,0.,0.],[1.,1.,1.])
        self.assertEqual(visibility((2.,.5,1.2),[(0.,0.,-1.)],[box]),1.)
        self.assertEqual(visibility((.5,.5,.5),[(0.,1.,0.)],[box]),0.)

if __name__=='__main__': unittest.main()
