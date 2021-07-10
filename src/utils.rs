
use crypto::digest::Digest;

pub fn get_md5(content:&str) -> String {
    let mut m = crypto::md5::Md5::new();
    m.input_str(content);
    m.result_str()
}

pub fn select_option_by_clone<T>(a:&Option<T>,b:&Option<T>) -> Option<T>
    where T:Clone 
{
    match a {
        Some(_a) => {
            Some(_a.clone())
        },
        None => b.clone()
    }
}

pub fn option_owned_by_clone<T>(a:Option<&T>) -> Option<T> 
    where T:Clone
{
    match a{
        Some(_a) => {
            Some(_a.clone())
        },
        None => None
    }
}