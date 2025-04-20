#[derive(Debug, Clone)]
pub enum PathEntry {
    Index(usize),
    Field(String),
}

pub type Path = Vec<PathEntry>;

pub struct P<'a>(pub &'a Path);

impl std::fmt::Display for P<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut first = true;
        for entry in self.0 {
            if !first {
                write!(f, ".")?;
            }
            match entry {
                PathEntry::Index(idx) => write!(f, "[{}]", idx)?,
                PathEntry::Field(field) => write!(f, "{}", field)?,
            }
            first = false;
        }
        Ok(())
    }
}

pub fn starts_with(path: &Path, pattern: &Path) -> bool {
    if path.len() < pattern.len() {
        return false;
    }
    for (i, pat) in pattern.iter().enumerate() {
        let path_entry = &path[i];
        if !match (pat, path_entry) {
            (PathEntry::Index(i0), PathEntry::Index(i1)) => i0 == i1,
            (PathEntry::Field(f0), PathEntry::Field(f1)) => f0 == f1,
            _ => false,
        } {
            return false;
        }
    }
    return true;
}

pub fn starts_with_indexonly(path: &Path, pattern: &Path) -> bool {
    if path.len() < pattern.len() {
        return false;
    }
    for (i, pat) in pattern.iter().enumerate() {
        let path_entry = &path[i];
        if !match (pat, path_entry) {
            (PathEntry::Index(i0), PathEntry::Index(i1)) => i0 == i1,
            (PathEntry::Field(f0), PathEntry::Field(f1)) => true,
            _ => false,
        } {
            return false;
        }
    }
    return true;
}

pub fn enrich_path_with_indices(target: &mut Path, source: &Path) {
    let mut t = 0;
    let mut s = 0;

    loop {
        if s >= source.len() {
            break;
        }
        match &source[s] {
            PathEntry::Index(i) => {
                target.insert(s, PathEntry::Index(*i));
                s += 1;
                t += 1;
            }
            PathEntry::Field(source_f) => {
                if t >= target.len() {
                    break;
                }
                if let PathEntry::Field(target_f) = &mut target[t] {
                    if source_f == target_f {
                        t += 1;
                        s += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }
}

pub fn set_field_in_sequence(
    yaml: &mut serde_yaml::Value,
    path: &Path,
    next: &PathEntry,
    field: &serde_yaml::Value,
    create: bool,
) -> bool {
    if let (serde_yaml::Value::Sequence(values), PathEntry::Index(i)) = (yaml, next) {
        if let Some(yaml) = values.get_mut(*i) {
            set_field(
                yaml,
                &path.clone().into_iter().skip(1).collect(),
                field,
                create,
            );
        }
        return true;
    }
    false
}

pub fn set_field_in_mapping(
    yaml: &mut serde_yaml::Value,
    path: &Path,
    next: &PathEntry,
    field: &serde_yaml::Value,
    create: bool,
) -> bool {
    if let (serde_yaml::Value::Mapping(mapping), PathEntry::Field(name)) = (yaml, next) {
        let index = serde_yaml::Value::String(name.to_string());
        if create {
            let entry = mapping.entry(index).or_insert(serde_yaml::Value::Null);
            set_field(
                entry,
                &path.clone().into_iter().skip(1).collect(),
                field,
                create,
            );
        } else {
            if let Some(yaml) = mapping.get_mut(index) {
                set_field(
                    yaml,
                    &path.clone().into_iter().skip(1).collect(),
                    field,
                    create,
                );
            }
        }
        return true;
    }
    false
}

pub fn set_field(
    yaml: &mut serde_yaml::Value,
    path: &Path,
    field: &serde_yaml::Value,
    create: bool,
) {
    if let Some(next) = path.first() {
        if set_field_in_sequence(yaml, path, next, field, create) {
            return;
        }
        if set_field_in_mapping(yaml, path, next, field, create) {
            return;
        }
        if create {
            match next {
                PathEntry::Index(i) => {
                    *yaml = serde_yaml::Value::Sequence(serde_yaml::Sequence::new());
                    set_field(yaml, path, field, create);
                }
                PathEntry::Field(name) => {
                    *yaml = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                    set_field(yaml, path, field, create);
                }
            }
        }
    } else {
        *yaml = field.clone();
    }
}

pub fn as_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(number) => Some(number.to_string()),
        serde_yaml::Value::String(text) => Some(text.to_string()),
        serde_yaml::Value::Tagged(tagged_value) => as_string(&tagged_value.value),
        _ => None,
    }
}

#[derive(Debug)]
pub struct YamlField<'a> {
    pub path: Path,
    pub value: &'a serde_yaml::Value,
}

pub fn as_sequence(field: YamlField<'_>) -> Vec<YamlField<'_>> {
    if let serde_yaml::Value::Sequence(sequence) = field.value {
        return sequence
            .iter()
            .map(|value| YamlField {
                path: field.path.clone(),
                value,
            })
            .collect();
    }
    return vec![field];
}

pub fn get_fields<'a>(
    yaml: &serde_yaml::Value,
    query: impl Into<QueryPath<'a>>,
    mut path: Path,
) -> Vec<YamlField<'_>> {
    let mut query = query.into();
    if query.has_next() {
        match yaml {
            serde_yaml::Value::Sequence(values) => {
                let mut fields = vec![];
                for (i, value) in values.iter().enumerate() {
                    let mut value_path = path.clone();
                    value_path.push(PathEntry::Index(i));

                    fields.append(&mut get_fields(value, query.clone(), value_path));
                }
                return fields;
            }
            serde_yaml::Value::Mapping(mapping) => {
                let fieldname = query.next().unwrap();
                if let Some(field) = mapping.get(serde_yaml::Value::String(fieldname.to_string())) {
                    path.push(PathEntry::Field(fieldname.to_string()));
                    return get_fields(field, query, path);
                }
                return vec![];
            }
            _ => {
                return vec![];
            }
        }
    }
    return vec![YamlField { path, value: yaml }];
}

pub fn get_field<'a>(
    yaml: &serde_yaml::Value,
    query: impl Into<QueryPath<'a>>,
    mut path: Path,
) -> Option<YamlField<'_>> {
    let mut query = query.into();
    if query.has_next() {
        match yaml {
            serde_yaml::Value::Sequence(values) => {
                return None;
            }
            serde_yaml::Value::Mapping(mapping) => {
                let fieldname = query.next().unwrap();
                if let Some(field) = mapping.get(serde_yaml::Value::String(fieldname.to_string())) {
                    path.push(PathEntry::Field(fieldname.to_string()));
                    return get_field(field, query, path);
                }
                return None;
            }
            _ => {
                return None;
            }
        }
    }
    return Some(YamlField { path, value: yaml });
}

#[derive(Debug, Clone)]
pub struct QueryPath<'a> {
    pub index: usize,
    pub parts: Vec<&'a str>,
}

impl<'a> QueryPath<'a> {
    pub fn next(&mut self) -> Option<&'a str> {
        let next = self.parts.get(self.index).map(|v| *v);
        if next.is_some() {
            self.index += 1;
        }
        return next;
    }

    pub fn has_next(&self) -> bool {
        self.parts.get(self.index).is_some()
    }
}

impl<'a> From<&'a str> for QueryPath<'a> {
    fn from(value: &'a str) -> Self {
        let parts: Vec<&str> = value.split('/').filter(|s| !s.is_empty()).collect();

        return QueryPath { index: 0, parts };
    }
}

pub fn path_from_str(value: &str) -> Path {
    value
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|x| PathEntry::Field(x.to_string()))
        .collect()
}
