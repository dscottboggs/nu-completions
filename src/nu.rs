use std::{
    fmt::Display,
    fs::File,
    io::BufRead,
    io::{self, BufReader, Seek, Write},
    iter::repeat,
    path::{Path, PathBuf},
    sync::{LazyLock, RwLock},
};

use anyhow::Result;
use joinery::JoinableIterator;
use log::{as_debug, debug, error, info, trace, warn};

use crate::{completions, config::Config, dir_walker::walk_dir};

pub(crate) fn processing_failed(path: impl AsRef<Path>, err: anyhow::Error) -> Result<!> {
    error!(
        "failed to process completions at {:?}: {err:?}",
        path.as_ref()
    );
    Err(err)
}

pub(crate) fn process_file_or_dir(path: PathBuf) -> Result<()> {
    process_file_or_dir_given_output_dir(path, Config::output_dir())
}

pub(crate) fn process_file_or_dir_given_output_dir(
    path: PathBuf,
    output_dir: impl AsRef<Path>,
) -> Result<()> {
    info!(file = path.to_string_lossy(); "processing file or directory");
    let output_dir = output_dir.as_ref();
    walk_dir(&path, (), |path, _| {
        process_file_given_output_dir(&path, output_dir).map(|_| ())
    })
}

pub(crate) fn process_file_given_output_dir(path: &Path, output_dir: &Path) -> Result<PathBuf> {
    info!(file = path.to_string_lossy(); "processing file");
    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{path:#?} is not a file"),
        )
        .into());
    }
    let errmsg = format!("reading file {path:#?}");
    let file = BufReader::new(File::open(&path)?);
    trace!(file = as_debug!(path); "opened file for processing");
    let completions =
        completions::Completions::parse(file.lines().map(|line| line.expect(&errmsg)))?;
    trace!("successfully parsed completions for {path:?}");
    let location = output_dir.join(
        path.with_extension("nu")
            .file_name()
            .expect("directory already checked for"),
    );
    debug!("writing completions parsed from {path:?} into {location:?}");
    Completions::at(&location)?.output(completions)?;
    Ok(location)
}

#[derive(Debug)]
pub(crate) struct Completions<IO: Seek + Write> {
    io: IO,
    indent: usize,
}

impl<IO: Seek + Write> Completions<IO> {
    fn new(io: IO) -> Self {
        Self { io, indent: 0 }
    }
}

impl Completions<File> {
    fn at(location: impl AsRef<Path>) -> Result<Self> {
        let file = File::create(location)?;
        Ok(Self::new(file))
    }
}

struct Synonym<'a> {
    synonym_of: String,
    name: String,
    description: Option<&'a str>,
}

impl<IO: Seek + Write> Completions<IO> {
    fn write(&mut self, it: impl Display + log::kv::ToValue) -> Result<&mut Self> {
        trace!(content=it, indent=self.indent; "writing data");
        write!(self.io, "{}{it}", self.indent_str())?;
        Ok(self)
    }

    fn eol(&mut self) -> Result<&mut Self> {
        write!(self.io, "\n")?;
        Ok(self)
    }

    pub(crate) fn output(&mut self, completions: completions::Completions) -> Result<()> {
        let mut command_count: usize = 0;
        for (cmd, opts) in completions.read().expect("rwlock read access").iter() {
            let cmd = if let Err(which::Error::CannotCanonicalize) = which::which(cmd) {
                cmd.replace('-', " ")
            } else {
                cmd.to_string()
            };
            let cmd = cmd.as_str();
            self.write(format!(r#"export extern "{cmd}" ["#))?.eol()?;
            self.indent += 1;
            let mut rules: usize = 0;
            let mut synonyms = vec![];
            for option in opts {
                let (mut def, mut arg) = (String::new(), String::new());
                match &option.old_option.as_slice() {
                    [] => {
                        if option.long.is_empty() {
                            match &option.short.as_slice() {
                                &[] => (),
                                [opt] => {
                                    def.push('-');
                                    def.push_str(opt);
                                }
                                options => {
                                    def.push('-');
                                    def.push_str(&options[0]);
                                    for opt in &options[1..] {
                                        synonyms.push(Synonym {
                                            name: format!("-{opt}"),
                                            synonym_of: format!("--{}", &options[0]),
                                            description: option
                                                .description
                                                .as_ref()
                                                .map(|it| it.as_str()),
                                        });
                                    }
                                }
                            }
                        } else {
                            let opt = option.long[0].as_ref();
                            def.push_str("--");
                            def.push_str(opt);
                            if !option.short.is_empty() {
                                def.push_str("(-");
                                def.push_str(&option.short[0]);
                                def.push(')');
                                for opt in &option.short[1..] {
                                    synonyms.push(Synonym {
                                        name: format!("-{opt}"),
                                        synonym_of: format!("--{}", &option.long[0]),
                                        description: option
                                            .description
                                            .as_ref()
                                            .map(|it| it.as_str()),
                                    });
                                }
                            }

                            for opt in &option.long[1..] {
                                synonyms.push(Synonym {
                                    name: format!("--{opt}"),
                                    synonym_of: format!("--{}", &option.long[0]),
                                    description: option.description.as_ref().map(|it| it.as_str()),
                                });
                            }
                        }
                    }

                    [opt] => {
                        def.push('-');
                        def.push_str(opt);
                    }
                    options => {
                        def.push('-');
                        def.push_str(&options[0]);
                        for opt in &options[1..] {
                            synonyms.push(Synonym {
                                name: format!("-{opt}"),
                                synonym_of: format!("-{}", &options[0]),
                                description: option.description.as_ref().map(|it| it.as_str()),
                            });
                        }
                    }
                }
                if def.is_empty() {
                    warn!(option = as_debug!(option), cmd = cmd; "no option or arg");
                    continue;
                }
                if option.argument.is_some() {
                    arg.push_str(": string");
                }
                let (def, arg) = (def.as_str(), arg.as_str());
                debug!(def=def, arg=arg, cmd=cmd; "writing command to file");
                self.write(def.to_owned() + arg)?;
                if let Some(description) = &option.description {
                    let description = description.as_str();
                    debug!(def=def, description=description; "writing description");
                    self.write("  # {".to_owned() + description)?.eol()?;
                }
                rules += 1;
            }
            for Synonym {
                synonym_of,
                name,
                description,
            } in &synonyms
            {
                debug!(cmd = cmd, opt = name; "writing synonym");
                let desc = if let Some(desc) = description {
                    format!("{desc} (synonym of {synonym_of})")
                } else {
                    format!("synonym of {synonym_of}")
                };
                self.write(format!("{name} #  {desc}"))?.eol()?;
                rules += 1;
            }
            debug!(rule_count=rules, cmd=cmd; "wrote rules");
            self.indent -= 1;
            self.write("]\n")?;
            command_count += 1;
        }
        debug!(command_count=command_count; "wrote commands");
        Ok(())
    }

    fn indent_str(&self) -> String {
        let mut cache = INDENT_CACHE.write().expect("poisoned mutex");
        if let Some(i) = cache.get(self.indent) {
            trace!(level=self.indent, str=i.as_str(); "got cached indent");
            i.clone()
        } else {
            let max_cached = cache.len();
            trace!(max_cached=max_cached, target_indent=self.indent; "indent not yet cached, filling");
            for i in max_cached..=self.indent {
                let text: String = repeat(' ').take(i * 4).collect();
                trace!(level=i, str=text.as_str(); "caching indent level");
                cache.push(text);
            }
            // Size is ensured above
            unsafe { cache.get_unchecked(self.indent).clone() }
        }
    }
}

static INDENT_CACHE: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(vec![]));
