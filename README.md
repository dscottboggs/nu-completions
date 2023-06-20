# [Nushell](nushell.sh) completions pack

## Moving on...

I think I'm going to abandon this project. Man pages are inconsistent across distributions, even for packages that are the same version. Whether you go all overboard like this and try to track a patch set, or just generate the completions once and track those completion files, you're stuck tracking changes to man pages in some way or another constantly for every distro.

Even if you do manage to put together enough people to do that thankless work and manage to get this overengineered solution to be robust and less klunky, there's another problem: nu takes about 1 second to load 1000 completion files on launch, meaning that launching my shell took at least 4.5-5 seconds every time. I put up with this for a little while, but then realized in a near-OOM situation I had no way to get to a shell that would open without compounding the problem by a lot (since `nu` was my default shell).

I think that I'll probably be trying out [these instructions for setting up Carapace](http://www.nushell.sh/book/custom_completions.html#external-completions) for completions next time I need to scratch this itch.

If you're interested in continuing this work feel free to fork it, and reach out to me if you need any help understanding the source, I've just gotten to the point where I can tell that the maintenance burden of this project is higher than I expected to take on when I wrote it.

----

This is a system for generating [`extern def`](https://www.nushell.sh/book/custom_completions.html#modules-and-custom-completions)s (tab-completion) in
[nu](nushell.sh).

> **Note:**
> This is not yet in a working state. Well, that is to say, it works fine if
  you're running Arch Linux, but testing on other distributions has immediately
  revealed that there are significant differences between distributions. We may
  need to start keeping track of patch-sets separately for each distribution,
  or perhaps switch to keeping track of the files themselves rather than a set
  of patches, or maybe both.

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
