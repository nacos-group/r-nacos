
use crypto::digest::Digest;

pub fn get_md5(content:&str) -> String {
    let mut m = crypto::md5::Md5::new();
    m.input_str(content);
    m.result_str()
}