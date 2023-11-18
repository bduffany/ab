# ab

Simple tool written in Rust to perform A/B experiments on commands.

Similar to [hyperfine](https://github.com/sharkdp/hyperfine), but
with the following advantages:

- Randomly alternates between commands, to rule out execution ordering
  as a possible reason for performance differences.
- Shows live output of results, including a heatmap.

## Usage

- The first argument must be a comma-separated list of parameter values that you would like to test.
- The remaining arguments specify the command to be run.
- One of the arguments should be `{}` - the parameter values will be substituted here.

```shell
ab --flag-to-test=value1,--flag-to-test=value2 my-command {} other-flags...
```

## Demo

(Click to see animation)

[![asciicast](https://asciinema.org/a/wqKEmXflJ3hH0quTiES9WlumH.svg)](https://asciinema.org/a/wqKEmXflJ3hH0quTiES9WlumH)

## Installation

(TODO: publish release)

If you have `cargo` installed, just copy and paste this into your shell:

```shell
bash -ec '
  cd $(mktemp -d)
  git clone https://github.com/bduffany/ab "$PWD"
  cargo build -r
  sudo cp target/release/ab /usr/local/bin
'
```
