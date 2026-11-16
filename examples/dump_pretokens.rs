use neuratal_tokenizer::PretokenizationMode;
fn main() {
    let text = std::env::args().nth(1).expect("text");
    let mode = std::env::args().nth(2).unwrap_or_else(|| "cjk8".into());
    let mode = match mode.as_str() {
        "cjk8" => PretokenizationMode::MathSymbolAwareCjk8,
        "math" => PretokenizationMode::MathSymbolAware,
        "cjk4" => PretokenizationMode::MathSymbolAwareCjk4,
        "hybrid" => PretokenizationMode::UnicodeHybrid,
        other => panic!("unknown mode {other}"),
    };
    for t in mode.split_text(&text) {
        println!("{t:?}");
    }
}
