use serde::Deserialize;
use serde::de::{Deserializer, Error};

pub fn search(q: String) -> poll_promise::Promise<Vec<Place>> {
    let (sender, promise) = poll_promise::Promise::new();
    common::execute(async move {
        let mut url = reqwest::Url::parse("https://nominatim.openstreetmap.org/search").unwrap();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("format", "json");
            query.append_pair("q", &q);
        }
        let mut request = ehttp::Request::get(url);
        request.headers.insert("user-agent", "42");

        let res = common::http::fetch(&request)
            .await
            .unwrap();
        let res: Vec<Place> = serde_json::from_slice(&res.bytes).unwrap();
        sender.send(res);
    });
    return promise;
}

#[derive(Debug, Deserialize)]
pub struct Place {
    pub place_id: u64,
    pub licence: String,
    pub osm_type: String,
    pub osm_id: u64,
    #[serde(deserialize_with = "de_f64_from_str")]
    pub lat: f64,
    #[serde(deserialize_with = "de_f64_from_str")]
    pub lon: f64,
    pub class: String,
    #[serde(rename = "type")]
    pub place_type: String,
    pub place_rank: u32,
    pub importance: f64,
    pub addresstype: String,
    pub name: String,
    pub display_name: String,
    #[serde(deserialize_with = "de_vec_f64_from_strs")]
    pub boundingbox: Vec<f64>,
}

fn de_f64_from_str<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(D::Error::custom)
}

fn de_vec_f64_from_strs<'de, D>(deserializer: D) -> Result<Vec<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec = Vec::<String>::deserialize(deserializer)?;
    vec.into_iter()
        .map(|s| s.parse::<f64>().map_err(D::Error::custom))
        .collect()
}
