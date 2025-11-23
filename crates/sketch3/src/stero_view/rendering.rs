use three_d::*;

use crate::stero_view::*;

pub struct Renderer {
    pub program: Program,
    pub left: RendererEye,
    pub right: RendererEye,
    pub distortion: LensDistortion,
    pub color: Texture2D,
    pub depth: DepthTexture2D,
    pub viewport: Viewport,
}

pub struct RendererEye {
    pub positions: VertexBuffer<Vec2>,
    pub uvs: VertexBuffer<Vec2>,
    pub indices: ElementBuffer<u32>,
    pub start: Vec2,
    pub end : Vec2,
}

impl RendererEye {
    pub fn new(
        context: &Context,
        lens: &LensDistortionEye,
        start: Vec2,
        end : Vec2,
    ) -> Self {
        let p: Vec<_> = lens
            .mesh
            .vertex_data
            .iter()
            .map(|x| vec2(x.x, x.y))
            .collect();
        let uv: Vec<_> = lens.mesh.uvs_data.iter().map(|x| vec2(x.x, x.y)).collect();

        Self {
            positions: VertexBuffer::new_with_data(context, &p),
            uvs: VertexBuffer::new_with_data(context, &uv),
            indices: ElementBuffer::new_with_data(context, &lens.mesh.index_data),
            start,
            end,
        }
    }

    pub fn render(&mut self, viewport: Viewport, program: &Program, color: &Texture2D) {
        program.use_vertex_attribute("a_Position", &self.positions);
        program.use_vertex_attribute("a_TexCoords", &self.uvs);
        program.use_texture("u_Texture", color);
        program.use_uniform("u_Start", self.start);
        program.use_uniform("u_End", self.end);

        program.draw_elements(
            RenderStates::default(),
            viewport,
            &self.indices,
            crate::context::TRIANGLE_STRIP,
        );
    }
}

impl Renderer {
    pub fn new(context: &Context, distortion: LensDistortion, viewport: Viewport) -> Self {
        let vertex_shader_source = r#"
    layout (location = 0) in vec2 a_Position;
    layout (location = 1) in vec2 a_TexCoords;

    uniform mat2 u_Rotation;

    out vec2 v_TexCoords;

    void main() {
      vec2 pos = u_Rotation * a_Position;
      gl_Position = vec4(pos, 0, 1);
      v_TexCoords = a_TexCoords;
    }
"#;

        let fragment_shader_source = r#"
    precision mediump float;

    uniform sampler2D u_Texture;
    uniform vec2 u_Start;
    uniform vec2 u_End;
    in vec2 v_TexCoords;
    out vec4 o_FragColor;

    void main() {
      vec2 coords = u_Start + v_TexCoords * (u_End - u_Start);
      o_FragColor = texture(u_Texture, coords);
    }
"#;
        let program =
            Program::from_source(context, vertex_shader_source, fragment_shader_source).unwrap();

        // Create a color texture to render into
        let is_portrait = viewport.height > viewport.width;
        let (tex_width, tex_height) = if is_portrait {(viewport.height,viewport.width)} else {(viewport.width,viewport.height)};

        let mut texture = Texture2D::new_empty::<[u8; 4]>(
            &context,
            tex_width,
            tex_height,
            Interpolation::Linear,
            Interpolation::Linear,
            None,
            Wrapping::ClampToEdge,
            Wrapping::ClampToEdge,
        );

        // Also create a depth texture to support depth testing
        let mut depth_texture = DepthTexture2D::new::<f32>(
            &context,
            tex_width,
            tex_height,
            Wrapping::ClampToEdge,
            Wrapping::ClampToEdge,
        );
        Self {
            color: texture,
            depth: depth_texture,
            program,
            left: RendererEye::new(context, &distortion.eye_left, Vec2::zero(), vec2(0.5, 1.)),
            right: RendererEye::new(context, &distortion.eye_right, vec2(0., 0.), vec2(1., 1.)),
            distortion,
            viewport,
        }
    }
    pub fn render(
        &mut self,
        is_portrait: bool,
        viewport: Viewport,
        mut render: impl FnMut(Viewport),
    ) {
        let w = viewport.width / 2;
        let h = viewport.height / 2;
        let viewports = if is_portrait { [
            Viewport {
                x: 0,
                y: h as i32,
                width: viewport.width,
                height: h,
            },
            Viewport {
                x: 0,
                y: 0,
                width: viewport.width,
                height: h,
            },
        ]} else { [
            Viewport {
                x: 0,
                y: 0,
                width: w,
                height: viewport.height,
            },
            Viewport {
                x: w as i32,
                y: 0,
                width: w,
                height: viewport.height,
            },
        ]};

        let inter_lens_distance = 0.0681293;

        let translations = [
            vec3(inter_lens_distance * 0.5, 0., 0.),
            vec3(-inter_lens_distance * 0.5, 0., 0.),
        ];

        let w2 = self.viewport.width / 2;
        let viewports2 = [
            Viewport {
                x: 0,
                y: 0,
                width: w,
                height: self.viewport.height,
            },
            Viewport {
                x: w as i32,
                y: 0,
                width: w,
                height: self.viewport.height,
            },
        ];
        let pixels = RenderTarget::new(
            self.color.as_color_target(None),
            self.depth.as_depth_target(),
        )
        .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
        .write(|| {
            let result: Result<(), crate::CoreError> = Ok(());
            for v in viewports2.iter() {
                render(v.clone());
            }
            return result;
        });

        let rotation: Mat2 = if is_portrait {
            Mat2::new(0.0, -1.0, 1.0, 0.0)
        } else {
            Mat2::new(1.0, 0.0, 0.0, 1.0)
        };
        self.program.use_uniform("u_Rotation", rotation);

        self.left.render(viewports[0].clone(), &self.program, &self.color);
        self.right.render(viewports[1].clone(), &self.program, &self.color);
    }
}
