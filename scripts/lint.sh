#!/usr/bin/env sh

cargo clippy --features=test,test-bpf -- -D warnings -A clippy::await_holding_refcell_ref -A clippy::comparison_chain -A clippy::too_many_arguments
