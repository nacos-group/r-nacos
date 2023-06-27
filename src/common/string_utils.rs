
pub struct StringUtils;


impl StringUtils {
    pub fn is_empty(s:&str) -> bool {
        s.is_empty()
    }

    pub fn eq(a:&str,b:&str) -> bool {
        a==b
    }

    pub fn like(a:&str,b:&str) -> Option<usize> {
        a.rfind(b)
    }
}