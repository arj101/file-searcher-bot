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
use std::env;
use tokio;

mod search_command;

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
    let _ = search_command::search(ctx, msg).await;
   
    Ok(())
}
