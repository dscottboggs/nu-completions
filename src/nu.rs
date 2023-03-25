use std::{
    collections::HashSet,
    fmt::Display,
    fs::File,
    io::BufRead,
    io::{self, BufReader, Seek, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{LazyLock, RwLock},
};

use anyhow::Result;
use log::{as_debug, as_serde, debug, error, info, trace, warn};

use crate::{completions, config::Config, dir_walker::walk_dir};

/// Log a failure to process a completion and return the error. This is
/// essentially a convenience function for logging.
pub(crate) fn processing_failed(path: impl AsRef<Path>, err: anyhow::Error) -> Result<!> {
    error!(
        "failed to process completions at {:?}: {err:?}",
        path.as_ref()
    );
    Err(err)
}

/// A type which contains the state necessary to generate nu completions and
/// the import file.
#[derive(Debug, Default)]
pub(crate) struct CompletionsProcessor {
    definition_files: RwLock<HashSet<PathBuf>>,
}

impl CompletionsProcessor {
    /// A convenience function for calling
    /// [`CompletionsProcessor::process_file_or_dir_given_output_dir`] with
    /// [`Config::output_dir`].
    pub(crate) fn process_file_or_dir(&self, path: PathBuf) -> Result<()> {
        self.process_file_or_dir_given_output_dir(path, Config::output_dir())
    }

    /// Walk the given output directory, calling `[CompletionsProcessor::process_file_given_output_dir`]
    /// on each file.
    pub(crate) fn process_file_or_dir_given_output_dir(
        &self,
        path: PathBuf,
        output_dir: impl AsRef<Path>,
    ) -> Result<()> {
        info!(file = path.to_string_lossy(); "processing file or directory");
        let output_dir = output_dir.as_ref();
        walk_dir(&path, (), |path, _| {
            self.process_file_given_output_dir(&path, output_dir)
                .map(|_| ())
        })
    }

    /// Parse the completions listed in the given file, and write their
    /// equivalent nushell definition into `output_dir` (with the file name,
    /// but the extension `.nu`). Update the `CompletionsProcessor` state with
    /// the path to the new definition so that it can be sourced later.
    pub(crate) fn process_file_given_output_dir(
        &self,
        path: &Path,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        info!(file = path.to_string_lossy(); "processing file");
        if !path.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{path:#?} is not a file"),
            )
            .into());
        }
        let errmsg = format!("reading file {path:#?}");
        let file = BufReader::new(File::open(path)?);
        trace!(file = as_debug!(path); "opened file for processing");
        let completions =
            completions::Completions::parse(file.lines().map(|line| line.expect(&errmsg)))?;
        trace!("successfully parsed completions for {path:?}");
        {}
        let location = output_dir.join(
            path.with_extension("nu")
                .file_name()
                .expect("directory already checked for"),
        );
        debug!("writing completions parsed from {path:?} into {location:?}");
        Completions::at(&location)?.output(completions)?;
        self.definition_files
            .write()
            .expect("rwlock write access")
            .insert(location.clone());
        Ok(location)
    }

    /// After all the completions have been generated and their filenames,
    /// this function is used to create a `imports.nu` file which sources all
    /// of the definitions and can be sourced in turn.
    pub(crate) fn write_sourcing_file(&self, to: &Path) -> Result<()> {
        let mut file = File::create(to)?;
        for def in self
            .definition_files
            .read()
            .expect("rwlock read access")
            .iter()
        {
            file.write_all(format!("source {def:?}\n").as_bytes())?;
        }
        Ok(())
    }
}

/// A type with the state and methods necessary to write a
/// [`completions::Completions`] to an `IO` (such as a file).
#[derive(Debug)]
pub(crate) struct Completions<IO: Seek + Write> {
    io: IO,
    indent: usize,
}

impl<IO: Seek + Write> Completions<IO> {
    /// Wrap the given IO with `Completions`.
    fn new(io: IO) -> Self {
        Self { io, indent: 0 }
    }
}

impl Completions<File> {
    /// Open a file at the given location and wrap it with `Completions`.
    fn at(location: impl AsRef<Path>) -> Result<Self> {
        let file = File::create(location)?;
        Ok(Self::new(file))
    }
}

/// Some completions have multiple synonyms specified in one line, like
/// ```fish
/// complete -s r -s R -l recursive -d "copy files recursively"
/// ```
/// This tracks the relationship between these synonyms.
struct Synonym<'a> {
    synonym_of: String,
    name: String,
    description: Option<&'a str>,
}

impl<IO: Seek + Write> Completions<IO> {
    /// Write the given line at the current indentation level.
    fn write(&mut self, it: impl Display + log::kv::ToValue) -> Result<&mut Self> {
        trace!(content=it, indent=self.indent; "writing data");
        write!(self.io, "{}{it}", self.indent_str())?;
        Ok(self)
    }

    /// Write a newline
    fn eol(&mut self) -> Result<&mut Self> {
        writeln!(self.io)?;
        Ok(self)
    }

    /// output this set of completions as an extern command.
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
                                            description: option.description.as_deref(),
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
                                        description: option.description.as_deref(),
                                    });
                                }
                            }

                            for opt in &option.long[1..] {
                                synonyms.push(Synonym {
                                    name: format!("--{opt}"),
                                    synonym_of: format!("--{}", &option.long[0]),
                                    description: option.description.as_deref(),
                                });
                            }
                        }
                    }

                    [opt] => {
                        warn!(opt = opt; "skipping old-style long option");
                        continue;
                        // def.push('-');
                        // def.push_str(opt);
                    }
                    options => {
                        warn!(options = as_serde!(options); "skipping old-style long options");
                        continue;
                        // def.push('-');
                        // def.push_str(&options[0]);
                        // for opt in &options[1..] {
                        //     synonyms.push(Synonym {
                        //         name: format!("-{opt}"),
                        //         synonym_of: format!("-{}", &options[0]),
                        //         description: option.description.as_ref().map(|it| it.as_str()),
                        //     });
                        // }
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
                    self.write("  # ".to_owned() + description)?.eol()?;
                } else {
                    self.eol()?;
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

    /// The string necessary to indent to the current level of indentation.
    fn indent_str(&self) -> String {
        let mut cache = INDENT_CACHE.write().expect("poisoned mutex");
        if let Some(i) = cache.get(self.indent) {
            trace!(level=self.indent, str=i.as_str(); "got cached indent");
            i.clone()
        } else {
            let max_cached = cache.len();
            trace!(max_cached=max_cached, target_indent=self.indent; "indent not yet cached, filling");
            for i in max_cached..=self.indent {
                let text: String = " ".repeat(i * 4);
                trace!(level=i, str=text.as_str(); "caching indent level");
                cache.push(text);
            }
            // Size is ensured above
            unsafe { cache.get_unchecked(self.indent).clone() }
        }
    }
}

static INDENT_CACHE: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| RwLock::new(vec![]));

pub(crate) static INTERNAL_COMMANDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let cmd = Command::new("nu")
        .arg("-c")
        .arg("help commands | where command_type != external | get name | str join (char nl)")
        .stdout(Stdio::piped())
        .output()
        .expect("nu help commands to succeed");
    if cmd.status.success() {
        let list = cmd
            .stdout
            .split(|c| *c == 0xA)
            .map(String::from_utf8_lossy)
            .map(String::from)
            .collect();
        info!(internal_commands = as_debug!(list); "internal command list gathered");
        list
    } else {
        panic!("nu help command failed")
    }
});
