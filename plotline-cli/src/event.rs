use crate::{Error, Result};
use clap::{Args, Subcommand};
use plotline::{
    event::{
        application::{EventApplication, EventRepository},
        Event,
    },
    id::Id,
};

#[derive(Args)]
struct EventSaveArgs {
    /// The name of the event.
    #[arg(long, short)]
    name: Option<String>,
    /// The period during which the event takes place.
    #[arg(long, short, num_args(1..=2))]
    interval: Option<Vec<String>>,
}

#[derive(Subcommand)]
#[clap(subcommand_negates_reqs = true, subcommand_precedence_over_arg = true)]
enum EventSubCommand {
    /// Save an event.
    Save(EventSaveArgs),
}

#[derive(Args)]
#[command(arg_required_else_help = true)]
pub struct EventCommand {
    /// The id of the event.
    event: Option<String>,
    /// The action to perform.
    #[command(subcommand)]
    command: Option<EventSubCommand>,
}

pub struct EventCli<EventRepo> {
    pub event_app: EventApplication<EventRepo>,
}

impl<EventRepo> EventCli<EventRepo>
where
    EventRepo: 'static + EventRepository + Sync + Send,
    EventRepo::Intv: TryFrom<Vec<String>> + Sync + Send,
    <EventRepo::Intv as TryFrom<Vec<String>>>::Error: Into<Error>,
{
    /// Given a [EventCommand], executes the corresponding logic.
    pub fn execute(&self, event_cmd: EventCommand) -> Result {
        let event_id = event_cmd.event.map(TryInto::try_into).transpose()?;
        if let Some(command) = event_cmd.command {
            return self.execute_subcommand(command, event_id);
        }

        Ok(())
    }

    fn execute_subcommand(
        &self,
        subcommand: EventSubCommand,
        event_id: Option<Id<Event<EventRepo::Intv>>>,
    ) -> Result {
        match subcommand {
            EventSubCommand::Save(args) => {
                let event_id = event_id.unwrap_or_default();
                self.event_app
                    .save_event(event_id)
                    .with_name(args.name.map(TryInto::try_into).transpose()?)
                    .with_interval(
                        args.interval
                            .map(TryInto::try_into)
                            .transpose()
                            .map_err(Into::into)?,
                    )
                    .execute()?;

                println!("{}", event_id);
            }
        }

        Ok(())
    }
}
