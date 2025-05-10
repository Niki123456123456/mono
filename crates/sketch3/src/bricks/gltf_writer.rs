use super::gltf;
use ordered_float::NotNan;
use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};
use weldr::Command;
use weldr::DrawContext;
use weldr::Error;
use weldr::Mat4;
use weldr::Vec3;

pub fn write_gltf(
    lines_enabled: bool,
    source_file: &weldr::SourceFile,
    source_map: &weldr::SourceMap,
) -> Result<three_d::CpuModel, Error> {
    let mut assets = three_d_asset::io::RawAssets::new();
    let asset = gltf::Asset {
        version: "2.0".to_string(),
        min_version: None,
        generator: None,
        copyright: None,
    };
    let scene = gltf::Scene {
        name: None,
        nodes: vec![0],
    };

    let mut gltf = gltf::Gltf {
        asset,
        nodes: Vec::new(),
        scenes: vec![scene],
        buffers: Vec::new(),
        buffer_views: Vec::new(),
        accessors: Vec::new(),
        meshes: Vec::new(),
        scene: Some(0),
    };

    let mut buffer = Vec::new();

    // Avoid creating the same mesh more than once.
    // This also saves memory for importers with instancing support.
    let mut filename_to_mesh_index = HashMap::new();

    // Recursively add a node for each file.
    add_nodes(
        lines_enabled,
        "root",
        source_file,
        None,
        source_map,
        &mut gltf,
        &mut buffer,
        &mut filename_to_mesh_index,
    );

    gltf.buffers.push(gltf::Buffer {
        name: None,
        byte_length: buffer.len() as u32,
        uri: Some("buffer.glbuf".to_string()),
    });

    assets.insert("buffer.glbuf", buffer);
    let json = serde_json::to_string_pretty(&gltf).unwrap();
    assets.insert("test.gltf", json.as_bytes().to_vec());

    let mut model = assets.deserialize("test.gltf").unwrap();
    Ok(model)
}

fn add_mesh(geometry_cache: &GeometryCache, gltf: &mut gltf::Gltf, buffer: &mut Vec<u8>) {
    // TODO: glTF is LE only; should convert on BE platforms
    let vertices = &geometry_cache.vertices;
    let vertices_bytes: &[u8] = bytemuck::cast_slice(&vertices[..]);

    // TODO: Line indices?
    let vertex_buffer_view_index = gltf.buffer_views.len() as u32;
    gltf.buffer_views.push(gltf::BufferView {
        name: Some("vertex_buffer".to_string()),
        buffer_index: 0,
        byte_length: vertices_bytes.len() as u32,
        byte_offset: buffer.len() as u32,
        byte_stride: Some(12),
        target: Some(gltf::BufferTarget::ArrayBuffer as u32),
    });
    buffer.extend_from_slice(vertices_bytes);

    let vertex_accessor = gltf::Accessor {
        name: Some("vertex_data".to_string()),
        component_type: gltf::ComponentType::Float,
        count: vertices.len() as u32,
        attribute_type: gltf::AttributeType::Vec3,
        buffer_view_index: vertex_buffer_view_index,
        byte_offset: 0,
        normalized: false,
        min: vertices
            .iter()
            .copied()
            .reduce(|a, b| weldr::Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)))
            .map(|v| [v.x, v.y, v.z]),
        max: vertices
            .iter()
            .copied()
            .reduce(|a, b| weldr::Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)))
            .map(|v| [v.x, v.y, v.z]),
    };

    let mut primitives = Vec::new();
    // TODO: Line indices.
    if !geometry_cache.triangle_indices.is_empty() {
        let attributes = HashMap::from([("POSITION".to_string(), gltf.accessors.len() as u32)]);
        gltf.accessors.push(vertex_accessor);

        // TODO: glTF is LE only; should convert on BE platforms
        let triangle_indices_bytes: &[u8] =
            bytemuck::cast_slice(&geometry_cache.triangle_indices[..]);

        let byte_offset = buffer.len() as u32;
        let byte_length = triangle_indices_bytes.len() as u32;
        let index_buffer_view_index = gltf.buffer_views.len() as u32;

        gltf.buffer_views.push(gltf::BufferView {
            name: Some("index_buffer".to_string()),
            buffer_index: 0,
            byte_length,
            byte_offset,
            byte_stride: None,
            target: Some(gltf::BufferTarget::ElementArrayBuffer as u32),
        });
        buffer.extend_from_slice(triangle_indices_bytes);

        let index_accessor = gltf::Accessor {
            name: Some("index_data".to_string()),
            component_type: gltf::ComponentType::UnsignedInt,
            count: geometry_cache.triangle_indices.len() as u32,
            attribute_type: gltf::AttributeType::Scalar,
            buffer_view_index: index_buffer_view_index,
            byte_offset: 0,
            normalized: false,
            min: None,
            max: None,
        };

        let primitive = gltf::Primitive {
            attributes,
            indices: gltf.accessors.len() as u32,
            mode: gltf::PrimitiveMode::Triangles,
        };
        primitives.push(primitive);
        gltf.accessors.push(index_accessor);
    }

    // TODO: mesh name?
    let mesh = gltf::Mesh {
        name: None,
        primitives,
    };

    gltf.meshes.push(mesh);
}

fn add_nodes(
    lines_enabled: bool,
    filename: &str,
    source_file: &weldr::SourceFile,
    transform: Option<weldr::Mat4>,
    source_map: &weldr::SourceMap,
    gltf: &mut gltf::Gltf,
    buffer: &mut Vec<u8>,
    mesh_cache: &mut HashMap<String, Option<u32>>,
) -> u32 {
    let matrix = transform.map(|m| m.to_cols_array());

    let node_index = gltf.nodes.len();
    let node = gltf::Node {
        name: Some(filename.into()),
        children: Vec::new(),
        mesh_index: None,
        matrix,
    };
    gltf.nodes.push(node);

    // Create geometry if any for this node
    let opt_mesh_index = mesh_cache.entry(filename.into()).or_insert_with(|| {
        let mesh_index = gltf.meshes.len() as u32;
        let geometry = create_geometry(lines_enabled, source_file, source_map);
        // Don't set empty meshes to avoid import errors.
        if !geometry.vertices.is_empty() && !geometry.triangle_indices.is_empty() {
            add_mesh(&geometry, gltf, buffer);
            Some(mesh_index)
        } else {
            None
        }
    });
    gltf.nodes[node_index].mesh_index = *opt_mesh_index;

    // Recursively parse sub-files
    for cmd in &source_file.cmds {
        if let Command::SubFileRef(sfr_cmd) = cmd {
            if let Some(subfile) = source_map.get(&sfr_cmd.file) {
                // Don't apply node transforms to preserve the scene hierarchy.
                // Applications should handle combining the transforms.
                let transform = sfr_cmd.matrix();

                let child_node_index = add_nodes(
                    lines_enabled,
                    &sfr_cmd.file,
                    subfile,
                    Some(transform),
                    source_map,
                    gltf,
                    buffer,
                    mesh_cache,
                );
                gltf.nodes[node_index].children.push(child_node_index);
            }
        }
    }

    node_index as u32
}

fn create_geometry(
    lines_enabled: bool,
    source_file: &weldr::SourceFile,
    source_map: &weldr::SourceMap,
) -> GeometryCache {
    let mut geometry_cache = GeometryCache::new();
    for (draw_ctx, cmd) in source_file.iter(source_map) {
        match cmd {
            Command::Line(l) => {
                if lines_enabled {
                    geometry_cache.add_line(&draw_ctx, &l.vertices)
                }
            }
            Command::Triangle(t) => geometry_cache.add_triangle(&draw_ctx, &t.vertices),
            Command::Quad(q) => geometry_cache.add_quad(&draw_ctx, &q.vertices),
            Command::OptLine(l) => {
                if lines_enabled {
                    geometry_cache.add_line(&draw_ctx, &l.vertices)
                }
            }
            _ => {}
        }
    }
    geometry_cache
}

#[derive(Hash, PartialEq, Eq)]
struct VecRef {
    x: NotNan<f32>,
    y: NotNan<f32>,
    z: NotNan<f32>,
}

impl std::convert::From<&VecRef> for Vec3 {
    fn from(vec: &VecRef) -> Vec3 {
        Vec3 {
            x: vec.x.into(),
            y: vec.y.into(),
            z: vec.z.into(),
        }
    }
}

impl std::convert::From<Vec3> for VecRef {
    fn from(vec: Vec3) -> VecRef {
        // TODO - handle NaN to avoid panic on unwrap()
        VecRef {
            x: NotNan::new(vec.x).unwrap(),
            y: NotNan::new(vec.y).unwrap(),
            z: NotNan::new(vec.z).unwrap(),
        }
    }
}

impl std::convert::From<&Vec3> for VecRef {
    fn from(vec: &Vec3) -> VecRef {
        // TODO - handle NaN to avoid panic on unwrap()
        VecRef {
            x: NotNan::new(vec.x).unwrap(),
            y: NotNan::new(vec.y).unwrap(),
            z: NotNan::new(vec.z).unwrap(),
        }
    }
}

struct GeometryCache {
    vertices: Vec<Vec3>,
    vertex_map: HashMap<VecRef, u32>,
    line_indices: Vec<u32>,
    triangle_indices: Vec<u32>,
}

impl GeometryCache {
    fn new() -> Self {
        Self {
            vertices: vec![],
            vertex_map: HashMap::new(),
            line_indices: vec![],
            triangle_indices: vec![],
        }
    }

    /// Insert a new vertex and return its index.
    fn insert_vertex(&mut self, vec: &Vec3, transform: &Mat4) -> u32 {
        let vec = transform.transform_point3(*vec);
        match self.vertex_map.get(&vec.into()) {
            Some(index) => *index,
            None => {
                let index = self.vertices.len();
                self.vertices.push(vec);
                let index = index as u32;
                self.vertex_map.insert(vec.into(), index);
                index
            }
        }
    }

    fn add_line(&mut self, draw_ctx: &DrawContext, vertices: &[Vec3; 2]) {
        let i0 = self.insert_vertex(&vertices[0], &draw_ctx.transform);
        let i1 = self.insert_vertex(&vertices[1], &draw_ctx.transform);
        self.line_indices.push(i0);
        self.line_indices.push(i1);
    }

    fn add_triangle(&mut self, draw_ctx: &DrawContext, vertices: &[Vec3; 3]) {
        let i0 = self.insert_vertex(&vertices[0], &draw_ctx.transform);
        let i1 = self.insert_vertex(&vertices[1], &draw_ctx.transform);
        let i2 = self.insert_vertex(&vertices[2], &draw_ctx.transform);
        self.triangle_indices.push(i0);
        self.triangle_indices.push(i1);
        self.triangle_indices.push(i2);
    }

    fn add_quad(&mut self, draw_ctx: &DrawContext, vertices: &[Vec3; 4]) {
        let i0 = self.insert_vertex(&vertices[0], &draw_ctx.transform);
        let i1 = self.insert_vertex(&vertices[1], &draw_ctx.transform);
        let i2 = self.insert_vertex(&vertices[2], &draw_ctx.transform);
        let i3 = self.insert_vertex(&vertices[3], &draw_ctx.transform);
        self.triangle_indices.push(i0);
        self.triangle_indices.push(i2);
        self.triangle_indices.push(i1);
        self.triangle_indices.push(i0);
        self.triangle_indices.push(i3);
        self.triangle_indices.push(i2);
    }
}
