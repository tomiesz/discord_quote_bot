use chrono::Utc;
use poise::serenity_prelude as serenity;
use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{query, Pool};

// User data, which is stored and accessible in all command invocations
struct Data {
    database: Pool<Sqlite>,
}

impl Data {
    async fn new() -> Data {
        let database = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename("database.sqlite")
                    .create_if_missing(true),
            )
            .await
            .expect("Couldn't connect to database");
        Self { database }
    }
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, subcommands("add", "random"))]
async fn quote(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[derive(Debug)]
enum DatabaseError {
    MalformedEntry,
}

impl std::error::Error for DatabaseError {}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            DatabaseError::MalformedEntry => {
                f.write_str("An entry was malformed, and didn't contain necessary data")
            }
        }
    }
}

#[poise::command(slash_command)]
async fn add(
    ctx: Context<'_>,
    #[description = "Selected user"] user: serenity::User,
    #[description = "Quote you want to add"] quote: String,
) -> Result<(), Error> {
    let date = Utc::now().date_naive();
    let user_id = user.id.as_u64().to_string();
    query!(
        "INSERT INTO quotes (user_id, quote_date, quote) VALUES (?,?,?)",
        user_id,
        date,
        quote,
    )
    .execute(&ctx.data().database)
    .await?;
    let response = format!("Quote: {}, by {} added!", quote, user.name);
    ctx.say(response).await?;
    Ok(())
}

#[poise::command(slash_command)]
async fn random(
    ctx: Context<'_>,
    #[description = "Selected user"] user: serenity::User,
) -> Result<(), Error> {
    let user_id = user.id.as_u64().to_string();
    let entry = query!(
        "SELECT * FROM quotes WHERE user_id = ? ORDER BY RANDOM() LIMIT 1;",
        user_id
    )
    .fetch_one(&ctx.data().database)
    .await;
    if let Ok(body) = entry {
        let response = serenity::MessageBuilder::new()
            .push_bold_safe(body.quote.ok_or(DatabaseError::MalformedEntry)?)
            .push("\n")
            .mention(&user)
            .push(" on ")
            .push(body.quote_date.ok_or(DatabaseError::MalformedEntry)?)
            .build();
        ctx.say(response).await?;
    } else {
        let response = format!("No quotes found for user: {} ", user.name);
        ctx.send(|f| f.content(response).ephemeral(true)).await?;
    };
    Ok(())
}

#[tokio::main]
async fn main() {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![quote()],
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data::new().await)
            })
        });

    framework.run().await.unwrap();
}
