pub struct EncrytionHelper;

impl EncrytionHelper {

    pub fn encrypt(&self, data: &str) -> String {
        // Placeholder for encryption logic
        format!("encrypted_{}", data)
    }

    pub fn decrypt(&self, data: &str) -> String {
        // Placeholder for decryption logic
        data.replace("encrypted_", "")
    }
}