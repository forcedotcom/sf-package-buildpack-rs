#![allow(dead_code)]
use anyhow::anyhow;
use libcnb::{join, read_file, write_file};
use openssl::symm::Cipher;
use std::env;
use std::path::PathBuf;

pub struct EncFile {
    pub key: String,
    pub iv: String,
    pub file: PathBuf,
    pub cipher: Cipher,
}

impl EncFile {
    pub fn new(file: PathBuf, key: String, iv: String) -> Result<Self, anyhow::Error> {
        if !file.is_file() {
            Err(anyhow!(
                "Encrypted file {} not found",
                file.to_str().unwrap()
            ))
        } else {
            Ok(EncFile {
                key,
                iv,
                file,
                cipher: Cipher::aes_256_cbc(),
            })
        }
    }

    pub fn from_env(file: PathBuf) -> Result<Self, anyhow::Error> {
        let key = env::var("OPENSSL_ENC_KEY");
        let iv = env::var("OPENSSL_ENC_IV");
        if key.is_err() || iv.is_err() {
            Err(anyhow!("OPENSSL_ENC_KEY and OPENSSL_ENC_IV are required for encrypting and decrypting files"))
        } else {
            EncFile::new(file, key.unwrap(), iv.unwrap())
        }
    }
}

pub(crate) fn encrypt(enc: &EncFile, target: &PathBuf) -> Result<(), anyhow::Error> {
    let key = hex::decode(&enc.key)?;
    let iv = hex::decode(&enc.iv)?;

    let data = read_file(&enc.file).unwrap();
    let enc_data = openssl::symm::encrypt(
        enc.cipher,
        key.as_slice(),
        Some(iv.as_slice()),
        data.as_slice(),
    )?;
    let base64_data = base64::encode(enc_data);
    write_file(base64_data.as_bytes(), target);
    Ok(())
}

pub(crate) fn decrypt(enc: &EncFile, target: &PathBuf) -> Result<(), anyhow::Error> {
    let key = hex::decode(&enc.key)?;
    let iv = hex::decode(&enc.iv)?;

    let file_data = read_file(&enc.file).unwrap();
    let enc_data = base64::decode(&file_data)
        .unwrap_or(base64::decode(join(&file_data)?).unwrap_or(file_data));

    let data = openssl::symm::decrypt(
        enc.cipher,
        key.as_slice(),
        Some(iv.as_slice()),
        enc_data.as_slice(),
    )?;
    write_file(data.as_slice(), target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use libcnb::read_file_to_string;
    use std::env;
    use tempfile::tempdir;

    fn setup() {
        env::set_var(
            "OPENSSL_ENC_KEY",
            "C639A572E14D5075C526FDDD43E4ECF6B095EA17783D32EF3D2710AF9F359DD4",
        );
        env::set_var("OPENSSL_ENC_IV", "D09A4D2C5DC39843FE075313A7EF2F4C");
    }

    #[test]
    fn it_loads_from_env() {
        setup();
        let result = EncFile::from_env(PathBuf::new());
        assert!(result.is_err(), "No such file, so error should be thrown");
    }

    #[test]
    fn it_encrypts_and_decrypts() {
        setup();

        let temp_dir = tempdir().unwrap();
        let home = temp_dir.as_ref();
        let file = home.join("dummy.key");
        let content = "Whoa this is some content.";
        write_file(content.as_bytes(), &file);

        let enc_file = home.join("dummy.key.enc");
        let unenc_file = home.join("dummy.key.unenc");

        encrypt(&EncFile::from_env(file).unwrap(), &enc_file).unwrap();
        decrypt(&EncFile::from_env(enc_file).unwrap(), &unenc_file).unwrap();

        let text = read_file_to_string(unenc_file).unwrap();
        assert_eq!(content, text.as_str());
    }
}
