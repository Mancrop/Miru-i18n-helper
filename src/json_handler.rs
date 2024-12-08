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
    idle: u64,
) -> Result<(), Error> {
    fn translate_json(
        src: &Map<String, Value>,
        dst: &mut Map<String, Value>,
        ref_json: &Map<String, Value>,
        src_lang: &str,
        dst_lang: &str,
        translator: &impl translate::Translate,
        idle: u64,
    ) -> MyResult<()> {
        for (key, value) in src {
            // println!("key: {key}, value: {value}");
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
                        translate(s, src_lang, dst_lang, translator, idle)?
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
                        idle,
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
        idle,
    )?;

    write_json(&dst_path, &dst_json).or(Err(Error::new(
        ErrorType::WriteJsonError,
        "Write Json Error",
    )))?;

    Ok(())
}

fn translate(
    text: &str,
    src_lang: &str,
    dst_lang: &str,
    translator: &impl translate::Translate,
    idle: u64,
) -> MyResult<String> {
    let placeholder_pattern = regex::Regex::new(r"\{[^}]+\}").unwrap();
    let mut result = String::new();
    let mut last_end = 0;

    // 分段翻译括号前的内容
    for mat in placeholder_pattern.find_iter(text) {
        let start = mat.start();

        // 翻译括号前的文本
        if start > last_end {
            let text_before = text[last_end..start].trim();
            if !text_before.is_empty() {
                let translated = translator
                    .translate(src_lang, dst_lang, text_before, idle)
                    .cast(ErrorType::TranslateError)?;
                result.push_str(&translated);
                result.push(' '); // 添加空格以保持可读性
            }
        }

        // 添加括号内容
        result.push_str(mat.as_str());
        last_end = mat.end();
    }

    // 处理最后一个括号后的文本
    if last_end < text.len() {
        let text_after = text[last_end..].trim();
        if !text_after.is_empty() {
            let translated = translator
                .translate(src_lang, dst_lang, text_after, idle)
                .cast(ErrorType::TranslateError)?;
            result.push(' '); // 添加空格以保持可读性
            result.push_str(&translated);
        }
    }

    // 如果文本中没有括号，直接翻译整个文本
    if result.is_empty() {
        result = translator
            .translate(src_lang, dst_lang, text, idle)
            .cast(ErrorType::TranslateError)?;
    }

    // 清理多余的空格
    result = result.trim().to_string();

    Ok(result)
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
        let result = handle_json_translate(root_path, src_lang, dst_lang, &translator, 300);
        if let Err(e) = result {
            println!("{e}");
            panic!();
        }
    }
}
