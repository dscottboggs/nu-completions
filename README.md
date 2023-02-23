# [Nushell](nushell.sh) completions pack

This is a system for generating [`extern def`](https://www.nushell.sh/book/custom_completions.html#modules-and-custom-completions)s (tab-completion) in
[nu](nushell.sh).

## Background
The [fish shell](fishshell.com/) project has a [long, complicated script](https://github.com/fish-shell/fish-shell/blob/master/share/tools/create_manpage_completions.py)
for parsing man pages to gather completions definitions. Fortunately for us,
Fish completions are defined in simple command line arguments, which we parse
here and use to generate completions for Nu.

## Installation and basic usage
1. Download and optionally install this script. For a prebuilt binary, use
   [cargo-binstall](https://lib.rs/crates/cargo-binstall)...
   ~~~console
   cargo binstall nu-completion-script
   ~~~
   ...or if you don't have `cargo-binstall`, download the release from the
   [releases page](https://github.com/dscottboggs/nu-completions/releases) on Github.

   You may also build from source using `cargo install`, or cloning the
   repository and building with `cargo install`. See [Development Process](#development-process) for instructions.
2. Have `fish` installed.
3. From a `fish` shell, run
   ~~~fish
   fish_update_completions
   ~~~
4. Generate the `nu` definitions.
   ~~~console
   nu-completions ~/.local/share/fish/generated_completions/*.fish
   ~~~
5. Source the definitions
   ~~~console
   nu-completions --install
   ~~~

## Reporting bugs
If an error occurs, please run your command again with `-vvvv`, save the logs,
and include them when submitting a [Github
Issue](https://github.com/dscottboggs/nu-completions/issues/new). For example,
if an error occurs while running the above command to generate the definitions,
run this command instead:

~~~console
nu-completions -vvvv ~/.local/share/fish/generated_completions/*.fish | save log.json
~~~

And include the contents of `log.json` within a preformatted block, like this:

~~~text
```json
{"level":30,"time":1677171831187,"msg":"beginning translation phase"}
{"level":30,"time":1677171831187,"msg":"processing file or directory","file":"../../../.local/share/fish/generated_completions/7z.fish"}
{"level":30,"time":1677171831187,"msg":"processing file","file":"../../../.local/share/fish/generated_completions/7z.fish"}
{"level":10,"time":1677171831187,"msg":"opened file for processing","file":"\"../../../.local/share/fish/generated_completions/7z.fish\""}

...

{"level":10,"time":1677171833182,"msg":"opened file for processing","file":"\"../../../.local/share/fish/generated_completions/java-openjdk8.fish\""}
{"level":50,"time":1677171833182,"msg":"error parsing shell words","line":"complete -c java-openjdk8 -o 'disablesystemassertions"}
{"level":50,"time":1677171833182,"msg":"failed to process completions at \"../../../.local/share/fish/generated_completions/java-openjdk8.fish\": missing closing quote"}
```
~~~

## Development state
Since these definitions are auto-generated and incomplete, we need to modify
the auto-generated output to provide additional application-specific functions
to each definition. In order to provide a consistent workflow which smoothly
(we hope) handles upstream man-page updates, these application-specific
modifications take the form of a set of patches which are a part of this
repository.

## Development process

> **Note:**
> These directions assume you're using Linux, and have `git` and `cargo` installed)

1. [Fork this repository](https://github.com/dscottboggs/nu-completions/fork)
2. Clone your fork and enter the directory
   ~~~console
   git clone https://github.com/YOUR-GITHUB-USER/nu-completions.git
   cd nu-completions
   ~~~
3. Compile the binary
   ~~~console
   cargo build --release
   ~~~
   Alternatively, install the latest release's prebuilt binary (requires
   [cargo-binstall](https://lib.rs/crates/cargo-binstall))...
   ~~~console
   cargo binstall nu-completion-script
   ~~~
   ...or if you don't have `cargo-binstall`, download the release from the
   [releases page](https://github.com/dscottboggs/nu-completions/releases) on Github.
4. Have `fish` installed.
5. From a `fish` shell, run
   ~~~fish
   fish_update_completions
   ~~~
6. Generate the `nu` definitions.
   ~~~console
   nu-completions ~/.local/share/fish/generated_completions/*.fish
   ~~~
7. Make the changes you want to whichever of the generated files you want.
8. Generate a patch (or patches) from your changes
   ~~~console
   nu-completions --patch-dir=./patches patches generate ~/.local/share/fish/generated_completions/*.fish
   ~~~
9. Commit your changes, push them to your fork, and follow the link in your
   console to submit a pull request for the changes.