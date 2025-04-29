pub struct Color {
    pub id: u32,
    pub name: String,
    pub rgb: String,
    pub is_trans: bool,
}

pub struct PartCategories {
    pub id: u32,
    pub name: String,
}
// part_num,name,part_cat_id,part_material
pub struct Part {
    pub number: String,
    pub name: String,
    pub category: u32,
    pub material: String,
}
