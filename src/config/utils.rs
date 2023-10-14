pub struct ParamUtils;
const VALID_CHARS: [char; 4] = ['_', '-', '.', ':'];
const TENANT_MAX_LEN: usize = 128;

impl ParamUtils {
    pub fn check_tenant( tenant:&Option<String>) -> anyhow::Result<()>{
        if let Some(t) = tenant  {
                if !t.is_empty() {
                if !Self::is_valid(&t.trim()) {
                return Err(anyhow::anyhow!("invalid tenant"));
                }
                if t.len() > TENANT_MAX_LEN {
                    return Err(anyhow::anyhow!("Too long tenant, over 128"));
                }
            }
        }
        Ok(())
    }

    fn is_valid_char(ch: char) -> bool {
        VALID_CHARS.iter().any(|&c| c == ch)
    }

    pub fn is_valid(param: &str) -> bool {
        if param.is_empty() {
            return false;
        }
        for ch in param.chars() {
            if !ch.is_alphanumeric() && !Self::is_valid_char(ch) {
                return false;
            }
        }
        true
    }
    
}