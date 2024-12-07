use crate::translate;
use colored::Colorize;
use serde_json::{json, Map, Value};

#[derive(Debug)]
#[allow(unused)]
pub enum ErrorType {
    // Error when reading json file
    ReadJsonError,
    // Error when writing json file
    WriteJsonError,
    // Error when translating
    TranslateError,
    // Unsupported json type
    UnsupportedJsonType,
    // Unknown error
    UnknownError,
}

#[derive(Debug)]
pub struct Error {
    error_type: ErrorType,
    message: String,
}

impl Error {
    fn new(error_type: ErrorType, message: &str) -> Self {
        Error {
            error_type,
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {:?}, message: {}", self.error_type, self.message)
    }
}

trait ErrorCast<T, E: ToString>: Sized + Into<Result<T, E>> {
    fn cast(self, error_type: ErrorType) -> Result<T, Error> {
        self.into().map_err(|e| Error {
            error_type,
            message: e.to_string(),
        })
    }
}

impl<T, E: std::error::Error> ErrorCast<T, E> for Result<T, E> {}
impl<T> ErrorCast<T, translate::Error> for Result<T, translate::Error> {}

type MyResult<T> = std::result::Result<T, Error>;

fn read_json(path: &str) -> MyResult<Value> {
    let file = std::fs::File::open(path).cast(ErrorType::ReadJsonError)?;
    let reader = std::io::BufReader::new(file);
    let v = serde_json::from_reader(reader).cast(ErrorType::ReadJsonError)?;
    Ok(v)
}

fn write_json(path: &str, json_map: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(file, &json_map)?;
    Ok(())
}

// fn insert_into_json(json_obj: &mut Value, keys: Vec<&str>, value: Value) {
//     let mut current = json_obj;

//     for key in keys.iter().take(keys.len() - 1) {
//         if !current.is_object() {
//             *current = json!({});
//         }
//         if !current[key].is_object() {
//             current[key] = json!({});
//         }
//         current = &mut current[key];
//     }

//     let last_key = keys.last().unwrap().to_string();
//     current[last_key] = value;
// }

pub fn handle_json_translate(
    root_path: &str,
    src_lang: &str,
    dst_lang: &str,
    translator: &impl translate::Translate,
) -> Result<(), Error> {
    fn translate_json(
        src: &Map<String, Value>,
        dst: &mut Map<String, Value>,
        ref_json: &Map<String, Value>,
        src_lang: &str,
        dst_lang: &str,
        translator: &impl translate::Translate,
    ) -> MyResult<()> {
        for (key, value) in src {
            println!("key: {key}, value: {value}");
            // ignore languages settings
            if key == "languages" {
                dst.insert(key.to_string(), value.clone());
                continue;
            }
            match value {
                Value::String(s) => {
                    let translated = if let Some(Value::String(original)) = ref_json.get(key) {
                        original.clone()
                    } else {
                        translate_with_placeholders(s, src_lang, dst_lang, translator, 300)?
                    };
                    dst.insert(key.to_string(), Value::String(translated));
                    continue;
                }
                Value::Object(o) => {
                    if !dst.contains_key(key) {
                        dst.insert(key.to_string(), json!({}));
                    } else if !dst[key].is_object() {
                        dst[key] = json!({});
                    }
                    let empty_json = json!({});
                    let new_ref_json = if let Some(Value::Object(original)) = ref_json.get(key) {
                        original
                    } else {
                        empty_json.as_object().unwrap()
                    };
                    translate_json(
                        o,
                        dst[key].as_object_mut().unwrap(),
                        new_ref_json,
                        src_lang,
                        dst_lang,
                        translator,
                    )?;
                }
                _ => {
                    return Err(Error::new(
                        ErrorType::UnsupportedJsonType,
                        "Unsupported json type",
                    ));
                }
            }
        }
        Ok(())
    }

    let src_path = format!("{root_path}/{src_lang}.json");
    let dst_path = format!("{root_path}/{dst_lang}.json");
    let src_json = read_json(&src_path)?;
    let mut dst_json = json!({});
    let dst_json_original = read_json(&dst_path);
    let dst_json_original = if let Err(err_msg) = dst_json_original {
        let fmt_str = format!("Warning: read target json error --> {err_msg}");
        println!("{}", fmt_str.yellow());
        json!({})
    } else {
        dst_json_original.unwrap()
    };

    translate_json(
        src_json.as_object().unwrap(),
        dst_json.as_object_mut().unwrap(),
        dst_json_original.as_object().unwrap(),
        src_lang,
        dst_lang,
        translator,
    )?;

    write_json(&dst_path, &dst_json).or(Err(Error::new(
        ErrorType::WriteJsonError,
        "Write Json Error",
    )))?;

    Ok(())
}

fn translate_with_placeholders(
    text: &str,
    src_lang: &str,
    dst_lang: &str,
    translator: &impl translate::Translate,
    idle: u64,
) -> MyResult<String> {
    let placeholder_pattern = regex::Regex::new(r"\{[^}]+\}").unwrap();
    let mut placeholders = Vec::new();
    let mut temp_text = text.to_string();

    for (i, mat) in placeholder_pattern.find_iter(text).enumerate() {
        let placeholder = mat.as_str();
        placeholders.push(placeholder.to_string());
        temp_text = temp_text.replace(placeholder, &format!("__PLACEHOLDER_{i}__"));
    }

    let translated = translator
        .translate(src_lang, dst_lang, &temp_text, idle)
        .cast(ErrorType::TranslateError)?;

    let mut final_text = translated;
    for (i, placeholder) in placeholders.iter().enumerate() {
        final_text = final_text.replace(&format!("__PLACEHOLDER_{i}__"), placeholder);
    }

    Ok(final_text)
}

#[cfg(test)]
mod test {
    #[allow(unused)]
    use super::*;

    #[test]
    fn test_handle_json_translate() {
        let root_path = "./mytest";
        let src_lang = "en";
        let dst_lang = "zh";
        let translator = crate::tencent_translate::TencentTranslate::new();
        let result = handle_json_translate(root_path, src_lang, dst_lang, &translator);
        if let Err(e) = result {
            println!("{e}");
            panic!();
        }
    }
}
