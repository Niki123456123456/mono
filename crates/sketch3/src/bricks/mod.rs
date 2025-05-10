use std::collections::HashMap;
use std::io::Read;

pub mod gltf;
pub mod gltf_writer;

pub struct Color {
    pub id: u32,
    pub name: String,
    pub rgb: String,
    pub is_trans: bool,
}

#[derive(Debug, Default, serde::Deserialize, Clone)]
pub struct PartCategories {
    pub id: u32,
    pub name: String,
    #[serde(skip)]
    pub parts: Vec<Part>,
}

#[derive(Debug, Default, serde::Deserialize, Clone)]
pub struct Part {
    #[serde(rename = "part_num")]
    pub number: String,
    pub name: String,
    #[serde(rename = "part_cat_id")]
    pub category: u32,
    #[serde(rename = "part_material")]
    pub material: String,
}

pub fn get_ldraw_lib() -> ZipResolver {
    let ldraw = include_bytes!("./ldraw.zip");
    return ZipResolver { data: ldraw };
}

pub struct ZipResolver {
    pub data: &'static [u8; 95330110],
}

impl weldr::FileRefResolver for ZipResolver {
    fn resolve<P: AsRef<std::path::Path>>(&self, filename: P) -> Result<Vec<u8>, weldr::ResolveError> {
        let filename = filename.as_ref().to_str().unwrap().replace("\\", "/");

        let ldraw_cursor = std::io::Cursor::new(self.data);
        let mut archive = zip::ZipArchive::new(ldraw_cursor).unwrap();

        let paths = vec![
            format!("ldraw/parts/{}", filename),
            format!("ldraw/p/{}", filename),
        ];
        for path in paths.iter() {
            if let Ok(mut file) = archive.by_name(&path) {
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)
                    .map_err(|_| weldr::ResolveError::new_raw(&filename))?;
                return Ok(buffer);
            }
        }
        return Err(weldr::ResolveError::new_raw(&filename));
    }
}

pub static PART_CATEGORIES: std::sync::LazyLock<Vec<PartCategories>> =
    std::sync::LazyLock::new(|| {
        let categories = include_bytes!("./part_categories.csv");
        let parts = include_bytes!("./parts.csv");

        let mut categories = csv::Reader::from_reader(&categories[..])
            .deserialize()
            .into_iter()
            .filter_map(|x| x.ok())
            .map(|x: PartCategories| (x.id, x))
            .collect::<HashMap<u32, PartCategories>>();

        let parts = csv::Reader::from_reader(&parts[..])
            .deserialize()
            .into_iter()
            .filter_map(|x| x.ok())
            .map(|x: Part| (x.number.clone(), x))
            .collect::<HashMap<String, Part>>();

        for part in parts.values() {
            if let Some(category) = categories.get_mut(&part.category) {
                category.parts.push(part.clone());
            }
        }

        categories.into_iter().map(|x| x.1).collect()
    });
