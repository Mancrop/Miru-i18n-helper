mod json_handler;
mod tencent_translate;
mod translate;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use tabled::{
    settings::{Alignment, Style},
    Tabled,
};

#[derive(Clone, ValueEnum, Debug)]
#[non_exhaustive]
enum Translator {
    Tencent,
}

impl Translator {
    fn get_translator(&self) -> impl translate::Translate {
        match self {
            Translator::Tencent => tencent_translate::TencentTranslate::new(),
        }
    }
}

#[derive(Parser, Debug)]
// #[command(name = "translate", about = "Translate text")]
struct Args {
    #[clap(short, long, help = "Source language", default_value = "en")]
    src: String,
    #[clap(
        short,
        long,
        help = "Target languagem, \"all\" for all json in root path",
        default_value = "zh"
    )]
    dst: String,
    #[clap(short, long, help = "folder path to json files", default_value = ".")]
    path: String,
    #[clap(
        short,
        long,
        help = "Translator",
        value_enum,
        default_value = "tencent"
    )]
    translator: Translator,
    #[clap(short, long, help = "idle time", default_value = "200")]
    idle: u64,
}

#[derive(Tabled)]
struct CliResult {
    file: String,
    msg: String,
    status: String,
}

impl CliResult {
    fn new(dst: &str, res: &Result<(), json_handler::Error>) -> Self {
        let status = if res.is_ok() { "Success".green() } else { "Failed".red() };
        let msg = if let Err(e) = res {
            let err = format!("{e}");
            err.red()
        } else {
            "-".green()
        };
        Self {
            file: dst.to_string(),
            msg: msg.to_string(),
            status: status.to_string(),
        }
    }
}

fn get_all_lang(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut langs = Vec::new();
    let paths = std::fs::read_dir(path)?;
    for path in paths {
        let path = path?.path();
        if let Some(ext) = path.extension() {
            if ext == "json" {
                if let Some(file) = path.file_stem() {
                    let lang = file.to_str().map(String::from);
                    if let Some(lang) = lang {
                        langs.push(lang);
                    }
                }
            }
        }
    }
    Ok(langs)
}

fn main() -> Result<(), ()> {
    let args: Args = Args::parse();
    let translator = args.translator.get_translator();
    let src_lang = &args.src;
    // let dst_lang = &args.dst;
    let root_path = &args.path;
    let langs = if args.dst == "all" {
        match get_all_lang(root_path) {
            Ok(v) => v,
            Err(e) => {
                let err = format!("Error: {e}");
                println!("{}", err.red());
                return Err(());
            }
        }
    } else {
        vec![args.dst.clone()]
    };
    let mut res_vec = Vec::new();

    if langs.is_empty() {
        println!("{}", "No json files found".red());
        return Err(());
    }
    let langs_info = format!("Found {} types of languages, {:?}", langs.len(), langs);
    println!("{}", langs_info.blue());

    for dst_lang in &langs {
        if dst_lang == src_lang {
            let info = format!("Skip translating from {src_lang} to {dst_lang}");
            println!("{}", info.blue());
            res_vec.push(Ok(()));
            continue;
        }
        let info = format!("Translating from {src_lang} to {dst_lang}");
        println!("{}", info.blue());
        let result =
            json_handler::handle_json_translate(root_path, src_lang, dst_lang, &translator, args.idle);
        res_vec.push(result);
    }
    let table_vec = res_vec
        .iter()
        .zip(langs.iter())
        .map(|(res, lang)| CliResult::new(lang, res))
        .collect::<Vec<_>>();

    let table = tabled::Table::new(&table_vec)
        .with(Style::modern())
        .with(Alignment::center())
        .to_string();
    println!("{table}");

    Ok(())
}
