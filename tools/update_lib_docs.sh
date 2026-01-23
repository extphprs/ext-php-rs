#!/bin/bash
### This script updates the documentation in the macros library by copying
### the content from the guide files into the macros library source code.
###
### This is needed, because importing them using the `#[doc]` attribute
### does not work with rust analyser, which is used by the IDEs.
### https://github.com/rust-lang/rust-analyzer/issues/11137

set -e

# Check if the script is run from the root directory
if [ ! -f "tools/update_lib_docs.sh" ]; then
  echo "Please run this script from the root directory of the project."
  exit 1
fi

function update_docs() {
  file_name="$1.md"

  if [ ! -f "guide/src/macros/$file_name" ]; then
    echo "File guide/src/macros/$file_name does not exist."
    exit 1
  fi

  if ! grep -q "// BEGIN DOCS FROM $file_name" "crates/macros/src/lib.rs"; then
    echo "No start placeholder found for $file_name in crates/macros/src/lib.rs."
    exit 1
  fi

  if ! grep -q "// END DOCS FROM $file_name" "crates/macros/src/lib.rs"; then
    echo "No end placeholder found for $file_name in crates/macros/src/lib.rs."
    exit 1
  fi

  lead="^\/\/ BEGIN DOCS FROM $file_name$"
  tail="^\/\/ END DOCS FROM $file_name$"

  # Make content a doc comment
  sed -e "s/^/\/\/\/ /" "guide/src/macros/$file_name" |
    # Disable doc tests for the pasted content
    sed -e "s/rust,no_run/rust,no_run,ignore/" |
    # Replace the section in the macros library with the content from the guide
    sed -i -e "/$lead/,/$tail/{ /$lead/{p; r /dev/stdin" -e "}; /$tail/p; d }" "crates/macros/src/lib.rs"
}

update_docs "classes"
update_docs "constant"
update_docs "extern"
update_docs "function"
update_docs "impl"
update_docs "module"
update_docs "zval_convert"
update_docs "enum"
update_docs "interface"

# Format to remove trailing whitespace
rustup run nightly rustfmt --edition 2024 crates/macros/src/lib.rs
