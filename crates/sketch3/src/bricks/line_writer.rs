use std::collections::HashMap;

use three_d::*;

pub fn get_lines(
    source_file: &weldr::SourceFile,
    source_map: &weldr::SourceMap,
) -> Vec<weldr::Vec3> {
    let mut lines: Vec<weldr::Vec3> = vec![];
    for cmd in &source_file.cmds {
        if let weldr::Command::SubFileRef(cmd) = cmd {
            if let Some(subfile) = source_map.get(&cmd.file) {
                let transform = cmd.matrix();

                for pos in get_lines(subfile, source_map) {
                    let pos = transform * weldr::Vec4::new(pos.x, pos.y, pos.z, 1.0);
                    lines.push(weldr::Vec3::new(pos.x, pos.y, pos.z));
                }
            }
        }
        if let weldr::Command::Line(line) = cmd {
            lines.push(line.vertices[0]);
            lines.push(line.vertices[1]);
        }
    }
    return lines;
}

fn is_matrix_reversed(matrix: weldr::Mat4) -> bool {
    matrix.determinant() < 0.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Winding { // https://www.ldraw.org/article/415.html
    CW,  // Clockwise
    CCW, // Counter-clockwise
}

impl std::ops::Not for Winding {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Winding::CW => Winding::CCW,
            Winding::CCW => Winding::CW,
        }
    }
}

pub fn get_triangles(
    source_file: &weldr::SourceFile,
    source_map: &weldr::SourceMap,
) -> Vec<weldr::Vec3> {
    let mut lines: Vec<weldr::Vec3> = vec![];
    let mut invert_next = false;
    let mut winding = Winding::CCW;
    let indices = HashMap::from([
        (Winding::CCW, [[2, 1, 0], [0, 3, 2]]),
        (Winding::CW, [[0, 1, 2], [2, 3, 0]]),
    ]);
    for cmd in &source_file.cmds {
        if let weldr::Command::Comment(cmd) = cmd {
            if cmd.text.starts_with("BFC") {
                println!("command: {}", cmd.text);
            }
            if cmd.text == "BFC INVERTNEXT" {
                invert_next = true;
            }
            if cmd.text == "BFC CERTIFY CCW" {
                winding = Winding::CCW;
            }
            if cmd.text == "BFC CERTIFY CW" {
                winding = Winding::CW;
            }
            continue;
        }
        if let weldr::Command::SubFileRef(cmd) = cmd {
            if let Some(subfile) = source_map.get(&cmd.file) {
                let transform = cmd.matrix();

                if invert_next {
                    println!("file");
                }
                if is_matrix_reversed(transform) && invert_next {
                    println!("reversed matrix + invert next");
                }
                let indices = if (is_matrix_reversed(transform) && !invert_next)
                    || (!is_matrix_reversed(transform) && invert_next)
                {
                    [2, 1, 0]
                } else {
                    [0, 1, 2]
                };

                for pos in get_triangles(subfile, source_map).chunks(3) {
                    for i in indices {
                        let pos = transform * weldr::Vec4::new(pos[i].x, pos[i].y, pos[i].z, 1.0);
                        lines.push(weldr::Vec3::new(pos.x, pos.y, pos.z));
                    }
                }
                if invert_next {
                    println!("end file");
                }
            }
            invert_next = false;
        }
        if let weldr::Command::Triangle(triangle) = cmd {
            for i in indices[&winding][0] {
                lines.push(triangle.vertices[i]);
            }
            invert_next = false;
        }
        if let weldr::Command::Quad(quad) = cmd {
            if invert_next {
                println!("invert quad");
            }
            for i in indices[&winding][0] {
                lines.push(quad.vertices[i]);
            }
            for i in indices[&winding][1] {
                lines.push(quad.vertices[i]);
            }
            invert_next = false;
        }
    }
    return lines;
}

pub fn get_line_mesh(source_file: &weldr::SourceFile, source_map: &weldr::SourceMap) -> CpuMesh {
    let mut mesh = three_d::CpuMesh::default();

    let lines = get_lines(source_file, source_map);
    let lines: Vec<_> = lines
        .iter()
        .map(|v| three_d::vec3(v.x, -v.y, v.z))
        .collect();

    let (indices, lines) = get_indices(lines);

    mesh.indices = three_d::Indices::U32(indices);
    mesh.positions = three_d::Positions::F32(lines);

    return mesh;
}

pub fn get_indices(triangles: Vec<Vec3>) -> (Vec<u32>, Vec<Vec3>) {
    let mut triangles2 = vec![];
    let mut indices = vec![];
    for v in triangles.into_iter() {
        if let Some(p) = triangles2.iter().position(|x| x == &v) {
            indices.push(p as u32);
        } else {
            indices.push(triangles2.len() as u32);
            triangles2.push(v);
        }
    }
    return (indices, triangles2);
}

pub fn get_triangle_mesh(
    source_file: &weldr::SourceFile,
    source_map: &weldr::SourceMap,
) -> CpuMesh {
    let mut mesh = three_d::CpuMesh::default();

    let triangles = get_triangles(source_file, source_map);
    let triangles: Vec<_> = triangles
        .iter()
        .map(|v| three_d::vec3(v.x, -v.y, v.z))
        .collect();

    let (indices, triangles) = get_indices(triangles);

    mesh.indices = three_d::Indices::U32(indices);
    mesh.positions = three_d::Positions::F32(triangles);

    return mesh;
}

pub struct LineMesh {
    pub base_mesh: BaseMesh,
    pub context: Context,
    pub aabb: AxisAlignedBoundingBox,
    pub transformation: Mat4,
    pub animation_transformation: Mat4,
    pub animation: Option<Box<dyn Fn(f32) -> Mat4 + Send + Sync>>,
}

impl LineMesh {
    pub fn new(context: &Context, cpu_mesh: &CpuMesh) -> Self {
        let aabb = cpu_mesh.compute_aabb();
        Self {
            context: context.clone(),
            base_mesh: BaseMesh::new(context, cpu_mesh),
            aabb,
            transformation: Mat4::identity(),
            animation_transformation: Mat4::identity(),
            animation: None,
        }
    }

    pub fn from_vector(context: &Context, vertices: Vec<Vec3>) -> Self {
        let mut cpu_mesh = CpuMesh::default();
        cpu_mesh.positions = Positions::F32(vertices);
        Self::new(context, &cpu_mesh)
    }
}

pub struct BaseMesh {
    pub indices: IndexBuffer,
    pub positions: VertexBuffer<Vec3>,
    pub normals: Option<VertexBuffer<Vec3>>,
    pub tangents: Option<VertexBuffer<Vec4>>,
    pub uvs: Option<VertexBuffer<Vec2>>,
    pub colors: Option<VertexBuffer<Vec4>>,
    pub context: Context,
}

impl BaseMesh {
    pub fn new(context: &Context, cpu_mesh: &CpuMesh) -> Self {
        Self {
            indices: match &cpu_mesh.indices {
                Indices::U8(ind) => IndexBuffer::U8(ElementBuffer::new_with_data(context, ind)),
                Indices::U16(ind) => IndexBuffer::U16(ElementBuffer::new_with_data(context, ind)),
                Indices::U32(ind) => IndexBuffer::U32(ElementBuffer::new_with_data(context, ind)),
                Indices::None => IndexBuffer::None,
            },
            positions: VertexBuffer::new_with_data(context, &cpu_mesh.positions.to_f32()),
            normals: cpu_mesh
                .normals
                .as_ref()
                .map(|data| VertexBuffer::new_with_data(context, data)),
            tangents: cpu_mesh
                .tangents
                .as_ref()
                .map(|data| VertexBuffer::new_with_data(context, data)),
            uvs: cpu_mesh.uvs.as_ref().map(|data| {
                VertexBuffer::new_with_data(
                    context,
                    &data
                        .iter()
                        .map(|uv| vec2(uv.x, 1.0 - uv.y))
                        .collect::<Vec<_>>(),
                )
            }),
            colors: cpu_mesh.colors.as_ref().map(|data| {
                VertexBuffer::new_with_data(
                    context,
                    &data.iter().map(|c| c.to_linear_srgb()).collect::<Vec<_>>(),
                )
            }),
            context: context.clone(),
        }
    }

    pub fn draw(&self, program: &Program, render_states: RenderStates, viewer: &dyn Viewer) {
        self.use_attributes(program);

        match &self.indices {
            IndexBuffer::None => program.draw_arrays(
                render_states,
                viewer.viewport(),
                self.positions.vertex_count(),
                crate::context::LINES,
            ),
            IndexBuffer::U8(element_buffer) => program.draw_elements(
                render_states,
                viewer.viewport(),
                element_buffer,
                crate::context::LINES,
            ),
            IndexBuffer::U16(element_buffer) => program.draw_elements(
                render_states,
                viewer.viewport(),
                element_buffer,
                crate::context::LINES,
            ),
            IndexBuffer::U32(element_buffer) => program.draw_elements(
                render_states,
                viewer.viewport(),
                element_buffer,
                crate::context::LINES,
            ),
        }
    }

    pub fn draw_instanced(
        &self,
        program: &Program,
        render_states: RenderStates,
        viewer: &dyn Viewer,
        instance_count: u32,
    ) {
        self.use_attributes(program);

        match &self.indices {
            IndexBuffer::None => program.draw_arrays_instanced(
                render_states,
                viewer.viewport(),
                self.positions.vertex_count(),
                instance_count,
            ),
            IndexBuffer::U8(element_buffer) => program.draw_elements_instanced(
                render_states,
                viewer.viewport(),
                element_buffer,
                instance_count,
            ),
            IndexBuffer::U16(element_buffer) => program.draw_elements_instanced(
                render_states,
                viewer.viewport(),
                element_buffer,
                instance_count,
            ),
            IndexBuffer::U32(element_buffer) => program.draw_elements_instanced(
                render_states,
                viewer.viewport(),
                element_buffer,
                instance_count,
            ),
        }
    }

    fn use_attributes(&self, program: &Program) {
        program.use_vertex_attribute("position", &self.positions);

        if program.requires_attribute("normal") {
            if let Some(normals) = &self.normals {
                program.use_vertex_attribute("normal", normals);
            }
        }

        if program.requires_attribute("tangent") {
            if let Some(tangents) = &self.tangents {
                program.use_vertex_attribute("tangent", tangents);
            }
        }

        if program.requires_attribute("uv_coordinates") {
            if let Some(uvs) = &self.uvs {
                program.use_vertex_attribute("uv_coordinates", uvs);
            }
        }

        if program.requires_attribute("color") {
            if let Some(colors) = &self.colors {
                program.use_vertex_attribute("color", colors);
            }
        }
    }

    fn vertex_shader_source(&self) -> String {
        format!(
            "{}{}{}{}{}{}",
            if self.normals.is_some() {
                "#define USE_NORMALS\n"
            } else {
                ""
            },
            if self.tangents.is_some() {
                "#define USE_TANGENTS\n"
            } else {
                ""
            },
            if self.uvs.is_some() {
                "#define USE_UVS\n"
            } else {
                ""
            },
            if self.colors.is_some() {
                "#define USE_VERTEX_COLORS\n"
            } else {
                ""
            },
            include_str!("./shared.frag"),
            include_str!("./mesh.vert"),
        )
    }
}

impl Geometry for LineMesh {
    fn aabb(&self) -> AxisAlignedBoundingBox {
        self.aabb
            .transformed(self.transformation * self.animation_transformation)
    }

    fn animate(&mut self, time: f32) {
        if let Some(animation) = &self.animation {
            self.animation_transformation = animation(time);
        }
    }

    fn draw(&self, viewer: &dyn Viewer, program: &Program, render_states: RenderStates) {
        let local2world = self.transformation * self.animation_transformation;
        if let Some(inverse) = local2world.invert() {
            program.use_uniform_if_required("normalMatrix", inverse.transpose());
        } else {
            // determinant is float zero
            return;
        }

        program.use_uniform("viewProjection", viewer.projection() * viewer.view());
        program.use_uniform("modelMatrix", local2world);

        self.base_mesh.draw(program, render_states, viewer);
    }

    fn vertex_shader_source(&self) -> String {
        self.base_mesh.vertex_shader_source()
    }

    fn id(&self) -> GeometryId {
        GeometryId(0x9000)
    }

    fn render_with_material(
        &self,
        material: &dyn Material,
        viewer: &dyn Viewer,
        lights: &[&dyn Light],
    ) {
        if let Err(e) = render_with_material(&self.context, viewer, &self, material, lights) {
            panic!("{}", e.to_string());
        }
    }

    fn render_with_effect(
        &self,
        material: &dyn Effect,
        viewer: &dyn Viewer,
        lights: &[&dyn Light],
        color_texture: Option<ColorTexture>,
        depth_texture: Option<DepthTexture>,
    ) {
        if let Err(e) = render_with_effect(
            &self.context,
            viewer,
            self,
            material,
            lights,
            color_texture,
            depth_texture,
        ) {
            panic!("{}", e.to_string());
        }
    }
}
