extern crate encryptfile;

use encryptfile as ef;

pub(crate) trait EncryptFile {
    fn encrypt(&self);
    fn decrypt(&self);
}

pub(crate) struct UploadedFile {
    pub(crate) file_name: String,
    pub(crate) saved_file: String,
    pub(crate) password: Option<String>,
    pub(crate) private_key: Option<String>,
}

impl EncryptFile for UploadedFile {
    fn encrypt(&self) {
        // Encrypt
        let in_file = self.file_name.clone();

        let mut c = ef::Config::new();
        let password = match self.password.clone() {
            None => {
                panic!("this module cannot encrypt without password")
            }
            Some(val) => val,
        };

        c.input_stream(ef::InputStream::File(in_file.to_owned()))
            .output_stream(ef::OutputStream::File(self.saved_file.to_owned()))
            .add_output_option(ef::OutputOption::AllowOverwrite)
            .initialization_vector(ef::InitializationVector::GenerateFromRng)
            .password(ef::PasswordType::Text(
                password.to_owned(),
                ef::scrypt_defaults(),
            ))
            .encrypt();
        let _ = ef::process(&c).map_err(|e| panic!("error encrypting: {:?}", e));
    }

    fn decrypt(&self) {
        // Decrypt
        let password = match self.password.clone() {
            None => {
                panic!("this module cannot decrypt without password")
            }
            Some(val) => val,
        };

        let mut c = ef::Config::new();
        c.input_stream(ef::InputStream::File(self.saved_file.to_owned()))
            .output_stream(ef::OutputStream::File(self.file_name.to_owned()))
            .add_output_option(ef::OutputOption::AllowOverwrite)
            .password(ef::PasswordType::Text(
                password.to_owned(),
                ef::PasswordKeyGenMethod::ReadFromFile,
            ))
            .decrypt();
        let _ = ef::process(&c).map_err(|e| panic!("error decrypting: {:?}", e));
    }
}
