use chrono::Utc;
use clap::Parser;
use poise::serenity_prelude as serenity;
use sqlx::sqlite::{Sqlite, SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{migrate::Migrator, query, Pool};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Access token for your bot. Required to run.
    #[arg(short, long, required(true))]
    token: String,
    /// Path to where the database should be stored, relative to the current path. If it doesn't exist it will
    /// be created. By default it will use './database.sqlite'.
    #[arg(short, long)]
    database: Option<String>,
    /// Optional guild id to connect to. Useful for testing, speeds up registering of commands.
    #[arg(short, long)]
    guild: Option<u64>,
    /// Print a premade systemd unit with your options.
    #[arg(long = "make-systemd-unit")]
    unit: bool,
}

struct Data {
    database: Pool<Sqlite>,
    guild: Option<serenity::GuildId>,
}

impl Data {
    async fn migrate(&self) {
        static MIGRATOR: Migrator = sqlx::migrate!();
        MIGRATOR.run(&self.database).await.unwrap();
    }
    async fn from(guild: Option<u64>, db: Option<String>) -> Self {
        let path = if let Some(db) = db {
            db
        } else {
            "database.sqlite".to_string()
        };

        let id = if let Some(gid) = guild {
            Some(serenity::GuildId(gid))
        } else {
            None
        };
        let database = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true),
            )
            .await
            .expect("Couldn't connect to database"); // TODO handle database creation & connection
                                                     // errors better
        let out = Self {
            database,
            guild: id,
        };
        out.migrate().await;
        out
    }
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

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

#[poise::command(slash_command, subcommands("add", "random"))]
async fn quote(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
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

/// Bring up a random quote by a particular user
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
    let cli = Cli::parse();
    if cli.unit {
        println!("{}", systemd_unit(&cli.token, &cli.database, cli.guild));
        return;
    }
    let data = Data::from(cli.guild, cli.database).await;
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![quote()],
            ..Default::default()
        })
        .token(cli.token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                if let Some(guild) = data.guild {
                    poise::builtins::register_in_guild(ctx, &framework.options().commands, guild)
                        .await
                        .expect("Invalid Guild Id");
                    println!("Registering guild commands!");
                };
                println!("Registering global commands");
                Ok(data)
            })
        });

    framework.run().await.unwrap();
}

fn systemd_unit(tok: &str, db: &Option<String>, guild: Option<u64>) -> String {
    let mut unit = format!(
        r"[Unit]
Description=Discord quote bot
[Service]
ExecStart=/usr/bin/discord_quote_bot --token {}",
        tok
    );
    if let Some(database) = db {
        unit.push_str(&format!("--database {}", database));
    }
    if let Some(guild) = guild {
        unit.push_str(&format!("--guild {}", guild.to_string()));
    }
    unit.push_str("\n[Install]\nWantedBy=multi-user.target");
    unit
}
