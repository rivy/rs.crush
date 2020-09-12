use signature::signature;
use crate::lang::files::Files;
use crate::lang::execution_context::CommandContext;
use crate::lang::errors::CrushResult;
use std::path::{Path, PathBuf};
use crate::lang::stream::OutputStream;
use crate::lang::data::table::Row;
use crate::lang::value::Value;
use crate::lang::value::ValueType;
use crate::lang::data::table::ColumnType;
use lazy_static::lazy_static;
use crate::lang::command::OutputType::Known;
use crate::util::directory_lister::{DirectoryLister, directory_lister};
use  std::os::unix::fs::MetadataExt;

lazy_static! {
    static ref OUTPUT_TYPE: Vec<ColumnType> = vec![
        ColumnType::new("size", ValueType::Integer),
        ColumnType::new("blocks", ValueType::Integer),
        ColumnType::new("file", ValueType::File),
    ];
}

#[signature(
du,
can_block = true,
output = Known(ValueType::TableStream(OUTPUT_TYPE.clone())),
short = "Calculate the recursive directory size.",
)]
pub struct Du {
    #[unnamed()]
    #[description("the files to calculate the recursive size of.")]
    directory: Files,
    #[description("do not show directory sizes for subdirectories.")]
    #[default(false)]
    silent: bool,
}

fn size(
    path: &Path,
    silent: bool,
    is_directory: bool,
    output: &OutputStream,
    lister: &impl DirectoryLister,
) -> CrushResult<(u64, u64)> {
    let mut sz = path.metadata().map(|m| m.size()).unwrap_or(0);
    let mut bl = path.metadata().map(|m| m.blocks()).unwrap_or(0);
    Ok(if is_directory {
        for child in lister.list(path)? {
            let (child_sz, child_bl) = size(&child.full_path, silent, child.is_directory, output, lister)?;
            if !silent && child.is_directory {
                output.send(Row::new(
                    vec![
                        Value::Integer(child_sz as i128),
                        Value::Integer(child_bl as i128),
                        Value::File(child.full_path),
                    ]
                ))?;
            }
            sz += child_sz;
            bl += child_bl;
        }
        (sz, bl)
    } else {
        (sz, bl)
    })
}

fn du(context: CommandContext) -> CrushResult<()> {
    let cfg: Du = Du::parse(context.arguments, &context.printer)?;
    let mut output = context.output.initialize(OUTPUT_TYPE.clone())?;
    let dirs = if cfg.directory.had_entries() {
        cfg.directory.into_vec()
    } else {
        vec![PathBuf::from(".")]
    };
    for file in dirs {
        let (sz, bl) = size(&file, cfg.silent, file.is_dir(), &output, &directory_lister())?;

        output.send(Row::new(
            vec![
                Value::Integer(sz as i128),
                Value::Integer(bl as i128),
                Value::File(file),
            ]
        ))?
    }

    Ok(())
}
