use log::{debug, trace};
use reqwest::Method;
use std::path::PathBuf;
use crate::config;
use std::convert::TryFrom;
use structopt::StructOpt;
use crate::errors::{Error, HurlResult};
use crate::session::make_safe_pathname;

///A comand line HTTP client 
#[derive(StructOpt, Debug)]
#[structopt(name = "hurl")]
pub struct App{
/// Activafte quite mode
/// 
/// This overriders anu verbose string
#[structopt(short, long)]
pub quiet: bool, 

/// Verbose mode (-v, -vv, etc.)
#[structopt(short, long, parse(from_occurrences))]
pub verbose: u8,

///From mode.
#[structopt(short, long)]
pub form:bool,

///Basic autifation
/// 
/// A string of the form 'usersname:pasword'. if only 
/// 'username' is given then you will be prompted
/// for a password. If you wish  to use no password 
/// then use form 'username:'
#[structopt(long, short)]
pub auth: Option<String>,

/// Bearer token autentification
/// 
/// A token wich will be sent as "Bearaer <token>" in 
/// the authorization header. 
#[structopt(short, long)]
pub token: Option<String>,
 ///Sesion name
 #[structopt(logs)]
 pub session: Option<String>,

 ///Sesion storage location
 #[structopt(long,parse(from_os_str))]
 pub session_dir: Option<PathBuf>,

 /// if true then use the store session to argument the request
 /// but not modify by stored
 #[structopt(long)]
 pub read_only: bool,
/// Default transport 
/// 
/// If URL given without a transport, i.e. example.com/foo
/// http will be used as transport as default. If this flag 
/// is set then https will be used instead. 
#[structopt(short, long)]
pub secure: bool,
///Configuration file
/// 
/// A TOML file which is stored by default at HOME/.config/hurl/confiig
/// where home is platform dependet.
/// The file supports the following optionals keys with the given types:
/// verbose: u8
/// form: bool
/// auth: string
/// token: string
/// secure: bool
/// 
/// 
/// 
/// 
/// Each optional has the same meaning as the same corresponding configuration
/// optioal with the same name. The verbose setting is a number from 0
/// meaning no logging to 5 meaning as the coresponding configuration 
/// 
/// 
#[structopt(short,long, env="HURL_CONFIG", parse(from_os_str))]
pub config: Option<PathBuf>,
/// The HTTP method to use: GET, POST, HEAD, PUT, PATCH, DELETE
#[structopt(subcommand)]
pub cmd: Option<Method>, 

/// The URL to issue a request to if a method subcomand is not specified
pub url: Option<String>,
/// The parameters for the request if a method subcommand is not specified
/// 
/// There are seven types of parameters that can be added to a command-line
/// Each type of parameter is distinguished by the unique separator between 
/// the key and value
/// 
/// Header --key value 
/// 
/// e.a. X-API-TOKEN:abc123
/// 
/// File upload -- key@filename 
/// 
/// this simulated a file upload via multipart/from-data and requires --from 
/// 
/// Query parameter --key= value 
/// 
/// e.g. foo = bar becomes  {"foo":"bar"} for JSON or form encoded 
/// 
/// Data field from file -- key=@filename
/// 
/// e.g. foo = @bar.txt becomes {"foo":"the content of bar.txt"} or from encoded
/// 
/// Raw JSON data where the value should be parsed to JSON first --key:value
/// 
/// e.g. foo:= [1,2,3] become {"foo:[1,2,3]"}
/// 
/// Raw JSON data from file -- key:=@filename
/// 
/// e.g. foo:=@bar.json becomes {"foo":{"bar":this is from bar.json}}

#[structopt(parse(try_from_str = parse_param))]
pub parametrs: Vec<Parameter>, 
}




impl App{
    pub fn validate(&mut self)-> HurlResult{
        if self.cmd.is_none() && self.url.is_none(){
            return Err(Error::MissingUrlAndCommand);
        }
        Ok(())
    }
    pub fn process_config_file(&mut self){
        let config_path = config::config_file(self);
        let config_opt = config::read_config_file(config_path);
        if let Some(mut config) = config_opt{
            if self.verbose == 0{
                if let Some(v) = config.verbose {
                    self.verbose = v;
                }
            }
            if !self.form{
                if let Some(f) = config.form{
                    self.secure = f;    
                }
            }
            if !self.secure {
                if let Some(s) = config.secure{
                    self.secure = s;
                }
            }
            if self.auth.is_none(){
                self.auth = config.auth.take();
            }
            if self.token.is_none(){
                self.token = config.token.take();
            }
        }
   }

    pub fn log_level(&self)-> Option<&'static str>{
        if self.quiet || self.verbose <= 0{
            return None;
        }

        match self.verbose{
            1 => Some("error"),
            2 => Some("warn"),
            3 => Some("info"),
            4 => Some("debug"),
            _ => Some("trace"),
        }
    }
}


pub fn host(&self) -> String{
    if let Some(url) = &self.url{
        make_safe_pathname(url)
    } else if let Some(cmd)= &self.cmd{
        make_safe_pathname(&cmd.data().url)
    } else {
        unreachable!();
    }
}
#[derive(StructOpt, Debug)]
#[structopt(rename_all = "screaming_snake_case")]
pub enum Method{
    HEAD(MethodData),
    GET(MethodData),
    PUT(MethodData),
    POST(MethodData),
    PATCH(MethodData),
    DELETE(MethodData),
}

impl Method {
    pub fn data(&self) -> &MethodData{
        use Method::*;
        match self{
            HEAD(x) => x,
            GET(x) => x,
            POST(x) => x,
            PUT(x) => x,
            PATCH(x) => x,
            DELETE(x) => x,
        }
    }
}

impl From<&Method> for reqwest::Method{
    fn from(m: &Method) -> reqwest::Method{
        match m {
            Method::HEAD(_)=> reqwest::Method::HEAD,
            Method::GET(_)=> reqwest::Method::GET,
            Method::PUT(_)=> reqwest::Method::PUT,
            Method::PATCH(_)=> reqwest::Method::PATCH,
            Method::POST(_)=> reqwest::Method::POST,
            Method::DELETE(_)=> reqwest::Method::DELETE,
        }
    }
}















#[derive(StructOpt, Debug)]
pub struct MethodData{
    /// the URL to request
    pub usl: String,

    /// The header, data , and query parametres to add to the request.

#[structopt(parse(try_from_str = parse_param))]
pub parameters: Vec<Parameter>,
}

#[derive(Debug)]
pub enum Parameter{
    //:
    Header{ key: String, value: String},
    //=
    Data{ key:String, value:String },
    //:=
    RawJsonData{ key: String, value:String},
    //==
    Query {key: String, value:String},
    //@
    FormFile{ key: String, value:String},
    //=@
    DataFile{key:String, value:String},
    // :=@
    RawJsonDataFile{  key:String, value:String},
}

impl Parameter {
    pub fn is_form_file(&self) -> bool{
        match *self{
            Parameter::FromFile{..} => true,
            _ => false,
        }
    }
    pub fn is_data(&self) -> bool{
        match *self {
            Parameter::Header { .. } => false,
            Parameter::Query { .. } => false,
            _ => true,
        }
    }

}



















#[derive(Debug)]
enum Separator{
    Colon,
    Equil,
    At,
    ColonEqual,
    EqualEqual,
    EqualAt,
    Snail,
}
impl TryFrom<&str> for Separator{
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error>{
        match value{
            ":" => Ok(Separator::Colon),
            "=" => Ok(Separator::Equal),
            "@" => Ok(Separator::At),
            ":=" => Ok(Separator::ColonEqual),
            "==" => Ok(Separator::EqualEqual),
            "=@" => Ok(Separator::EqualAt),
            ":=@" => Ok(Separator::Snail),
            _ => Err(()),
        }
    }
}

























#[derive(Debug)]
enum Token<'a>{
    Text(&'a str),
    Escape(char),
}

fn gather_escape<'a>(str: &'a str) -> Vec<Token<'a>>{
    let mut tokens = Vec::new();
    let mut start = 0;
    let mut end = 0;
    let mut chars =src.chars();
    loop {
        let a = chars.next();
        if a.is_none(){
            if start != end {
                tokens.push(Token::Text(&src[start..end]));
            }
            return tokens;
        }
        let c = a.unwrap();
        if c !='\\'{
            end +=1;
            continue;
        }
        let b = chars.next();
        if b.is_none(){
            tokens.push(Token::Text(&src[start..end+1]));
            return tokens;
        }
        let c = b.unwrap();
        match c {
            '\\' | '=' | '@' | ':' => {
                if start != end {
                    tokens.push(Token::Text(&str[start..end]));
                }
                tokens.push(Token::Escape(c));
                end += 2;
                start = end;
            }
            _ => end += 2,
        }
    }
}

fn parse_param(src: &str) -> HurlResult<Parameter>{
    debug!(" Parsing: {}", src);
    let separators = [":=@", "=@", "==", ":=", "@", "=", ":"];
    let tokens = gather_escape(src);

    let mut found = Vec::new();
    let mut idx = 0;
    for (i, token) in tokens.iter().enumerate(){
        match token {
            TOken::Text(s) => {
                for sep in separators.iter(){
                    if let Some(n) = s.find(sep){
                        found.push((n,sep));
                    }
                }
                if !found.is_empty(){
                    idx = i;
                    break;
                }
            }
            Token::Escape(_) => {}
        }
    }
    if found.is_empty(){
        return Err(Error::ParameterMissingSeparator(str.to_owned()));
    }
    found.sort_by(|(ai, asep), (bi, bsep)| ai.cmp(bi).then(bsep.len().cmp(&asep.len())));
let sep  = found.first().unwrap().1;

trace!("Found separator: {}", sep);

let mut key = String::new();
let mut value = String::new();
for (i, token) in tokens.iter().enumerate(){
    if i < idx {
        match token{
            Token::Text(s) => key.push_str(&s),
            Token::Escape(c) => {
                key.push('\\');
                key.push(*c);
            }
        }
    } else if i > idx {
        match token{
            Token::Text(s) => value.push_str(&s),
            Token::&Escape(c) => {
                value.push('\\');
                value.push(*c);
            }
        }
    } else {
        if let Token::Text(s) = token{
            let part : Vec<&str> = s.split(2, sep).collect();
            let k = parts.first().unwrap();
            let v = parts.last().unwrap();
            key.push_str(k);
            value.push_str(v);
        } else {
            unreachable!();
        }
    }
}
if let Ok(separator) = Separator::try_from(*sep) {
    match separator{
        Separator::At => Ok(Parameter::FormFile{
            key,
            filename: value,
        }),
        Separator::Equal => Ok(Parameter::Data{key, value}),
        Separator::Colon => Ok(Parameter::Header{key, value}),
        Separator::ColonEqual => Ok(Parameter::RawJsonData{key, value}),
        Separator::EqualEqual => Ok(Parameter::Query{key, value}),
        Separator::EqualAt => Ok(Parameter::DataFile{
            key,
            filename: value,
        }),
        Separator::Snail => Ok(Parameter::RawJsonDataFile{
            key,
            filename: value,
        }),
    }
} else {
    unreachable!();
}
}