use std::env;
use std::fs::File;
use std::io::Read;
use std::ops::Index;
use std::path::Path;
use std::io::Write;
use base64::Engine;
use base64::engine::general_purpose;


fn load_source(src: &str, base_path: &Option<&Path>) -> Option<Vec<u8>> {
    if src.starts_with("http") {
        let req = reqwest::Client::new()
            .get(src)
            .build()
            .map_err(|e|eprintln!("{e}"))
            .ok()?;
        //let encoding = req.headers().get("Content-Type");
        return req.body()?.as_bytes().map(|b|b.to_vec())
    }

    let mut file = (||{
        let p1 = Path::new(src);
        if p1.exists()  {
            return Some(File::open(p1))
        }
        return if let Some(base) = base_path {
            let p2 = base.join(p1);
            if p2.exists() {
                Some(File::open(p2))
            } else {
                None
            }
        } else {
            None
        }
    })()?.map_err(|e|eprintln!("{e}")).ok()?;

    let mut data = vec![];
    file.read_to_end(&mut data).map_err(|e|eprintln!("{e}")).ok()?;
    return Some(data)
}

fn load_binary_source(src: &str, base_path: &Option<&Path>) -> Option<String> {
    let data = load_source(src, base_path)?;
    let base64data = general_purpose::STANDARD_NO_PAD.encode(&data);

    let ext = src.split(".").last().and_then(|s| {
        match s.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some("image/jpeg"),
            "png" => Some("image/png"),
            "webp" => Some("image/webp"),
            "gif" => Some("image/gif"),
            _ => None
        }
    }).unwrap_or("unknown");

    Some(format!("data:{ext};base64, {base64data}"))
}
fn load_string_source(src: &str, base_path: &Option<&Path>) -> Option<String> {
    let data = load_source(src, base_path)?;
    String::from_utf8(data)
        .map_err(|e|eprintln!("{e}"))
        .ok()
}

fn main() {
    let filename = env::args().nth(1).expect("No input file given");
    let filename_out = env::args().nth(2).expect("No output file given");
    let mut file = File::open(&filename).expect("Can't open file");
    let mut data: String = String::new();
    file.read_to_string(&mut data).expect("Can't read file");
    let img_expr_src = regex::RegexBuilder::new(r#"<img.*?src="(.*?)".*?/>"#)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let script_expr_src = regex::RegexBuilder::new(r#"<script.*?src="(.*?)".*?></script>"#)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let css_expr_src = regex::RegexBuilder::new(r#"<link.*?href="(.*?.css)".*?>"#)
        .dot_matches_new_line(true)
        .build()
        .unwrap();
    let mut replacements: Vec<(&str, String)> = vec![];

    let base_path = Path::new(&filename).parent();
    for (full_match, [src]) in script_expr_src.captures_iter(&data).map(|c| c.extract()) {
        eprintln!("Found script source '{src}'");
        if let Some(body) = load_string_source(src, &base_path) {
            replacements.push((full_match, format!("<script type=\"application/javascript\">\n{body}\n</script>")));
        } else {
            eprintln!("Problem loading body for the source '{src}' skip it.");
        }
    }
    for (full_match, [src]) in css_expr_src.captures_iter(&data).map(|c| c.extract()) {
        eprintln!("Found style source '{src}'");
        if let Some(body) = load_string_source(src, &base_path) {
            replacements.push((full_match, format!("<style type=\"text/css\">\n{body}\n</style>")));
        } else {
            eprintln!("Problem loading body for the source '{src}' skip it.");
        }
    }
    for (full_match, [src]) in img_expr_src.captures_iter(&data).map(|c| c.extract()) {
        eprintln!("Found image source '{src}'");
        if let Some(body) = load_binary_source(src, &base_path) {
            replacements.push((full_match, format!("<img src=\"{body}\"/>")));
        } else {
            eprintln!("Problem loading body for the source '{src}' skip it.");
        }
    }
    let mut data_out: String = data.clone();
    for (original, replacement) in replacements.into_iter() {
        //eprintln!("Replace {original} with {}", &replacement);
        data_out = data_out.replace(original, &replacement);
    }
    let mut fileout = File::create(filename_out).unwrap();
    write!(fileout, "{data_out}").unwrap();
}
