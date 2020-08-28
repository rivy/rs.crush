use crate::lang::argument::ArgumentHandler;
use crate::lang::errors::{CrushResult, to_crush_error, argument_error, mandate};
use crate::lang::execution_context::CommandContext;
use crate::lang::data::scope::Scope;
use crate::lang::data::r#struct::Struct;
use crate::lang::value::Value;
use signature::signature;
use systemd::journal::{JournalFiles, Journal, JournalSeek};
use crate::lang::data::table::Row;
use lazy_static::lazy_static;
use crate::lang::{data::table::ColumnType, value::ValueType};
use crate::lang::ordered_string_map::OrderedStringMap;
use chrono::{DateTime, Local};
use std::convert::TryFrom;
use crate::lang::command::OutputType::Known;

lazy_static! {
    static ref JOURNAL_OUTPUT_TYPE: Vec<ColumnType> = vec![
        ColumnType::new("time", ValueType::Time),
        ColumnType::new("data", ValueType::Struct),
    ];
}

#[signature(
journal,
can_block = true,
output = Known(ValueType::TableStream(JOURNAL_OUTPUT_TYPE.clone())),
short = "Show the systemd journal"
)]
struct JournalSignature {
    #[description("wait indefinitely for more data once the end of the journal is reached.")]
    #[default(false)]
    follow: bool,
    #[description("ignore system logs.")]
    #[default(false)]
    skip_system_files: bool,
    #[description("ignore this users logs.")]
    #[default(false)]
    skip_user_files: bool,
    #[description("start reading at the end of the log.")]
    #[default(false)]
    runtime_only: bool,
    #[description("only show logs generated by localhost.")]
    #[default(false)]
    local_only: bool,
    #[description("seek to the specified timestamp.")]
    seek: Option<Value>,
    #[description("filter to only show journal entries with the specified key-value pairs.")]
    #[named()]
    filters: OrderedStringMap<String>,
}

fn parse_files(cfg: &JournalSignature) -> CrushResult<JournalFiles> {
    match (!cfg.skip_system_files, !cfg.skip_user_files) {
        (true, true) => Ok(JournalFiles::All),
        (true, false) => Ok(JournalFiles::System),
        (false, true) => Ok(JournalFiles::CurrentUser),
        (false, false) => argument_error("No files specified"),
    }
}

fn usec_since_epoch(tm: DateTime<Local>) -> CrushResult<u64> {
    let epoch = DateTime::from(std::time::UNIX_EPOCH);
    let duration = tm - epoch;
    to_crush_error(u64::try_from(mandate(duration.num_microseconds(), "Time overflow")?))
}

fn journal(context: CommandContext) -> CrushResult<()> {
    let cfg: JournalSignature = JournalSignature::parse(context.arguments, &context.printer)?;
    let mut journal = to_crush_error(Journal::open(parse_files(&cfg)?, cfg.runtime_only, cfg.local_only))?;

    match cfg.seek {
        Some(Value::Time(tm)) => {
            to_crush_error(journal.seek(JournalSeek::ClockRealtime {
                usec: usec_since_epoch(tm)?
            }))?;
        }
        Some(v) => {
            return argument_error(format!("Don't know how to seek to {}", v.value_type()));
        }
        None => {}
    }

    for (key, value) in &cfg.filters {
        to_crush_error(journal.match_add(key, value.as_bytes()))?;
    }

    let output = context.output.initialize(JOURNAL_OUTPUT_TYPE.clone())?;

    loop {
        match to_crush_error(if cfg.follow { journal.await_next_record(None) } else { journal.next_record() })? {
            None => if !cfg.follow {
                break
            },
            Some(row) => {
                let data = Value::Struct(Struct::new(
                    row.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect(),
                    None));
                output.send(Row::new(vec![
                    Value::Time(DateTime::from(journal.timestamp()?)),
                    data,
                ]))?;
            }
        }
    }
    Ok(())
}

pub fn declare(root: &Scope) -> CrushResult<()> {
    root.create_namespace(
        "systemd",
        Box::new(move |systemd| {
            JournalSignature::declare(systemd)?;
            Ok(())
        }),
    )?;
    Ok(())
}
