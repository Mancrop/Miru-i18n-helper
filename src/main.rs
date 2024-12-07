mod json_handler;
mod tencent_translate;
mod translate;
use clap::{Parser, ValueEnum};
use colored::Colorize;

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
    #[clap(short, long, help = "Target language", default_value = "zh")]
    dst: String,
    #[clap(short, long, help = "folder path to json files", default_value = ".")]
    path: String,
    #[clap(short, long, help = "Translator", value_enum, default_value = "tencent")]
    translator: Translator,
}

fn main() {
    let args: Args = Args::parse();
    let translator = args.translator.get_translator();
    let src_lang = &args.src;
    let dst_lang = &args.dst;
    let root_path = &args.path;
    let result = json_handler::handle_json_translate(root_path, src_lang, dst_lang, &translator);
    if let Err(e) = result {
        let error_str = format!("{e}");
        println!("{}", error_str.red());
    } else {
        println!("{}", "Translate success".green());
    }
}
