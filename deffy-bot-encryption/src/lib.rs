pub struct EncrytionHelper;

impl EncrytionHelper {

    pub fn encrypt(data: &str) -> String {
        // Placeholder for encryption logic
        format!("encrypted_{}", rbase64::encode(data.as_bytes()))
    }

    pub fn decrypt(data: &str) -> String {
        // Placeholder for decryption logic
        data.replace("encrypted_", "")
    }
}