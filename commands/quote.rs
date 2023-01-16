use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::CommandDataOption;

pub fn run(_options: &[CommandDataOption]) -> String {
    "TEST QUOTE".to_string()
}

pub fn register(command &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("quote").description("Show me a quote")
}
