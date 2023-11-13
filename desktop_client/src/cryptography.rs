/*
* Copyright (C) 2023 Rastislav Kish
*
* This program is free software: you can redistribute it and/or modify
* it under the terms of the GNU General Public License as published by
* the Free Software Foundation, version 3.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
* GNU General Public License for more details.
*
* You should have received a copy of the GNU General Public License
* along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm,
    };
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;
use rand::{RngCore, SeedableRng, rngs::StdRng};

use argon2::{
    password_hash::{PasswordHasher, Salt},
    Argon2,
    };

use anyhow::bail;
use base64::{Engine, engine::general_purpose as base64_eng};

/// Encrypts a string with password and returns the encrypted data encoded as base64.
/// The encryption algorithm is AES256GCM with PBKDF2 HMAC SHA256 key derivation function set to 700000 iterations.
pub fn encrypt(content: &str, password: &str) -> Result<String, anyhow::Error> {
    let mut rng=StdRng::from_entropy();
    let mut salt=[0u8; 16];
    let mut nonce=[0u8; 12];
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce);

    let key=pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), &salt, 700000);

    let cipher=Aes256Gcm::new(&key.into());
    let encrypted=match cipher.encrypt(&nonce.into(), content.as_bytes().as_ref()) {
        Ok(c) => c,
        Err(e) => bail!("Unable to decrypt data. {e}"),
        };

    let mut result: Vec<u8>=vec![0; 16+12+encrypted.len()];
    result[0..16].clone_from_slice(&salt);
    result[16..28].clone_from_slice(&nonce);
    result[28..].clone_from_slice(&encrypted);

    Ok(base64_eng::STANDARD_NO_PAD.encode(result))
    }

/// Decrypts a base64 encoded data with password and returns the decrypted string. Note the plain-data has to be utf-8 text, otherwise the function will error out.
/// The encryption algorithm is AES256GCM with PBKDF2 HMAC SHA256 key derivation function set to 700000 iterations.
pub fn decrypt(content: &str, password: &str) -> Result<String, anyhow::Error> {
    let content=base64_eng::STANDARD_NO_PAD.decode(content)?;

    let salt=&content[0..16];
    let nonce=&content[16..28];
    let key=pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), salt, 700000);

    let cipher=Aes256Gcm::new(&key.into());
    let decrypted=match cipher.decrypt(nonce.into(), content[28..].as_ref()) {
        Ok(c) => c,
        Err(e) => bail!("Unable to decypt data. {e}"),
        };

    Ok(std::str::from_utf8(&decrypted)?.to_string())
    }

/// Calculates a string hash, returning the hash in url-safe base64 format (+ -> -, / -> _)
/// This function uses a hard-code static salt value in order to make the hashes suitable for passwordless identification. DO NOT use for other purposes
/// The hashing algorithm used is Argon2 id v19
pub fn calculate_pseudosalted_password_hash(password: &str) -> String {
    let argon2=Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::default(),
        );

    let password_hash=argon2.hash_password(password.as_bytes(), Salt::from_b64("5eVS51U/D9XXhWK37D6qAg").unwrap()).unwrap();
    let hash=password_hash.hash.unwrap().to_string();

    hash.replace('+', "-").replace('/', "_")
    }

