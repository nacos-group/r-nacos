use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::{engine::general_purpose, Engine};

type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;
type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

pub fn decode_base64(data: &str) -> anyhow::Result<Vec<u8>> {
    Ok(general_purpose::STANDARD.decode(data)?)
}

pub fn encode_base64(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// 加密
/// key,iv长度需要是16的倍数
pub fn encrypt_aes128(key: &str, iv: &str, plain: &[u8]) -> anyhow::Result<Vec<u8>> {
    let pt_len = plain.len();
    let buf_len = if pt_len % 48 == 0 {
        pt_len
    } else {
        (48 - pt_len % 48) + pt_len
    };
    let mut buf = vec![0u8; buf_len];
    (buf[..pt_len]).copy_from_slice(plain);
    match Aes128CbcEnc::new(key.as_bytes().into(), iv.as_bytes().into())
        .encrypt_padded_b2b_mut::<Pkcs7>(plain, &mut buf)
    {
        Ok(ct) => Ok(ct.to_vec()),
        Err(e) => Err(anyhow::anyhow!("encrypt error,{}", &e)),
    }
}

/// 解密
/// key,iv长度需要是16的倍数
pub fn decrypt_aes128(key: &str, iv: &str, cipher: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher_len = cipher.len();
    let buf_len = if cipher_len % 48 == 0 {
        cipher_len
    } else {
        (48 - cipher_len % 48) + cipher_len
    };
    let mut buf = vec![0u8; buf_len];
    (buf[..cipher_len]).copy_from_slice(cipher);

    match Aes128CbcDec::new(key.as_bytes().into(), iv.as_bytes().into())
        .decrypt_padded_b2b_mut::<Pkcs7>(cipher, &mut buf)
    {
        Ok(pt) => Ok(pt.to_vec()),
        Err(e) => Err(anyhow::anyhow!("decrypt error,{}", &e)),
    }
}
