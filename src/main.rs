use serenity::{async_trait, client::{
        Client, 
        Context, 
        EventHandler
    }, collector::ReactionCollectorBuilder, framework::standard::{
        StandardFramework,
        CommandResult,
        Args,
        macros::{
            command,
            group
        }
    }, model::{Permissions, channel::{Message, PermissionOverwrite, PermissionOverwriteType, ReactionType}, gateway::Ready, id::RoleId}};
use futures::stream::StreamExt;
use std::env;

#[group]
#[commands(game)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Connected as {}", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("?")) // set the bot's prefix to "?"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("token");
    let mut client = Client::builder(token)
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
async fn game(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    // the channel name for the new channel
    let channel_name = args.message();
    let checkmark = "✅";

    //get the guild, ensure the command was not used in a DM
    let guild = match msg.guild_id {
        Some(id) => id,
        None => {
            msg.channel_id.say(ctx, "This command is only useable in servers").await?;
            return Ok(());
        },
    };
    //the permissions assigned to @everyone
    let permissions = Some(PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::READ_MESSAGES,
        kind: PermissionOverwriteType::Role(RoleId(guild.0)),
    });
    //the newly created channel, using channel_name and permissions respectively
    let channel = guild.create_channel(ctx, |c| c.name(channel_name).permissions(permissions)).await?;
    
    
    let first = async {
        //the reply made by the bot to get reactions, the reaction it will add, and adding the reaction
        let reply = msg.channel_id.say(ctx, format!("The channel {} was created! React to this post if interested.", channel)).await;
        let reply = match reply {
            Ok(reply) => reply,
            Err(error) => panic!("Error occured upon reply: {:?}", error),
        };

        let _ = reply.react(&ctx.http, ReactionType::Unicode(checkmark.to_string())).await;

        //ReactionCollectorBuilder watching for opt-in reactions
        let reactions = ReactionCollectorBuilder::new(&ctx)
        .message_id(reply.id)
        .collect_limit(5u32)
        .await;

        //for each reaction added allow the user to read messages in the created event channel
        let _react = reactions.for_each(|reaction| {
            let channel = channel.id;
            async move {
                println!("{:?}", reaction);
                let _ = channel.create_permission(ctx, &PermissionOverwrite {
                    kind: PermissionOverwriteType::Member(reaction.as_inner_ref().user_id.unwrap()),
                    allow: Permissions::READ_MESSAGES,
                    deny: Permissions::empty(),
                }).await;
            }
        }).await;
    };
    
    
    let second = async {
        //first message added to new channel, will be used to delete channel, reacts to own message with checkmark
        let _message = channel.id.say(ctx, format!("Welcome to {}, react to close this channel.", channel_name)).await;
        let _message = match _message {
            Ok(message) => message,
            Err(error) => panic!("Error occured upon reply: {:?}", error),
        };
        let _ = _message.react(&ctx.http, ReactionType::Unicode(checkmark.to_string())).await;

        //could be made better, is used to watch for reaction, and delete channel on reaction        
        if let Some(_close) = &_message.await_reaction(&ctx).await {
            let _ = channel.delete(&ctx.http).await;
        };
    };

    tokio::join!(first, second);

    Ok(())
}