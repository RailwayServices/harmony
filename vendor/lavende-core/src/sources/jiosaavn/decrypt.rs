use base64::prelude::*;
use des::cipher::{BlockDecrypt, KeyInit, generic_array::GenericArray};

pub fn decrypt_url(encrypted: &str, secret_key: &[u8]) -> Option<String> {
    if secret_key.len() != 8 {
        return None;
    }
    let cipher = des::Des::new_from_slice(secret_key).ok()?;
    let mut data = BASE64_STANDARD.decode(encrypted).ok()?;
    for chunk in data.chunks_mut(8) {
        if chunk.len() == 8 {
            cipher.decrypt_block(GenericArray::from_mut_slice(chunk));
        }
    }
    if let Some(&last_byte) = data.last() {
        let padding = last_byte as usize;
        if (1..=8).contains(&padding) && data.len() >= padding {
            data.truncate(data.len() - padding);
        }
    }
    String::from_utf8(data).ok()
}
