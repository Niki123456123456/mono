struct MyCustomResolver;

impl weldr::FileRefResolver for MyCustomResolver {
    fn resolve<P: AsRef<std::path::Path>>(
        &self,
        filename: P,
    ) -> Result<Vec<u8>, weldr::ResolveError> {
        let filename = filename.as_ref().to_str().unwrap().replace("\\", "/");

        let paths = vec![
            format!("ldraw/parts/{}", filename),
            format!("ldraw/p/{}", filename),
        ];
        for path in paths.iter() {
            let path =
                std::path::Path::new("/Users/nnord/uni/projects/mono/crates/sketch3/src/bricks")
                    .join(path);

            let r = std::fs::read(&path).map_err(|e| weldr::ResolveError::new_raw(&filename));
            if r.is_ok() {
                return r;
            }
        }
        return Err(weldr::ResolveError::new_raw(&filename));
    }
}

pub fn create() {
    let mut parts = Vec::new();

    let mut resolver = super::get_ldraw_lib();
    let mut source_map = weldr::SourceMap::new();

    let mut i = 0;

    'outer: for x in super::PART_CATEGORIES.iter() {
        println!("Processing category: {}", x.name);
        for part in x.parts.iter() {
            println!("Processing part: {}", part.number);
            if part.number.contains("pr") {
                continue;
            }
            match weldr::parse(
                &format!("{}.dat", part.number),
                &MyCustomResolver {},
                &mut source_map,
            ) {
                Ok(x) => {
                    let source_file = source_map.get(&x).unwrap();

                    let lines = super::line_writer::get_lines(source_file, &source_map);
                    let lines: Vec<_> = lines.iter().flat_map(|v| [v.x, -v.y, v.z]).collect();

                    let (l_indices, lines) = super::line_writer::get_indices(lines);

                    let triangles = super::line_writer::get_triangles(source_file, &source_map);
                    let triangles: Vec<_> =
                        triangles.iter().flat_map(|v| [v.x, -v.y, v.z]).collect();
                    let (t_indices, triangles) = super::line_writer::get_indices(triangles);

                    parts.push(Part {
                        number: part.number.clone(),
                        l_indices,
                        lines,
                        t_indices,
                        triangles,
                    });
                    i += 1;
                    if i == 50 {
                        // break 'outer;
                    }
                }
                Err(err) => {
                    println!("Error parsing part {}: {}", part.number, err);
                }
            }
        }
    }

    println!("{} parts created", parts.len());
    for part in parts.chunks(1) {
        let mut string = String::new();

        for p in part.iter() {
            string.push_str(&format!(
                "UPDATE  parts:⟨{}⟩ SET lines = [{}], triangles = [{}], triangle_indices = [{}], line_indices = [{}], valid = true;\n",
                p.number,
                p.lines
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(", "),
                p.triangles
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(", "),
                     p.t_indices
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(", "),
                     p.l_indices
                    .iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let mut r = ehttp::Request::post(
            "https://delicate-weasel-06a8qi6eq5qs39dplfhab6frcc.aws-euw1.surreal.cloud/sql",
            string.as_bytes().to_vec(),
        );
        r.headers.insert("Content-Type", "application/json");
        r.headers.insert("Authorization", "Basic YWRtaW46YWRtaW4=");
        r.headers.insert("Accept", "application/json");
        r.headers.insert("Surreal-DB", "bricks");
        r.headers.insert("Surreal-NS", "bricks");
        loop {
            let resp = ehttp::fetch_blocking(&r);
            match resp {
                Ok(response) => {
                    println!("Response: {} {}", response.status, part[0].number);
                    break;
                }
                Err(err) => {
                    println!("Error fetching: {}", err);
                    break;
                }
            }
        }
    }
}

struct Part {
    number: String,
    l_indices: Vec<u32>,
    lines: Vec<f32>,
    t_indices: Vec<u32>,
    triangles: Vec<f32>,
}
