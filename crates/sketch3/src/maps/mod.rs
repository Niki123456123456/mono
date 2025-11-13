use glam::{DMat4, DVec3, Vec4Swizzles};
use std::sync::Arc;
use three_d::Geometry;

mod tiles;
pub use tiles::*;

mod orbitcontrol;
pub use orbitcontrol::*;

mod search;
pub use search::*;

pub fn glam_to_three_d(mat: &glam::Mat4) -> three_d::Mat4 {
    let cols = mat.to_cols_array();
    three_d::Mat4::new(
        cols[0], cols[1], cols[2], cols[3], cols[4], cols[5], cols[6], cols[7], cols[8], cols[9],
        cols[10], cols[11], cols[12], cols[13], cols[14], cols[15],
    )
}

pub fn three_d_to_glam(mat: &three_d::Mat4) -> glam::DMat4 {
    glam::DMat4::from_cols_array(&[
        mat[0][0] as f64,
        mat[0][1] as f64,
        mat[0][2] as f64,
        mat[0][3] as f64,
        mat[1][0] as f64,
        mat[1][1] as f64,
        mat[1][2] as f64,
        mat[1][3] as f64,
        mat[2][0] as f64,
        mat[2][1] as f64,
        mat[2][2] as f64,
        mat[2][3] as f64,
        mat[3][0] as f64,
        mat[3][1] as f64,
        mat[3][2] as f64,
        mat[3][3] as f64,
    ])
}

pub fn three_d_vec3_to_glam_d(vec: &three_d::Vec3) -> glam::DVec3 {
    glam::DVec3::new(vec.x as f64, vec.y as f64, vec.z as f64)
}

pub fn three_d_vec3_to_glam(vec: &three_d::Vec3) -> glam::Vec3 {
    glam::Vec3::new(vec.x, vec.y, vec.z)
}

pub fn glam_d_vec3_to_three_d(vec: &glam::DVec3) -> three_d::Vec3 {
    three_d::Vec3::new(vec.x as f32, vec.y as f32, vec.z as f32)
}

pub struct TileContent {
    mesh: three_d::CpuMesh,
    texture: three_d::CpuTexture,
    mat: glam::Mat4,
}

pub struct TileContentGPU {
    mesh_gpu: three_d::Mesh,
    texture_gpu: three_d::Texture2DRef,
}

pub enum TileContentState {
    None,
    Loading(poll_promise::Promise<Vec<TileContent>>),
    Ready(Vec<TileContentGPU>),
}

pub struct TileCache {
    pub client: poll_promise::Promise<Arc<RestClient>>,
    pub cache: std::collections::HashMap<String, Tile>,
    pub roots: Vec<String>,
    pub node_promises: Vec<poll_promise::Promise<(String, Node)>>,
    pub material: three_d::ColorMaterial,
    pub has_load_root: bool,
}

impl TileCache {
    pub fn new(ctx3d: &three_d::Context, key: String) -> Self {
        let mut m = three_d::ColorMaterial::new(
            ctx3d,
            &three_d::CpuMaterial {
                albedo: three_d::Srgba::WHITE,
                ..Default::default()
            },
        );
        m.render_states.cull = three_d::Cull::Back;

        let (sender, client) = poll_promise::Promise::new();
        common::execute(async move {
            let client = Arc::new(RestClient::new(key).await);
            sender.send(client);
        });

        let mut cache = Default::default();

        let mut s = Self {
            client,
            cache,
            material: m,
            roots: Default::default(),
            node_promises: Default::default(),
            has_load_root: false,
        };

        return s;
    }

    pub fn load(&mut self, ctx3d: &three_d::Context) {
        if let Some(client) = self.client.ready() {
            if !self.has_load_root {
                self.has_load_root = true;
                Tile::fill(
                    &client.root,
                    &client,
                    None,
                    &mut self.cache,
                    &mut self.roots,
                    true,
                    ctx3d,
                );
            }
            let mut items_to_remove = vec![];
            for (i, a) in self.node_promises.iter_mut().enumerate() {
                if let Some((parent, node)) = a.ready_mut() {
                    items_to_remove.push(i);
                    let mut roots = vec![];
                    Tile::fill(
                        &node,
                        &client,
                        Some(&parent),
                        &mut self.cache,
                        &mut roots,
                        true,
                        ctx3d,
                    );
                    if let Some(p) = self.cache.get_mut(parent) {
                        p.children.append(&mut roots);
                    }
                }
            }
            for i in items_to_remove.into_iter().rev() {
                let _ = self.node_promises.remove(i);
            }
            for (_, t) in self.cache.iter_mut() {
                if let TileContentState::Loading(l) = &mut t.content {
                    if let Some(r) = l.ready_mut() {
                        let mut contents = vec![];

                        for r in r.iter() {
                            let mut mesh_gpu = three_d::Mesh::new(&ctx3d, &r.mesh);
                            mesh_gpu.set_transformation(glam_to_three_d(&r.mat));

                            let texture_gpu =
                                three_d::Texture2DRef::from_cpu_texture(&ctx3d, &r.texture);

                            contents.push(TileContentGPU {
                                mesh_gpu,
                                texture_gpu,
                            });
                        }
                        t.content = TileContentState::Ready(contents);
                    }
                }
            }
        }
    }

    pub fn render(&mut self, camera: &three_d::Camera, lights: &[&dyn three_d::Light]) -> usize {
        if let Some(client) = self.client.ready() {
            let position = three_d_vec3_to_glam_d(&camera.position());
            let frustum = Frustum::from_view_proj_with_origin_far(
                three_d_to_glam(&(camera.projection() * camera.view())),
                position,
            );
            let s = ViewState {
                frustum,
                planes: extract_planes(&three_d_to_glam(&(camera.projection() * camera.view()))),
                position: three_d_vec3_to_glam_d(&camera.position()),
                viewport_size: glam::dvec2(
                    camera.viewport().width as f64,
                    camera.viewport().height as f64,
                ),
                culling_volume: CullingVolume::new_matrix(three_d_to_glam(
                    &(camera.projection() * camera.view()),
                )),
                projection_matrix: three_d_to_glam(&camera.projection()),
            };

            let mut counter = 0;
            for r in self.roots.iter() {
                render_tile(
                    r,
                    &mut self.cache,
                    &s,
                    &self.material,
                    camera,
                    lights,
                    &mut counter,
                    &client,
                    &mut self.node_promises,
                    20,
                );
            }
            return counter;
        }
        return 0;
    }
}

pub fn render_tile(
    id: &String,
    cache: &mut std::collections::HashMap<String, Tile>,
    s: &ViewState,
    material: &three_d::ColorMaterial,
    camera: &three_d::Camera,
    lights: &[&dyn three_d::Light],
    counter: &mut usize,
    rest_client: &Arc<tiles::RestClient>,
    node_promises: &mut Vec<poll_promise::Promise<(String, Node)>>,
    max_level: usize,
) {
    let mut childern = vec![];
    let mut meet_sse = false;

    if let Some(t) = cache.get_mut(id) {
        let is_visible = t.bv.is_visible(s.position) && t.bv.intersects_frustum(&s.frustum);

        if is_visible {
            let meet_sse = s.does_tile_meet_sse(t);

            // && t.children.iter().all(|c| cache.get(c).is_some_and(||))
            if !t.children.is_empty() && !meet_sse && max_level > 0 {
                childern = t.children.clone();
            } else {
                if let TileContentState::None = &t.content {
                    t.content = TileContentState::Loading(get_contents(id.clone(), rest_client));
                }
                if !meet_sse && !t.child_options.is_empty() && max_level > 0 {
                    for c in t.child_options.iter() {
                        node_promises.push(get_node(c.clone(), id.clone(), rest_client));
                    }
                    t.child_options.clear();
                }
                let mut m = material.clone();
                if let TileContentState::Ready(contents) = &t.content {
                    let mut m = material.clone();
                    for c in contents {
                        m.texture = Some(c.texture_gpu.clone());
                        three_d::Geometry::render_with_material(&c.mesh_gpu, &m, camera, lights);
                    }
                }
                *counter += 1;
                m.texture = None;
                if is_visible {
                    m.color = three_d::Srgba::WHITE;
                } else {
                    m.color = three_d::Srgba::RED;
                }
                if is_visible {
                    t.edges.render_with_material(&m, camera, lights);
                }
            }
        }
    }
    for id in childern.iter() {
        render_tile(
            id,
            cache,
            s,
            material,
            camera,
            lights,
            counter,
            rest_client,
            node_promises,
            max_level - 1,
        );
    }
}

pub struct Tile {
    pub bv: BoundingVolume,
    pub bounding: OrientedBoundingBox,
    pub edges: crate::bricks::line_writer::LineMesh,
    pub geometric_error: f64,
    pub content: TileContentState,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub child_options: Vec<String>,
}

impl Tile {
    pub fn has_ready_content(&self) -> bool {
        if let TileContentState::Ready(_) = self.content {
            return true;
        }
        false
    }

    pub fn fill(
        n: &Node,
        c: &Arc<RestClient>,
        parent: Option<&String>,
        cache: &mut std::collections::HashMap<String, Tile>,
        roots: &mut Vec<String>,
        is_root: bool,
        ctx3d: &three_d::Context,
    ) -> Option<String> {
        if let Some((uri, mut tile)) = Self::from_node(n, c, parent, ctx3d) {
            for child in n.children.iter() {
                if let Some(url) = Self::fill(child, c, Some(&uri), cache, roots, false, ctx3d) {
                    if url.contains(".glb") {
                        tile.children.push(url);
                    } else {
                        tile.child_options.push(url);
                    }
                }
            }
            cache.insert(uri.clone(), tile);
            if is_root {
                roots.push(uri.clone());
            }
        } else {
            for child in n.children.iter() {
                Self::fill(child, c, None, cache, roots, is_root, ctx3d);
            }
        }
        if let Some(content) = &n.content {
            return Some(content.uri.clone());
        }
        None
    }
    pub fn from_node(
        n: &Node,
        c: &Arc<RestClient>,
        parent: Option<&String>,
        ctx3d: &three_d::Context,
    ) -> Option<(String, Self)> {
        if let Some(content) = &n.content {
            if content.uri.contains(".glb") {
                let tile = Self {
                    bv: n.bounding.clone(),
                    edges: n.bounding.as_mesh(ctx3d),
                    bounding: OrientedBoundingBox::new(
                        n.bounding.center,
                        glam::DMat3::from_cols(
                            n.bounding.x_axis,
                            n.bounding.y_axis,
                            n.bounding.z_axis,
                        ),
                    ),
                    geometric_error: n.err,
                    content: TileContentState::None,
                    parent: parent.cloned(),
                    children: vec![],
                    child_options: vec![],
                };
                return Some((content.uri.clone(), tile));
            }
        }
        None
    }
}

pub fn get_node(
    path: String,
    parent: String,
    c: &Arc<RestClient>,
) -> poll_promise::Promise<(String, Node)> {
    let c = c.clone();
    let (sender, promise) = poll_promise::Promise::new();
    common::execute(async move {
        let node = c.get_node(&path).await;
        sender.send((parent, node));
    });
    return promise;
}

pub fn get_contents(path: String, c: &Arc<RestClient>) -> poll_promise::Promise<Vec<TileContent>> {
    let (sender, promise) = poll_promise::Promise::new();

    let c = c.clone();
    common::execute(async move {
        let bytes = c.download(&path).await;

        let glb = gltf::Gltf::from_reader_without_validation(std::io::Cursor::new(bytes)).unwrap();
        let doc = glb.document;
        let blob = glb.blob;
        let buffer_data = gltf::import_buffers(&doc, None, blob).unwrap();
        let image_data = gltf::import_images(&doc, None, &buffer_data).unwrap();

        let n = doc.nodes().last().unwrap();

        let (p, r, s) = n.transform().decomposed();

        let translation = glam::vec3(p[0], p[1], p[2]);
        let rotation = glam::quat(r[0], r[1], r[2], r[3]);
        let scale = glam::vec3(s[0], s[1], s[2]);

        let mat = glam::Mat4::from_scale_rotation_translation(scale, rotation, translation);

        let source_mesh = doc.meshes().last().unwrap();
        let primitives: Vec<_> = source_mesh.primitives().collect();
        let images: Vec<_> = image_data.iter().collect();

        let mut contents = vec![];

        for (i, prim) in source_mesh.primitives().enumerate() {
            contents.push(get_content(&prim, mat, &images[i], &doc, &buffer_data).await);
        }

        sender.send(contents);
    });
    return promise;
}

pub async fn get_content(
    prim: &gltf::Primitive<'_>,
    mat: glam::Mat4,
    image: &gltf::image::Data,
    doc: &gltf::Document,
    buffer_data: &Vec<gltf::buffer::Data>,
) -> TileContent {
    let mut p: draco_gltf_rs::DecodedPrimitive = draco_gltf_rs::decode_draco(
        &prim,
        doc,
        buffer_data,
        &vec![
            draco_gltf_rs::AttrInfo {
                unique_id: 0,
                dim: 3,
                data_type: 9,
            },
            draco_gltf_rs::AttrInfo {
                unique_id: 1,
                dim: 2,
                data_type: 9,
            },
        ],
    )
    .await
    .unwrap();

    let m = glam::Mat4::from_cols_array_2d(&[
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let m_inverse = m.inverse();

    let mut mesh = three_d::CpuMesh::default();
    mesh.indices = three_d::Indices::U32(p.indices);
    mesh.positions = three_d::Positions::F32(
        p.positions
            .unwrap()
            .iter()
            // .map(|v| m * mat * glam::vec4(v[0], v[1], v[2], 1.))
            .map(|v| three_d::vec3(v[0], v[1], v[2]))
            .collect(),
    );
    mesh.uvs = Some(
        p.texcoords
            .get_mut(&0)
            .unwrap()
            .iter()
            .map(|v| three_d::vec2(v[0], v[1]))
            .collect(),
    );

    let mut texture = three_d::CpuTexture::default();
    texture.width = image.width;
    texture.height = image.height;
    texture.wrap_s = three_d::Wrapping::ClampToEdge;
    texture.wrap_t = three_d::Wrapping::ClampToEdge;

    texture.data = three_d::TextureData::RgbU8(
        image
            .pixels
            .chunks_exact(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
            .collect(),
    );

    return TileContent {
        mesh,
        texture,
        mat: m * mat,
    };
}

pub fn obb_in_frustum(planes: &[Plane; 6], obb: &BoundingVolume) -> bool {
    for plane in planes {
        let r = obb.x_axis.abs().dot(plane.normal.abs())
            + obb.y_axis.abs().dot(plane.normal.abs())
            + obb.z_axis.abs().dot(plane.normal.abs());
        let d = obb.center.dot(plane.normal) + plane.d;
        if d + r < 0.0 {
            return false;
        }
    }
    true
}

pub fn extract_planes(m: &DMat4) -> [Plane; 6] {
    let m = m.to_cols_array_2d();

    let make_plane = |a: DVec3, b: DVec3, c: DVec3, d: DVec3| -> Plane {
        let n = DVec3::new(
            a.x * b.x + c.x * d.x,
            a.y * b.y + c.y * d.y,
            a.z * b.z + c.z * d.z,
        );
        let len = n.length();
        Plane {
            normal: n / len,
            d: (m[3][0] + m[0][0]) / len,
        }
    };

    [
        Plane {
            normal: DVec3::new(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0]),
            d: m[3][3] + m[3][0],
        }, // Left
        Plane {
            normal: DVec3::new(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0]),
            d: m[3][3] - m[3][0],
        }, // Right
        Plane {
            normal: DVec3::new(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1]),
            d: m[3][3] + m[3][1],
        }, // Bottom
        Plane {
            normal: DVec3::new(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1]),
            d: m[3][3] - m[3][1],
        }, // Top
        Plane {
            normal: DVec3::new(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2]),
            d: m[3][3] + m[3][2],
        }, // Near
        Plane {
            normal: DVec3::new(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2]),
            d: m[3][3] - m[3][2],
        }, // Far
    ]
    .map(|mut p| {
        let len = p.normal.length();
        p.normal /= len;
        p.d /= len;
        p
    })
}

pub struct Plane {
    pub normal: glam::DVec3,
    pub d: f64,
}

impl Plane {
    pub fn new2(normal: glam::DVec3, distance: f64) -> Self {
        Self {
            normal,
            d: distance,
        }
    }
    pub fn new(point: glam::DVec3, normal: glam::DVec3) -> Self {
        Self {
            normal,
            d: -(normal.dot(point)),
        }
    }
    pub fn get_point_distance(&self, point: glam::DVec3) -> f64 {
        self.normal.dot(point) + self.d
    }

    pub fn project_point_onto_plane(&self, point: glam::DVec3) -> glam::DVec3 {
        let point_distance = self.get_point_distance(point);
        let scaled_normal = self.normal * point_distance;
        return point - scaled_normal;
    }
}

pub fn next_after(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return x + y;
    }

    let mut ux_i = x.to_bits();
    let uy_i = y.to_bits();
    if ux_i == uy_i {
        return y;
    }

    let ax = ux_i & (!1_u64 / 2);
    let ay = uy_i & (!1_u64 / 2);
    if ax == 0 {
        if ay == 0 {
            return y;
        }
        ux_i = (uy_i & (1_u64 << 63)) | 1;
    } else if ax > ay || ((ux_i ^ uy_i) & (1_u64 << 63)) != 0 {
        ux_i -= 1;
    } else {
        ux_i += 1;
    }

    let e = (ux_i >> 52) & 0x7ff;
    let ux_f = f64::from_bits(ux_i);
    ux_f
}

pub struct CullingVolume {
    pub left_plane: Plane,
    pub right_plane: Plane,
    pub top_plane: Plane,
    pub bottom_plane: Plane,
}

impl CullingVolume {
    pub fn new(
        position: glam::DVec3,
        direction: glam::DVec3,
        up: glam::DVec3,
        fovx: f64,
        fovy: f64,
    ) -> Self {
        let t = f64::tan(0.5 * fovy);
        let r = f64::tan(0.5 * fovx);
        let right = direction.cross(up);

        let position_len = position.length();
        let n = f64::max(1.0, next_after(position_len, f64::MAX) - position_len);
        let near_center = position + direction * n;

        let create_normal =
            |normal: glam::DVec3, f: &dyn Fn(glam::DVec3) -> glam::DVec3| -> Plane {
                Plane::new(
                    position,
                    (f)((near_center + normal - position).normalize()).normalize(),
                )
            };
        Self {
            left_plane: create_normal(right * -r, &|n| n.cross(up)),
            right_plane: create_normal(right * r, &|n| up.cross(n)),
            bottom_plane: create_normal(up * -t, &|n| right.cross(n)),
            top_plane: create_normal(up * t, &|n| n.cross(right)),
        }
    }

    pub fn new_matrix(clip_matrix: glam::DMat4) -> Self {
        let clip_matrix = clip_matrix.to_cols_array_2d();
        let create_normal = |a: f64, b: f64, c: f64, d: f64| -> Plane {
            let len = (a * a + b * b + c * c).sqrt();
            return Plane::new2(glam::dvec3(a / len, b / len, c / len), d / len);
        };
        Self {
            left_plane: create_normal(
                clip_matrix[0][3] + clip_matrix[0][0],
                clip_matrix[1][3] + clip_matrix[1][0],
                clip_matrix[2][3] + clip_matrix[2][0],
                clip_matrix[3][3] + clip_matrix[3][0],
            ),
            right_plane: create_normal(
                clip_matrix[0][3] - clip_matrix[0][0],
                clip_matrix[1][3] - clip_matrix[1][0],
                clip_matrix[2][3] - clip_matrix[2][0],
                clip_matrix[3][3] - clip_matrix[3][0],
            ),
            bottom_plane: create_normal(
                clip_matrix[0][3] - clip_matrix[0][1],
                clip_matrix[1][3] - clip_matrix[1][1],
                clip_matrix[2][3] - clip_matrix[2][1],
                clip_matrix[3][3] - clip_matrix[3][1],
            ),
            top_plane: create_normal(
                clip_matrix[0][3] + clip_matrix[0][1],
                clip_matrix[1][3] + clip_matrix[1][1],
                clip_matrix[2][3] + clip_matrix[2][1],
                clip_matrix[3][3] + clip_matrix[3][1],
            ),
        }
    }

    pub fn new_perspective(
        position: glam::DVec3,
        direction: glam::DVec3,
        up: glam::DVec3,
        l: f64,
        r: f64,
        b: f64,
        t: f64,
        n: f64,
    ) -> Self {
        let proj_matrix = glam::DMat4::perspective_rh_gl(l, r, b, t);
        let view_matrix = glam::DMat4::look_at_rh(position, position + direction, up);
        let clip_matrix = proj_matrix * view_matrix;
        Self::new_matrix(clip_matrix)
    }

    pub fn new_orthographic(
        position: glam::DVec3,
        direction: glam::DVec3,
        up: glam::DVec3,
        l: f64,
        r: f64,
        b: f64,
        t: f64,
        n: f64,
    ) -> Self {
        let proj_matrix = glam::DMat4::orthographic_rh_gl(l, r, b, t, n, f64::INFINITY);
        let view_matrix = glam::DMat4::look_at_rh(position, position + direction, up);
        let clip_matrix = proj_matrix * view_matrix;
        Self::new_matrix(clip_matrix)
    }

    pub fn is_bounding_volume_visible(&self, b: &OrientedBoundingBox) -> bool {
        if b.intersect_plane(&self.left_plane) == CullingResult::Outside {
            return false;
        }
        if b.intersect_plane(&self.right_plane) == CullingResult::Outside {
            return false;
        }
        if b.intersect_plane(&self.top_plane) == CullingResult::Outside {
            return false;
        }
        if b.intersect_plane(&self.bottom_plane) == CullingResult::Outside {
            return false;
        }
        return true;
    }
}

pub struct OrientedBoundingBox {
    pub center: glam::DVec3,
    pub half_axes: glam::DMat3,
    pub inverse_half_axes: glam::DMat3,
    pub lengths: glam::DVec3,
}

pub fn equals_epsilon(
    left: glam::DVec3,
    right: glam::DVec3,
    relative_epsilon: f64,
    absolute_epsilon: f64,
) -> bool {
    let diff = (left - right).abs();

    if diff.x <= absolute_epsilon || diff.y <= absolute_epsilon || diff.z <= absolute_epsilon {
        return true;
    }
    if diff.x <= relative_epsilon_to_absolute(left.x, left.x, relative_epsilon)
        || diff.y <= relative_epsilon_to_absolute(left.y, left.y, relative_epsilon)
        || diff.z <= relative_epsilon_to_absolute(left.z, left.z, relative_epsilon)
    {
        return true;
    }
    return false;
}

pub fn relative_epsilon_to_absolute(a: f64, b: f64, relative_epsilon: f64) -> f64 {
    return relative_epsilon * a.abs().max(b.abs());
}

impl OrientedBoundingBox {
    pub fn new(center: glam::DVec3, half_axes: glam::DMat3) -> Self {
        Self {
            center,
            half_axes,
            inverse_half_axes: half_axes.inverse(),
            lengths: glam::dvec3(
                half_axes.col(0).length(),
                half_axes.col(1).length(),
                half_axes.col(2).length(),
            ) * 2.,
        }
    }

    pub fn transform(&self, transformation: glam::DMat4) -> Self {
        Self::new(
            (transformation * glam::dvec4(self.center.x, self.center.y, self.center.z, 1.)).xyz(),
            glam::dmat3(
                transformation.col(0).xyz(),
                transformation.col(1).xyz(),
                transformation.col(1).xyz(),
            ) * self.half_axes,
        )
    }

    pub fn intersect_plane(&self, plane: &Plane) -> CullingResult {
        let rad_effective = self.half_axes.col(0).dot(plane.normal).abs()
            + self.half_axes.col(1).dot(plane.normal).abs()
            + self.half_axes.row(2).dot(plane.normal).abs();
        let distance_to_plane = plane.normal.dot(self.center + plane.d);
        if distance_to_plane <= -rad_effective {
            return CullingResult::Outside;
        }
        if distance_to_plane >= rad_effective {
            return CullingResult::Inside;
        }
        return CullingResult::Intersecting;
    }

    pub fn compute_distance_squared_to_position(&self, position: glam::DVec3) -> f64 {
        let offset = position - self.center;

        let mut u = self.half_axes.col(0);
        let mut v = self.half_axes.col(1);
        let mut w = self.half_axes.col(2);

        let uHalf = u.length();
        let vHalf = v.length();
        let wHalf = w.length();

        let uValid = uHalf > 0.;
        let vValid = vHalf > 0.;
        let wValid = wHalf > 0.;

        let mut numberOfDegenerateAxes = 0;
        if (uValid) {
            u /= uHalf;
        } else {
            numberOfDegenerateAxes += 1;
        }

        if (vValid) {
            v /= vHalf;
        } else {
            numberOfDegenerateAxes += 1;
        }

        if (wValid) {
            w /= wHalf;
        } else {
            numberOfDegenerateAxes += 1;
        }
        let mut validAxis1 = glam::DVec3::ZERO;
        let mut validAxis2 = glam::DVec3::ZERO;
        let mut validAxis3 = glam::DVec3::ZERO;

        if (numberOfDegenerateAxes == 1) {
            let mut degenerateAxis = u;
            validAxis1 = v;
            validAxis2 = w;

            if (!vValid) {
                degenerateAxis = v;
                validAxis1 = u;
            } else if (!wValid) {
                degenerateAxis = w;
                validAxis2 = u;
            }

            validAxis3 = validAxis1.cross(validAxis2);

            if (!uValid) {
                u = validAxis3;
            } else if (!vValid) {
                v = validAxis3;
            } else {
                w = validAxis3;
            }
        } else if (numberOfDegenerateAxes == 2) {
            if (uValid) {
                validAxis1 = u;
            } else if (vValid) {
                validAxis1 = v;
            } else {
                validAxis1 = w;
            }

            let mut crossVector = glam::dvec3(0., 1., 0.);
            if (equals_epsilon(validAxis1, crossVector, 1e-3, 1e-3)) {
                crossVector = glam::dvec3(1., 0., 0.);
            }

            validAxis2 = validAxis1.cross(crossVector).normalize();
            validAxis3 = validAxis1.cross(validAxis2).normalize();

            if (uValid) {
                v = validAxis2;
                w = validAxis3;
            } else if (vValid) {
                w = validAxis2;
                u = validAxis3;
            } else if (wValid) {
                u = validAxis2;
                v = validAxis3;
            }
        } else if (numberOfDegenerateAxes == 3) {
            u = glam::dvec3(1., 0., 0.);
            v = glam::dvec3(0., 1., 0.);
            w = glam::dvec3(0., 0., 1.);
        }

        let pPrime = glam::dvec3(offset.dot(u), offset.dot(v), offset.dot(w));

        let mut distanceSquared = 0.0;
        let mut d = 0.;

        if (pPrime.x < -uHalf) {
            d = pPrime.x + uHalf;
            distanceSquared += d * d;
        } else if (pPrime.x > uHalf) {
            d = pPrime.x - uHalf;
            distanceSquared += d * d;
        }

        if (pPrime.y < -vHalf) {
            d = pPrime.y + vHalf;
            distanceSquared += d * d;
        } else if (pPrime.y > vHalf) {
            d = pPrime.y - vHalf;
            distanceSquared += d * d;
        }

        if (pPrime.z < -wHalf) {
            d = pPrime.z + wHalf;
            distanceSquared += d * d;
        } else if (pPrime.z > wHalf) {
            d = pPrime.z - wHalf;
            distanceSquared += d * d;
        }

        return distanceSquared;
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum CullingResult {
    Outside = -1,
    Intersecting = 0,
    Inside = 1,
}

pub struct ViewState {
    pub frustum: Frustum,
    pub planes: [Plane; 6],
    pub position: glam::DVec3,
    pub viewport_size: glam::DVec2,
    pub culling_volume: CullingVolume,
    pub projection_matrix: glam::DMat4,
}

impl ViewState {
    pub fn compute_screen_space_error(&self, geometric_error: f64, distance: f64) -> f64 {
        let distance = distance.max(1e-7);
        let mut center_ndc = self.projection_matrix * glam::dvec4(0., 0., -distance, 1.);
        center_ndc /= center_ndc.w;
        let mut error_offset_ndc =
            self.projection_matrix * glam::dvec4(0., geometric_error, -distance, 1.);
        error_offset_ndc /= error_offset_ndc.w;
        let ndc_error = (error_offset_ndc - center_ndc).y;

        return -ndc_error * self.viewport_size.y / 2.;
    }

    pub fn does_tile_meet_sse(&self, tile: &Tile) -> bool {
        let distance = tile
            .bounding
            .compute_distance_squared_to_position(self.position)
            .sqrt();
        let sse = self
            .compute_screen_space_error(tile.geometric_error, distance)
            .abs();
        // println!("sse {}", sse);
        let maximum_screen_space_error = 16.0;
        return sse < maximum_screen_space_error;
    }
}
