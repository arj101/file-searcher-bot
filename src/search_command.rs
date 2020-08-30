use serenity::client::Context;
use serenity::model::channel::Message;
use hyper::body::HttpBody;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use std::collections::VecDeque;
use hyper::Client;
use std::str;

impl std::error::Error for CommandError {}//just a hack to get it working :P

#[derive(Debug)]
struct CommandError(String);

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.0)
    }
}

pub async fn search(ctx: &Context, msg: &Message) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let search_args = match SearchArgs::new(&msg.content) {
        Ok(search_args) => search_args,
        Err(err) => {
            msg.channel_id.say(&ctx.http,
            format!("\
Usage: `search [search word]`.

```nim
{}
```
", err)).await?;

            return Err(Box::new(CommandError(String::from("Wrong number of args provided"))));
        }
    };

    if msg.attachments.len() > 1 {
        const MSG: &str = "\
```nim
Only the first file will be searched.
Everything else will be ignored ;)
```";
        msg.channel_id.say(&ctx.http, MSG).await?;
    } else if msg.attachments.len() < 1 {
const MSG: &str = "\
```nim
Searching in 0 files is not supported yet 😉
```";
        msg.channel_id.say(&ctx.http, MSG).await?;
        return Ok(());
    }

    let attachment_url = &msg.attachments[0].url;

    let text_content = match download_text_file(&client, &attachment_url).await {
        Ok(text_content) => cancel_discord_markdown(text_content),
        Err(err) => {
            msg.channel_id.say(&ctx.http, format!("\
```nim
{}
```", &err)).await?;
            println!("{}", &err);
            return Err(Box::new(CommandError(format!("Error in function 'download_text_file': {}", err))));
        }
    };

    let search_result = match search_str(&search_args, &text_content) {
        Some(search_result) => search_result,
        None => {
            msg.channel_id.say(&ctx.http, "No matches found.").await?;
            return Ok(());
        }
    };

    let formatted_search_result = format_search_result(search_result);

    if formatted_search_result.len() > 2000 {
        const MSG: &str = "\
```nim
Search result too long (over 2000 characters) to send :/
This part is a work in progress 🙃
```";
        msg.channel_id.say(&ctx.http, MSG).await?;

        return Ok(());
    }

    msg.channel_id.say(&ctx.http, formatted_search_result).await?;

    Ok(())
}


async fn download_text_file(client: &Client<HttpsConnector<HttpConnector>>,url: &str) -> 
    Result<String, Box<dyn std::error::Error + Sync + Send>> {

    let mut text_content = String::new();
    
    let uri: hyper::Uri = match url.parse() {
        Ok(uri) => uri,
        Err(err) => return Err(Box::new(CommandError(format!("{}", err))))
    };

    let mut resp = match client.get(uri).await {
        Ok(resp) => resp,
        Err(err) => return Err(Box::new(CommandError(format!("{}",err))))
    };

    while let Some(chunk) = resp.body_mut().data().await {
        let chunk_bytes = match chunk {
            Ok(chunk_bytes) => chunk_bytes,
            Err(err) => return Err(Box::new(CommandError(format!("Failed downloading data from stream: {}", err))))
        };

        let string_chunk = match str::from_utf8(&chunk_bytes) {
            Ok(string_chunk) => string_chunk,
            Err(err) => return Err(Box::new(CommandError(format!("Cannot convert bytes to string: {}", err))))
        };
        text_content.push_str(string_chunk);
    }

    Ok(text_content)
}

struct SearchArgs<'a> {
    case_sensitive: bool,
    search_word: &'a str
}
impl SearchArgs<'_> {
    fn new<'a>(command: &'a str) -> Result<SearchArgs<'_>, String> {
        let mut args: VecDeque<&str> = command.split(" ")
            .filter(|word| word.len() >= 1)
            .collect();
        args.pop_front();

        if args.len() < 1 || args.len() > 1 {
            return Err(format!("Expected one argument, got {}.", args.len()));
        }

        let case_sensitive = true;
        let search_word = args[0];
        
        Ok(SearchArgs {
            case_sensitive,
            search_word
        })
    }
}

struct SearchResult<'a, 'b> {
    search_word: &'a str,
    lines: Vec<LineSearchResult<'b>>
}
struct LineSearchResult<'a> {
    line: &'a str,
    line_number: usize,
    matches: Vec<SearchMatch>
}
struct SearchMatch{
    index: usize
}

fn search_str<'a, 'b>(args: &SearchArgs<'a>, text: &'b str) -> Option<SearchResult<'a, 'b>>{
    let mut search_result = SearchResult {
        search_word: args.search_word,
        lines: Vec::new()
    };
    let mut line_number = 1usize;

    for line in text.lines() {
        if line.contains(args.search_word) {
            let mut line_search_result = LineSearchResult {
                line,
                line_number,
                matches: Vec::new()
            };
            let matches: Vec<(usize, &str)> = line.match_indices(args.search_word).collect();

            for search_match in matches {
                line_search_result.matches.push(
                    SearchMatch {
                        index: search_match.0
                    }
                );
            } 
            search_result.lines.push(line_search_result);
        }
        line_number += 1;
    }

    if search_result.lines.len() > 0 {
        Some(search_result)
    } else {
        None
    }
}

fn cancel_discord_markdown(text: String) -> String {
    text.replace("*", "\\*") //bold / italic text
        .replace("`", "\\`") //multiline and single line code block
        .replace("|", "\\|") //spoiler
        .replace("_", "\\_") //underline
        .replace("~", "\\~") //strikethrough
}

fn format_search_result(search_result: SearchResult) -> String {
    let mut search_result_formatted = String::new();
    let search_word_len = &search_result.search_word.len();
    
    for line_search_result in &search_result.lines {
        let line_number = line_search_result.line_number;
        let mut line_search_result_formatted = line_search_result.line.to_owned();

        let mut close_match = false;
        let mut index_offset = 0usize;
        for ( i, search_match ) in line_search_result.matches.iter().enumerate() {
            if !close_match {
                &line_search_result_formatted.insert_str(search_match.index + index_offset, "__");
                index_offset += 2;
            }
            if i < line_search_result.matches.len() - 1 { //if the next index exists 
                if line_search_result.matches[ i + 1 ].index
                == search_match.index + search_word_len {
                    close_match = true;
                } else {
                    close_match = false;
                }
            } else {
                close_match = false;
            }
            if !close_match {
                &line_search_result_formatted.insert_str(
                    search_match.index + search_word_len + index_offset,
                    "__"
                );
                index_offset += 2;
            }
        }
        line_search_result_formatted = format!("`[line: {}]` {}\n\n", line_number, line_search_result_formatted);
        search_result_formatted.push_str(&line_search_result_formatted);
    }

    search_result_formatted
}
