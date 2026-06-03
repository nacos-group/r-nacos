use std::io::Read;

use flate2::read::GzEncoder;

pub fn get_md5(content: &str) -> String {
    let digest = md5::compute(content.as_bytes());
    format!("{:x}", digest)
}

pub fn select_option_by_clone<T>(a: &Option<T>, b: &Option<T>) -> Option<T>
where
    T: Clone,
{
    a.as_ref().cloned().or_else(|| b.clone())
}

pub fn option_owned_by_clone<T>(a: Option<&T>) -> Option<T>
where
    T: Clone,
{
    a.map(|v| v.to_owned())
}

pub fn merge_option<T>(a: Option<T>, b: Option<T>) -> Option<T> {
    a.or(b)
}

pub fn get_bool_from_string(s: &Option<String>, default: bool) -> bool {
    if let Some(s) = s {
        if s.eq_ignore_ascii_case("true") {
            return true;
        }
        if s.is_empty() {
            return default;
        }
        false
    } else {
        default
    }
}

pub fn gz_encode(data: &[u8], threshold: usize) -> Vec<u8> {
    if data.len() <= threshold {
        data.to_vec()
    } else {
        let mut result = Vec::new();
        let mut z = GzEncoder::new(data, flate2::Compression::fast());
        z.read_to_end(&mut result).unwrap();
        result
    }
}
