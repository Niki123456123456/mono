use obj_exporter::{Geometry, ObjSet, Object, Primitive, Shape, TVertex, Vertex};
use three_d::*;

pub fn vertices(positions: &Positions) -> Vec<Vertex> {
    match positions {
        Positions::F32(positions) => positions
            .iter()
            .map(|v| Vertex {
                x: v.x as f64,
                y: v.y as f64,
                z: v.z as f64,
            })
            .collect(),
        Positions::F64(positions) => positions
            .iter()
            .map(|v| Vertex {
                x: v.x as f64,
                y: v.y as f64,
                z: v.z as f64,
            })
            .collect(),
    }
}

pub fn get_triangles(indices: &Indices) -> Vec<Shape> {
    match indices {
        Indices::None => vec![],
        Indices::U8(items) => vec![],
        Indices::U16(items) => vec![],
        Indices::U32(items) => items
            .chunks(3)
            .map(|chunk| Shape {
                primitive: Primitive::Triangle(
                    (chunk[0] as usize, None, None),
                    (chunk[1] as usize, None, None),
                    (chunk[2] as usize, None, None),
                ),
                groups: vec![],
                smoothing_groups: vec![],
            })
            .collect(),
    }
}

pub fn export(lines: &CpuMesh, triangles: &CpuMesh, name: &str) {
    let set = ObjSet {
        material_library: None,
        objects: vec![Object {
            name: name.to_string(),
            vertices: vertices(&triangles.positions),
            normals: vec![],
            tex_vertices: vec![],
            geometry: vec![Geometry {
                material_name: None,
                shapes: get_triangles(&triangles.indices),
            }],
        }],
    };
    
    common::filesave::save_file(&format!("{}.obj", name), |w| {
         obj_exporter::export(&set, w).unwrap();
    });
}
