use glam::{DMat4, DVec3, DVec4};
use gltf::json::extensions::camera;
use three_d::InnerSpace;

#[derive(Debug, Default, Clone)]
pub struct BoundingVolume {
    pub center: glam::DVec3,
    pub x_axis: glam::DVec3, // half
    pub y_axis: glam::DVec3, // half
    pub z_axis: glam::DVec3, // half
}

pub fn longlat_to_xyz(c: glam::DVec2, h: f64) -> glam::DVec3 {
    let tea_party = geoconv::Lle::<geoconv::Wgs84>::new(
        geoconv::Degrees::new(c.y),
        geoconv::Degrees::new(c.x),
        geoconv::Meters::new(h),
    );
    use geoconv::CoordinateSystem;
    let r = geoconv::Wgs84::lle_to_xyz(&tea_party);

    return glam::dvec3(r.x.as_float(), r.y.as_float(), r.z.as_float());
}

pub fn xyz_to_longlat(v: glam::DVec3) -> (glam::DVec2, f64) {
    // Create an XYZ coordinate in the WGS84 system
    let xyz = geoconv::Xyz {
        x: geoconv::Meters::new(v.x),
        y: geoconv::Meters::new(v.y),
        z: geoconv::Meters::new(v.z),
    };

    use geoconv::CoordinateSystem;
    // Convert to geodetic (latitude, longitude, elevation)
    let lle: geoconv::Lle<geoconv::Wgs84> = geoconv::Wgs84::xyz_to_lle(&xyz);

    // Return as (longitude, latitude) in degrees, and height in meters
    let longlat = glam::dvec2(lle.longitude.as_float(), lle.latitude.as_float());
    let h = lle.elevation.as_float();

    (longlat, h)
}

impl BoundingVolume {
    pub fn get_edge_points(&self) -> Vec<DVec3> {
        let mut points = vec![
            self.x_axis + self.y_axis + self.z_axis,
            self.x_axis + self.y_axis - self.z_axis,
            self.x_axis - self.y_axis + self.z_axis,
            self.x_axis - self.y_axis - self.z_axis,
            -self.x_axis + self.y_axis + self.z_axis,
            -self.x_axis + self.y_axis - self.z_axis,
            -self.x_axis - self.y_axis + self.z_axis,
            -self.x_axis - self.y_axis - self.z_axis,
        ];
        for p in points.iter_mut() {
            *p += self.center;
        }
        return points;
    }
    pub fn as_mesh(&self, ctx3d: &three_d::Context) -> crate::bricks::line_writer::LineMesh {
        let indices: Vec<u32> = vec![
            0, 4, 1, 5, 2, 6, 3, 7, // frist row
            0, 1, 0, 2, 2, 3, 3, 1, // second row
            4, 5, 4, 6, 6, 7, 7, 5, // last row
        ];

        let mut mesh = three_d::CpuMesh::default();
        mesh.indices = three_d::Indices::U32(indices);
        mesh.positions = three_d::Positions::F64(
            self.get_edge_points()
                .iter()
                .map(|v| three_d::vec3(v.x, v.y, v.z))
                .collect(),
        );

        crate::bricks::line_writer::LineMesh::new(&ctx3d, &mesh)
    }

    pub fn fromgeo_str(s: &str) -> Self {
        let parts: Vec<&str> = s.split(',').collect();
        let lon1 = parts[0].trim().parse::<f64>().ok().unwrap();
        let lat1 = parts[1].trim().parse::<f64>().ok().unwrap();
        let lon2 = parts[2].trim().parse::<f64>().ok().unwrap();
        let lat2 = parts[3].trim().parse::<f64>().ok().unwrap();

        let a = glam::dvec2(lon1, lat1);
        let b = glam::dvec2(lon1, lat2);
        let c = glam::dvec2(lon2, lat1);
        let h = 0.;

        let a = longlat_to_xyz(a, h);
        let b = longlat_to_xyz(b, h);
        let c = longlat_to_xyz(c, h);

        let ab = (b - a) * 0.5;
        let ac = (c - a) * 0.5;

        let orthogonal = ab.normalize().cross(ac.normalize()) * 4000.;

        Self {
            center: a + ab + ac,
            x_axis: ab,
            y_axis: ac,
            z_axis: orthogonal,
        }
    }

    /// Liefert orthonormale Einheitsachsen (A0,A1,A2) und die Half-Extents (e0,e1,e2).
    /// Nutzt die drei Halbachsenvektoren; robust gegen sehr kleine Extents.
    fn axes_and_extents(&self) -> ([glam::DVec3; 3], [f64; 3]) {
        const EPS: f64 = 1e-12;

        let ex = self.x_axis.length();
        let ey = self.y_axis.length();
        let ez = self.z_axis.length();

        // Erste Achse
        let mut a0 = if ex > EPS {
            self.x_axis / ex
        } else {
            glam::DVec3::X
        };

        // Zweite Achse – wenn vorhanden, orthogonalisieren; sonst eine beliebige Nicht-Parallelachse wählen
        let mut a1 = if ey > EPS {
            self.y_axis / ey
        } else {
            // irgendein Vektor, der nicht (zu) parallel zu a0 ist
            if a0.x.abs() < 0.9 {
                glam::DVec3::X
            } else {
                glam::DVec3::Y
            }
        };
        // Gram-Schmidt, um exakt orthonormal zu werden
        a1 = (a1 - a0 * a1.dot(a0)).normalize();

        // Dritte Achse als Kreuzprodukt (automatisch orthogonal)
        let mut a2 = a0.cross(a1);
        let n2 = a2.length();
        if n2 > EPS {
            a2 /= n2;
        } else {
            // Degenerat: bilde a2 aus beliebigem Vektor orthogonal zu a0
            let tmp = if a0.x.abs() < 0.9 {
                glam::DVec3::X
            } else {
                glam::DVec3::Y
            };
            a2 = a0.cross(tmp).normalize();
        }

        ([a0, a1, a2], [ex, ey, ez])
    }

    /// Alle 8 Eckpunkte als Weltkoordinaten (ECEF).
    pub fn corners(&self) -> [glam::DVec3; 8] {
        let c = self.center;
        let x = self.x_axis;
        let y = self.y_axis;
        let z = self.z_axis;
        [
            c - x - y - z,
            c + x - y - z,
            c - x + y - z,
            c + x + y - z,
            c - x - y + z,
            c + x - y + z,
            c - x + y + z,
            c + x + y + z,
        ]
    }

    /// Punkt-in-OBB-Test mit kleinem Toleranzwert.
    pub fn contains_point(&self, p: glam::DVec3) -> bool {
        const TOL: f64 = 1e-9;
        let ([a0, a1, a2], [ex, ey, ez]) = self.axes_and_extents();
        let d = p - self.center;
        let u = d.dot(a0).abs();
        let v = d.dot(a1).abs();
        let w = d.dot(a2).abs();
        u <= ex + TOL && v <= ey + TOL && w <= ez + TOL
    }

    /// Test: *self* enthält *other* vollständig (alle 8 Ecken von `other` liegen in `self`).
    pub fn contains(&self, other: &Self) -> bool {
        other.corners().into_iter().all(|p| self.contains_point(p))
    }

    /// OBB-vs-OBB Schnitt-/Überlappungstest via Separating-Axis-Theorem (15 Achsen).
    /// Liefert `true` bei Überlappung **oder** vollständigem Einschluss.
    pub fn intersects(&self, other: &Self) -> bool {
        // Nach Chr. Ericson, "Real-Time Collision Detection" (Kap. zu OBB-OBB)
        const EPS: f64 = 1e-12;

        // Achsen + Extents
        let ([a0, a1, a2], [ea0, ea1, ea2]) = self.axes_and_extents();
        let ([b0, b1, b2], [eb0, eb1, eb2]) = other.axes_and_extents();

        // Rotationsmatrix R = A^T * B (Dot-Produkte der Achsen)
        let r = [
            [a0.dot(b0), a0.dot(b1), a0.dot(b2)],
            [a1.dot(b0), a1.dot(b1), a1.dot(b2)],
            [a2.dot(b0), a2.dot(b1), a2.dot(b2)],
        ];
        // |R| mit Epsilon gegen numerische Fehler
        let abs_r = [
            [
                r[0][0].abs() + EPS,
                r[0][1].abs() + EPS,
                r[0][2].abs() + EPS,
            ],
            [
                r[1][0].abs() + EPS,
                r[1][1].abs() + EPS,
                r[1][2].abs() + EPS,
            ],
            [
                r[2][0].abs() + EPS,
                r[2][1].abs() + EPS,
                r[2][2].abs() + EPS,
            ],
        ];

        // Vektor der Zentren, im A-Frame ausgedrückt
        let t_world = other.center - self.center;
        let t = [t_world.dot(a0), t_world.dot(a1), t_world.dot(a2)];

        // Hilfslambdas
        let test_axis_a = |i: usize| -> bool {
            let ra = [ea0, ea1, ea2][i];
            let rb = eb0 * abs_r[i][0] + eb1 * abs_r[i][1] + eb2 * abs_r[i][2];
            t[i].abs() <= ra + rb
        };
        let test_axis_b = |j: usize| -> bool {
            let ra = ea0 * abs_r[0][j] + ea1 * abs_r[1][j] + ea2 * abs_r[2][j];
            let rb = [eb0, eb1, eb2][j];
            // t im B-Frame: tB[j] = t·b_j = tA·R_col(j)
            let t_b = t[0] * r[0][j] + t[1] * r[1][j] + t[2] * r[2][j];
            t_b.abs() <= ra + rb
        };
        let test_axis_axb = |i: usize, j: usize| -> bool {
            let ra = match i {
                0 => ea1 * abs_r[2][j] + ea2 * abs_r[1][j],
                1 => ea0 * abs_r[2][j] + ea2 * abs_r[0][j],
                _ => ea0 * abs_r[1][j] + ea1 * abs_r[0][j],
            };
            let rb = match j {
                0 => eb1 * abs_r[i][2] + eb2 * abs_r[i][1],
                1 => eb0 * abs_r[i][2] + eb2 * abs_r[i][0],
                _ => eb0 * abs_r[i][1] + eb1 * abs_r[i][0],
            };
            let t_term = match i {
                0 => (t[2] * r[1][j] - t[1] * r[2][j]).abs(),
                1 => (t[0] * r[2][j] - t[2] * r[0][j]).abs(),
                _ => (t[1] * r[0][j] - t[0] * r[1][j]).abs(),
            };
            t_term <= ra + rb
        };

        // 3 Achsen von A
        if !test_axis_a(0) {
            return false;
        }
        if !test_axis_a(1) {
            return false;
        }
        if !test_axis_a(2) {
            return false;
        }

        // 3 Achsen von B
        if !test_axis_b(0) {
            return false;
        }
        if !test_axis_b(1) {
            return false;
        }
        if !test_axis_b(2) {
            return false;
        }

        // 9 Kreuzproduktachsen A_i × B_j
        for i in 0..3 {
            for j in 0..3 {
                if !test_axis_axb(i, j) {
                    return false;
                }
            }
        }
        true
    }

    #[inline]
    pub fn occluded(&self, camera_pos: DVec3) -> bool {
        camera_pos.normalize().dot(self.center.normalize()) < 0.
    }

    /// Standard OBB vs. frustum culling. Returns true if any part of the box can be inside.
    #[inline]
    pub fn intersects_frustum(&self, frustum: &Frustum) -> bool {
        for p in &frustum.planes {
            // Project OBB onto plane normal to get an oriented "radius" along that normal.
            let r = (p.normal.dot(self.x_axis)).abs()
                + (p.normal.dot(self.y_axis)).abs()
                + (p.normal.dot(self.z_axis)).abs();

            // Signed distance from center to plane.
            let s = p.normal.dot(self.center) + p.d;

            // If the most positive point of the OBB along the plane normal is still outside,
            // the whole box is outside this plane.
            if s + r < 0.0 {
                return false;
            }
        }
        true
    }

    #[inline]
    pub fn is_visible(&self, camera_pos: DVec3) -> bool {
        for p in self.get_edge_points() {
            if p.normalize().dot(camera_pos.normalize()) > 0.0 {
                return true;
            }
        }
        return false;
    }
}

impl<'de> serde::Deserialize<'de> for BoundingVolume {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct BoxOnly {
            #[serde(rename = "box")]
            b: [f64; 12],
        }

        let helper = BoxOnly::deserialize(deserializer)?;
        let arr = helper.b;
        Ok(Self {
            center: glam::DVec3::new(arr[0], arr[2], -arr[1]),
            x_axis: glam::DVec3::new(arr[3], arr[5], -arr[4]),
            y_axis: glam::DVec3::new(arr[6], arr[8], -arr[7]),
            z_axis: glam::DVec3::new(arr[9], arr[11], -arr[10]),
        })
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct Node {
    #[serde(rename = "boundingVolume")]
    pub bounding: BoundingVolume,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default)]
    pub content: Option<Content>,
    #[serde(rename = "geometricError")]
    pub err: f64,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct GLBInfo {
    pub bounding: BoundingVolume,
    pub url: String,
    pub err: f64,
}

#[derive(Debug, serde::Deserialize)]
pub struct Content {
    pub uri: String,
}

impl Node {
    async fn get_glbs(&self, c: &RestClient, v: &BoundingVolume, glbs: &mut Vec<GLBInfo>) {
        if self.bounding.intersects(v) {
            if !self.children.is_empty() {
                for n in self.children.iter() {
                    n.get_glbs(c, v, glbs);
                }
            } else if let Some(content) = &self.content {
                if content.uri.contains(".json") {
                    c.get_node(&content.uri).await.get_glbs(c, v, glbs);
                } else if content.uri.contains(".glb") {
                    glbs.push(GLBInfo {
                        bounding: self.bounding.clone(),
                        url: content.uri.clone(),
                        err: self.err,
                    });
                }
            }
        }
    }
}

const URL: &'static str = "https://tile.googleapis.com";
pub struct RestClient {
    pub key: String,
    pub session: String,
    pub root: Node,
}

impl RestClient {
    pub async fn new(key : String) -> Self {
        let mut s = Self {
            key,
            session: "".into(),
            root: Node::default(),
        };

        s.root = s.get_node("/v1/3dtiles/root.json?").await;

        let uri = &s
            .root
            .children
            .first()
            .unwrap()
            .children
            .first()
            .unwrap()
            .content
            .as_ref()
            .unwrap()
            .uri;
        s.session = reqwest::Url::parse(&(URL.to_string() + uri))
            .unwrap()
            .query_pairs()
            .filter(|x| x.0 == "session")
            .last()
            .unwrap()
            .1
            .into();
        return s;
    }

    fn get_url(&self, path: &str) -> reqwest::Url {
        let mut url = reqwest::Url::parse(&(URL.to_string() + path)).unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("key", &self.key);
            if self.session != "" {
                query.append_pair("session", &self.session);
            }
        }
        return url;
    }

    pub async fn get_node(&self, path: &str) -> Node {
        let res = common::http::fetch(&ehttp::Request::get(self.get_url(path)))
            .await
            .unwrap();
        #[derive(Debug, serde::Deserialize)]
        pub struct ApiResponse {
            pub root: Node,
        }
        let res: ApiResponse = serde_json::from_slice(&res.bytes).unwrap();
        res.root
    }

    pub async fn download(&self, path: &str) -> Vec<u8> {
        let res = common::http::fetch(&ehttp::Request::get(self.get_url(path)))
            .await
            .unwrap();

        res.bytes
    }

    pub fn get_glbs(&self, v: &BoundingVolume) -> Vec<GLBInfo> {
        let mut glbs = vec![];
        self.root.get_glbs(self, v, &mut glbs);
        return glbs;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Plane {
    pub normal: DVec3, // points inward (toward the frustum)
    pub d: f64,        // plane eq: normal·x + d >= 0 is inside
}

impl Plane {
    #[inline]
    pub fn normalized(self) -> Self {
        let len = self.normal.length();
        if len == 0.0 {
            return self;
        }
        Self {
            normal: self.normal / len,
            d: self.d / len,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Frustum {
    pub planes: [Plane; 5], // L, R, B, T, N, F
}

impl Frustum {
    /// Extracts the 6 frustum planes from a column-major view-projection matrix (glam's DMat4).
    /// Works for typical OpenGL-style NDC. It is widely used and also works for DX/Vulkan setups
    /// in practice, but test once with your renderer (near/far signs).
    #[inline]
    pub fn from_view_proj(vp: DMat4) -> Self {
        // glam stores column-major; build row vectors explicitly.
        let m = vp.to_cols_array_2d(); // [[col0], [col1], [col2], [col3]]
        let row0 = DVec4::new(m[0][0], m[1][0], m[2][0], m[3][0]);
        let row1 = DVec4::new(m[0][1], m[1][1], m[2][1], m[3][1]);
        let row2 = DVec4::new(m[0][2], m[1][2], m[2][2], m[3][2]);
        let row3 = DVec4::new(m[0][3], m[1][3], m[2][3], m[3][3]);

        fn make_plane(v: DVec4) -> Plane {
            Plane {
                normal: v.truncate(),
                d: v.w,
            }
            .normalized()
        }

        let left = make_plane(row3 + row0);
        let right = make_plane(row3 - row0);
        let bottom = make_plane(row3 + row1);
        let top = make_plane(row3 - row1);
        let nearp = make_plane(row3 + row2);
        let farp = make_plane(row3 - row2);

        Frustum {
            planes: [left, right, bottom, top, farp],
        }
    }

    /// Same as `from_view_proj`, but the far plane is overridden to pass through the
    /// world origin (0,0,0). The plane's inward normal points toward the camera.
    #[inline]
    pub fn from_view_proj_with_origin_far(vp: DMat4, camera_pos: DVec3) -> Self {
        let mut f = Frustum::from_view_proj(vp);

        // Normal points inward (toward camera), interior means n·x + d >= 0.
        let n = if camera_pos.length_squared() > 0.0 {
            camera_pos.normalize() // origin -> camera
        } else {
            // Fallback; shouldn't happen in a globe viewer.
            DVec3::Z
        };

        // Plane through origin: n·x + d = 0 with d = -n·0 = 0.
        f.planes[4] = Plane { normal: n, d: 0.0 };
        f
    }
}
