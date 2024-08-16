pub struct OptionUtils;

impl OptionUtils {
    pub fn select<T>(a: Option<T>, b: Option<T>) -> Option<T>
    where
        T: Clone,
    {
        match a {
            Some(v) => Some(v),
            None => b,
        }
    }
}
