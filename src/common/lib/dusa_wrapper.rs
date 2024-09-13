use dusa_collection_utils::{errors::ErrorArrayItem, stringy::Stringy};

use crate::dusa::run;

pub fn encrypt_text(data: String) -> Result<Stringy, ErrorArrayItem> {
    match run(
        crate::dusa::ProgramMode::EncryptText,
        None,
        None,
        None,
        Some(data),
    )
    .uf_unwrap()
    {
        Ok(d) => match d {
            Some(d) => Ok(dusa_collection_utils::stringy::Stringy::Mutable(d)),
            None => {
                return Err(ErrorArrayItem::new(
                    dusa_collection_utils::errors::Errors::GeneralError,
                    String::from("No data received from dusa"),
                ))
            }
        },
        Err(mut e) => return Err(e.pop()),
    }
}

pub fn decrypt_text(data: Stringy) -> Result<Stringy, ErrorArrayItem> {
    match run(
        crate::dusa::ProgramMode::DecryptText,
        None,
        None,
        None,
        Some(data.to_string()),
    )
    .uf_unwrap()
    {
        Ok(d) => match d {
            Some(d) => Ok(dusa_collection_utils::stringy::Stringy::Mutable(d)),
            None => {
                return Err(ErrorArrayItem::new(
                    dusa_collection_utils::errors::Errors::GeneralError,
                    String::from("No data received from dusa"),
                ))
            }
        },
        Err(mut e) => return Err(e.pop()),
    }
}
