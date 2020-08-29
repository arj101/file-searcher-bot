use serenity::async_trait;
use serenity::client::{ Context, EventHandler};
use serenity::client;
use serenity::model::channel::Message;
use serenity::framework::standard::{
    StandardFramework,
    CommandResult,
    macros::{
        command,
        group
    }
};
use hyper::body::HttpBody as _;
use hyper_tls::HttpsConnector;
use tokio::io::{stdout, AsyncWriteExt as _};
use hyper::Client;
use std::env;
use tokio;
use std::{thread, time};

#[group]
#[commands(ping, search)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("_")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = client::Client::new(token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let bot_msg =  msg.channel_id.say(&ctx.http, "Pong").await?;
    
    msg.react(&ctx, '🏓').await?;
    bot_msg.react(&ctx, '🏓').await?;

    Ok(())
}

#[command]
async fn search(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Note: this command is a work in progress⚠️ ").await?;
    
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    
    for attachment in &msg.attachments {
        
        let uri = attachment.url.parse()?;
        let mut resp = client.get(uri).await?;
        
        let mut text_content = String::new();

        while let Some(chunk) = resp.body_mut().data().await {
            text_content.push_str(std::str::from_utf8(&chunk.unwrap_or_default()).unwrap());
        }
        text_content = text_content
            .replace("*", "\\*")
            .replace("`", "\\`")
            .replace("|", "\\|")
            .replace("_", "\\_")
            .replace("~", "\\~");

        let args:Vec<&str> = msg.content.split(' ').filter(|word| word.len() > 0).collect();

        msg.channel_id.say(&ctx.http, format!("\
```CSS
Searching for matches in '{}'...
```", attachment.filename)).await?;

        let search_results = search_str(args, &text_content)?;

        let search_word_len = search_results.search_word.len();

        let mut search_result_string = String::new();

        for line_search_result in search_results.lines {
            let line_number = line_search_result.line_number;
            let mut result_string =  line_search_result.line.to_owned();

            let mut close_match = false;
            
            let mut index_offset:usize = 0;
            for ( i, search_result_in_line ) in  line_search_result.results.iter().enumerate() {          
                if !close_match {
                    &result_string.insert_str(search_result_in_line.index + index_offset, "__");
                    index_offset += 2;
                }
                if i + 1 < line_search_result.results.len() {
                    if line_search_result.results[i+1].index == search_result_in_line.index+search_word_len {
                        close_match = true;
                    } else {
                        close_match = false;
                    }
                } else {
                    close_match = false;
                }

                if !close_match {
                    &result_string.insert_str(search_result_in_line.index + search_word_len + index_offset, "__");
                    index_offset += 2; 
                }
            }
            
            result_string = format!("`[line: {}]` {}\n\n", line_number, result_string);

            &search_result_string.push_str(&result_string);
        }

        if search_result_string.len() < 2000 {
            msg.channel_id.say(&ctx.http, search_result_string).await?;
        } else {
            let mut current_index:usize = 0;
            while current_index < search_result_string.len() {
                if current_index + 2000 < search_result_string.len() {
                    msg.channel_id.say(&ctx.http, &search_result_string[current_index..current_index+2000]).await?;
                } else {
                    msg.channel_id.say(&ctx.http, &search_result_string[current_index..search_result_string.len()]).await?;
                }
                current_index+=2000;                
            }
        }

    }

    Ok(())
}


struct SearchResults<'a, 'b> {
    search_word: &'a str,
    lines: Vec<LineSearchResult<'b>>
}

struct LineSearchResult<'a> {
    line: &'a str,
    line_number: usize,
    results: Vec<SearchResultInLine>
}

struct SearchResultInLine {
    index: usize
}

fn search_str<'a, 'b>(args: Vec<&'a str>,text: &'b str) -> Result<SearchResults<'a, 'b>, &'static str> {

    if args.len() < 2 {
        return Err("Arguments too short");
    }

    let mut case_sensitive = true;
    let mut search_word = args[1];

    
    if args.len() == 3 {
        search_word = args[2];
        if args[1] == "false" {
            case_sensitive = false;
        }
    }


    let mut search_results = SearchResults {
        search_word,
        lines: Vec::new()
    };

    let mut line_number = 1;

    for line in text.lines() {
        if case_sensitive {
            if line.contains(search_word) {
                let mut line_search_result = LineSearchResult {
                    line,
                    line_number,
                    results: Vec::new()
                };

                let matches: Vec<(usize, &str)> = line.match_indices(search_word).collect();

                for matched in matches {
                    line_search_result.results.push(SearchResultInLine {
                        index: matched.0
                    });
                }
                search_results.lines.push(line_search_result);
            }
        } else {
            let line_lowercase = line.to_lowercase();
            if line_lowercase.contains(&search_word.to_lowercase()) {
                let mut line_search_result = LineSearchResult {
                    line,
                    line_number,
                    results: Vec::new()
                };

                let matches: Vec<(usize, &str)> = line_lowercase.match_indices(&search_word.to_lowercase()).collect();

                for matched in matches {
                    line_search_result.results.push(SearchResultInLine {
                        index: matched.0
                    });
                }
                search_results.lines.push(line_search_result);
            }
        }
        line_number += 1;
    }

    Ok(search_results)
}