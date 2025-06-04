use three_d::*;

pub fn export(mesh: &CpuMesh, name: &str) {
    // for pos in mesh.positions.to_f32().iter() {
    //     println!("{:?}, {:?}, {:?},", pos.x, pos.y, pos.z);
    // }
    // for pos in mesh.indices.clone().into_u32().unwrap().iter() {
    //     print!("{:?}, ", pos);
    // }

    // return;
    common::filesave::save_file(&format!("{}.stl", name), |w| {
        stl_io::write_stl(w, get_triangles(mesh).iter()).unwrap();
    });
}

fn compute_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let edge1 = b - a;
    let edge2 = c - a;
    edge1.cross(edge2).normalize()
}

pub fn get_triangles(mesh: &CpuMesh) -> Vec<stl_io::Triangle> {
    let x = mesh.positions.to_f32();
    match &mesh.indices {
        Indices::None => vec![],
        Indices::U8(items) => vec![],
        Indices::U16(items) => vec![],
        Indices::U32(items) => items
            .chunks(3)
            .map(|chunk| {
                let p1 = x[chunk[0] as usize];
                let p2 = x[chunk[1] as usize];
                let p3 = x[chunk[2] as usize];
                let n = compute_normal(p1, p2, p3);
                return stl_io::Triangle {
                    normal: stl_io::Vertex::new([n.x, n.y, n.z]),
                    vertices: [
                        stl_io::Vertex::new([p1.x, p1.y, p1.z]),
                        stl_io::Vertex::new([p2.x, p2.y, p2.z]),
                        stl_io::Vertex::new([p2.x, p2.y, p2.z]),
                    ],
                };
            })
            .collect(),
    }
}
