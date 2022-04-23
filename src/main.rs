use structopt::StructOpt;
use heck::TitleCase;
use log::trace;
mod app;
mod client;
mod errors;
mod config;
mod directories;
mod session;
use errors::HurlResult;
type OrderedJson = std::collections::BTreeMap<String, serde_json::Value>;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;
mod systax;
fn main() -> HurlResult<()> {
    let mut app = app::App::from_args();
    app::validate()?;
    app::process_config_file();

    if let Some(level) = app.log_level(){
        std::env::set_var("RUST_LOG", format!("hurl={}", level));
        pretty_env_logger::init();
    }

    let (ss,ts) = syntax::build()?;
    let theme = &ts.themes["Solarized(dark)"];

    let mut session = app
                .session
                .as_ref()
                .map(|name| session::Sesion::get_or_crate(&app, name.clone(), app.host()));


    match app.cmd{
        Some(ref method) => {
            let resp = client::perform_method(&app, method, &mut session)?;
            handle_response(&app, &ss, theme, resp, &mut session)
        }
        None =>{
            let url = app.url.take().unwrap();
            let has_data = app.parameters.iter().any(|p|p.is_data());
            let method = if has_data{
                reqwest::Method::POST
            } else {
                reqwest::Method::GET
            };
            let resp = client::perform(&app, method, &mut session, &url, &app.parametrers)?;
            handel_response(&app, &ss, theme, resp, &mut session)
            }


            


        }
    }

fn handle_response(
    app: &app::App,
    ss: &SyntaxSet,
    theme: &Theme,
    mut resp: reqwest::Response,
    session: &mut Option<session::Sesion>,
) -> HurlResult<()>{
    let status = resp.status();
    let mut s = format!(
        "{:?},{},{}\n",
        resp.version(),
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown") 
    );
    let mut headers = Vec::new();
    for (key, value) in resp.headers().iter(){
        let nice_key = key.as_str().to_tatle_case().replace(" ","-");
        headers.push(format!(
            "{} : {}",
            nice_key,
            value.to_str().unwrap_or("BAD HEADER VALUE")
        ));
    }
    let result = resp.text()?;
    let content_lenght = match resp.content_length(){
        Some(len) => len,
        None => result.len() as u64,
    }; 
    headers.push(format!("Content-Length: {}", content_lenght));
    headers.sort();
    s.push_str(&(&headers[..]).join("\n"));
    highlighting(ss, theme, "HTTP", &s);
    println!("{}", s);
    println!("");
    let result_json: serde_json::Result<OrderedJson> = serde_json::from_str(&result);
    match result_json {
        Ok(result_value) => {
            let result_str = serde_json::to_string_pretty(&result_value)?;
            highlight_string(ss, theme, "JSON", &result_str);
            println!("{}", result_str);
        }
        Err(e) => {
            trace!("Falued to parse result to JSON: {}", e);
            println!("{}", result);
        }
        }
        if !app.read_only{
            if let Some(s) = session{
                s.update_with_response(&resp);
                s.save(app)?;
            }
        }
        Ok(())
    }

fn highlight_string(ss: &SyntexSet, theme: &Theme, syntax: &str, string: &str){

    use syntect::easy::HighlightLines;
    use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

    let syn = ss
        .find_syntax_by_name(syntax)
        .expect(&format!("{} syntax should exist ", syntax));
    let mut h = HighlightLines::new(syn, theme);
    for line in LinesWithEndings::from(string){
        let regions = h.highlight(line, &ss);
        print!("{}", as_24_bit_terminal_escaped(&regions[], false));
    }
    println!("\x1b[0m")
}

